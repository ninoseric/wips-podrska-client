// Prozor za odobravanje veze (proces `--cm`, connection manager): tehničar traži pristup →
// „Dopusti” / „Odbij”; za vrijeme veze „Prekini vezu”. Logika (IPC sa serverom, odobravanje,
// prekid) je RustDeskova (ui_cm_interface); ovdje su prozor, miš i timer. Proces završava kad nema veza.
use std::cell::RefCell;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, EndPaint, KillTimer, LoadCursorW, PostQuitMessage, SetCursor,
    SetTimer, IDC_HAND, PAINTSTRUCT, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONUP,
    WM_MOUSELEAVE, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WM_SIZE, WM_TIMER,
};

use super::d2d::Gfx;
use super::izgled::{self, CmButton, Fonts};
use super::win32::{self, WindowSpec};
use crate::ui_cm_interface::{self, Client, ConnectionManager, InvokeUiCM};

const CLASS: &str = "WiPSLaganiCm";
const TITLE: &str = "WiPS udaljena podrška";
const TIMER_REFRESH: usize = 1;

#[derive(Clone, PartialEq)]
struct Peer {
    id: i32,
    name: String,
    peer_id: String,
    authorized: bool,
}

/// Veze koje je javila jezgra; puni ih IPC nit, čita UI nit (timer).
static PEERS: Mutex<Vec<Peer>> = Mutex::new(Vec::new());
static HWND_CM: AtomicIsize = AtomicIsize::new(0);

#[derive(Clone)]
struct Handler;

impl InvokeUiCM for Handler {
    fn add_connection(&self, client: &Client) {
        log::info!(
            "lagani cm: veza {} od {} ({}), odobrena: {}",
            client.id,
            client.name,
            client.peer_id,
            client.authorized
        );
        let peer = Peer {
            id: client.id,
            name: client.name.clone(),
            peer_id: client.peer_id.clone(),
            authorized: client.authorized,
        };
        if let Ok(mut v) = PEERS.lock() {
            v.retain(|p| p.id != client.id);
            v.push(peer);
        }
        let hwnd = HWND_CM.load(Ordering::SeqCst);
        if hwnd != 0 {
            unsafe {
                win32::bring_to_front(hwnd as HWND);
            }
        }
    }

    fn remove_connection(&self, id: i32, close: bool) {
        log::info!("lagani cm: veza {} završena (close: {})", id, close);
        if !close {
            // Tehničar je odspojen (možda se vrati); prototip ga ne drži kao „odspojenog”.
            ui_cm_interface::remove(id);
        }
        if let Ok(mut v) = PEERS.lock() {
            v.retain(|p| p.id != id);
        }
        if ui_cm_interface::get_clients_length() == 0 {
            log::info!("lagani cm: nema više veza, izlaz");
            crate::platform::quit_gui();
        }
    }

    fn new_message(&self, _id: i32, _text: String) {}

    fn change_theme(&self, _dark: String) {}

    fn change_language(&self) {}

    fn show_elevation(&self, _show: bool) {}

    fn update_voice_call_state(&self, _client: &Client) {}

    fn file_transfer_log(&self, _action: &str, _log: &str) {}
}

struct Ui {
    gfx: Gfx,
    fonts: Fonts,
    shown: Option<Peer>,
    hover: Option<CmButton>,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = RefCell::new(None);
}

fn with_ui<R>(f: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    UI.with(|c| match c.try_borrow_mut() {
        Ok(mut g) => g.as_mut().map(f),
        Err(_) => None,
    })
}

fn state(peer: &Option<Peer>, hover: Option<CmButton>) -> izgled::Cm<'_> {
    izgled::Cm {
        peer: peer.as_ref().map(|p| izgled::CmPeer {
            name: &p.name,
            peer_id: &p.peer_id,
            authorized: p.authorized,
        }),
        hover,
    }
}

pub fn run() {
    let cm = ConnectionManager { ui_handler: Handler };
    std::thread::spawn(move || ui_cm_interface::start_ipc(cm));
    let hwnd = unsafe {
        win32::create_window(&WindowSpec {
            class: CLASS,
            title: TITLE,
            width: izgled::CM_W,
            height: izgled::CM_H,
            scale: win32::scale(),
            background: izgled::BG.0,
            topmost: true,
            wndproc: Some(wndproc),
        })
    };
    if hwnd.is_null() {
        log::error!("lagani cm: prozor nije stvoren");
        return;
    }
    HWND_CM.store(hwnd as isize, Ordering::SeqCst);
    log::info!(
        "lagani cm: prozor prikazan {} ms nakon pokretanja procesa",
        win32::ms_since_process_start()
    );
    unsafe {
        win32::message_loop();
    }
}

fn current_peer() -> Option<Peer> {
    let v = PEERS.lock().ok()?;
    v.iter().find(|p| !p.authorized).or_else(|| v.last()).cloned()
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            on_create(hwnd);
            0
        }
        WM_TIMER => {
            on_timer(hwnd);
            0
        }
        WM_PAINT => {
            on_paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            with_ui(|ui| ui.gfx.resize());
            0
        }
        WM_MOUSEMOVE => {
            let changed = with_ui(|ui| {
                let (x, y) = win32::mouse_point(lparam, ui.gfx.scale());
                let h = izgled::cm_hit(&state(&ui.shown, None), x, y);
                let changed = h != ui.hover;
                ui.hover = h;
                changed
            })
            .unwrap_or(false);
            win32::track_mouse_leave(hwnd);
            if changed {
                win32::redraw(hwnd);
            }
            0
        }
        WM_MOUSELEAVE => {
            if with_ui(|ui| ui.hover.take().is_some()).unwrap_or(false) {
                win32::redraw(hwnd);
            }
            0
        }
        WM_LBUTTONUP => {
            let hit = with_ui(|ui| {
                let (x, y) = win32::mouse_point(lparam, ui.gfx.scale());
                izgled::cm_hit(&state(&ui.shown, None), x, y)
            })
            .flatten();
            match hit {
                Some(CmButton::Allow) => on_allow(),
                Some(CmButton::Deny) | Some(CmButton::Disconnect) => on_close_peer(),
                None => {}
            }
            0
        }
        WM_SETCURSOR => {
            if with_ui(|ui| ui.hover.is_some()).unwrap_or(false) {
                SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                1
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_CLOSE => {
            // Zatvaranje prozora prekida sve veze; proces završava.
            if let Ok(v) = PEERS.lock() {
                for p in v.iter() {
                    ui_cm_interface::close(p.id);
                }
            }
            log::info!("lagani cm: korisnik zatvorio prozor, izlaz");
            crate::platform::quit_gui();
            0
        }
        WM_DESTROY => {
            KillTimer(hwnd, TIMER_REFRESH);
            UI.with(|c| {
                if let Ok(mut g) = c.try_borrow_mut() {
                    *g = None;
                }
            });
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_create(hwnd: HWND) {
    let gfx = match Gfx::new(windows::Win32::Foundation::HWND(hwnd as *mut _)) {
        Ok(g) => g,
        Err(e) => {
            log::error!("lagani cm: Direct2D nije dostupan: {}", e);
            return;
        }
    };
    let fonts = match Fonts::new(&gfx) {
        Ok(f) => f,
        Err(e) => {
            log::error!("lagani cm: fontovi nisu stvoreni: {}", e);
            return;
        }
    };
    UI.with(|c| {
        if let Ok(mut g) = c.try_borrow_mut() {
            *g = Some(Ui {
                gfx,
                fonts,
                shown: None,
                hover: None,
            });
        }
    });
    SetTimer(hwnd, TIMER_REFRESH, 300, None);
    on_timer(hwnd);
}

unsafe fn on_timer(hwnd: HWND) {
    let peer = current_peer();
    let changed = with_ui(|ui| {
        if ui.shown != peer {
            ui.shown = peer;
            ui.hover = None;
            true
        } else {
            false
        }
    })
    .unwrap_or(false);
    if changed {
        win32::redraw(hwnd);
    }
}

unsafe fn on_allow() {
    let Some(p) = current_peer() else {
        return;
    };
    if p.authorized {
        return;
    }
    log::info!("lagani cm: korisnik dopustio {} ({})", p.name, p.peer_id);
    ui_cm_interface::authorize(p.id);
    if let Ok(mut v) = PEERS.lock() {
        for q in v.iter_mut() {
            if q.id == p.id {
                q.authorized = true;
            }
        }
    }
}

unsafe fn on_close_peer() {
    let Some(p) = current_peer() else {
        return;
    };
    log::info!("lagani cm: korisnik prekinuo/odbio {} ({})", p.name, p.peer_id);
    ui_cm_interface::close(p.id);
}

unsafe fn on_paint(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    BeginPaint(hwnd, &mut ps);
    with_ui(|ui| {
        let Ui {
            gfx,
            fonts,
            shown,
            hover,
        } = ui;
        let s = state(shown, *hover);
        if let Err(e) = gfx.paint(izgled::BG, |c| izgled::crtaj_cm(c, fonts, &s)) {
            log::error!("lagani cm: crtanje nije uspjelo: {}", e);
        }
    });
    EndPaint(hwnd, &ps);
}
