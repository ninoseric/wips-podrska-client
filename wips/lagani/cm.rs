// Prozor za odobravanje veze (proces `--cm`, connection manager): tehničar traži pristup →
// „Dopusti” / „Odbij”; za vrijeme veze „Prekini vezu”. Logika (IPC sa serverom, odobravanje,
// prekid) je RustDeskova (ui_cm_interface); ovdje je samo prozor. Proces završava kad nema veza.
use std::cell::RefCell;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;

use hbb_common::log;
use winapi::shared::minwindef::{LOWORD, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HBRUSH, HDC, HWND};
use winapi::um::wingdi::{CreateSolidBrush, DeleteObject, FW_NORMAL, FW_SEMIBOLD};
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, EndPaint, KillTimer, PostQuitMessage, SetTimer, ShowWindow,
    DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, PAINTSTRUCT, SW_HIDE, SW_SHOW,
    WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN, WM_DESTROY, WM_PAINT, WM_TIMER,
};

use super::win32::{self, px, rgb, Font, WindowSpec};
use crate::ui_cm_interface::{self, Client, ConnectionManager, InvokeUiCM};

const CLASS: &str = "WiPSLaganiCm";
const TITLE: &str = "WiPS udaljena podrška";
const WIDTH: i32 = 380;
const HEIGHT: i32 = 236;
const TIP_REQUEST: &str = "Odobrite samo ako ste upravo razgovarali s podrškom WiPS. \
                           Vezu možete prekinuti u bilo kojem trenutku.";
const TIP_ACTIVE: &str = "Tehničar vidi vaš zaslon i upravlja mišem i tipkovnicom. \
                          Vezu možete prekinuti u bilo kojem trenutku.";

const BG: u32 = rgb(0xFA, 0xFA, 0xFB);
const TEXT: u32 = rgb(0x11, 0x18, 0x27);
const MUTED: u32 = rgb(0x4B, 0x55, 0x63);

const BTN_ALLOW: u16 = 2001;
const BTN_DENY: u16 = 2002;
const BTN_DISCONNECT: u16 = 2003;
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

struct Fonts {
    title: Font,
    body: Font,
    small: Font,
}

struct Ui {
    scale: f32,
    fonts: Fonts,
    bg_brush: HBRUSH,
    btn_allow: HWND,
    btn_deny: HWND,
    btn_disconnect: HWND,
    shown: Option<Peer>,
}

impl Drop for Ui {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.bg_brush as _);
        }
    }
}

thread_local! {
    static UI: RefCell<Option<Ui>> = RefCell::new(None);
}

pub fn run() {
    let cm = ConnectionManager { ui_handler: Handler };
    std::thread::spawn(move || ui_cm_interface::start_ipc(cm));
    let scale = win32::scale();
    let hwnd = unsafe {
        win32::create_window(&WindowSpec {
            class: CLASS,
            title: TITLE,
            width: WIDTH,
            height: HEIGHT,
            scale,
            background: BG,
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
        WM_COMMAND => {
            match LOWORD(wparam as u32) {
                BTN_ALLOW => on_allow(),
                BTN_DENY | BTN_DISCONNECT => on_close_peer(),
                _ => {}
            }
            0
        }
        WM_CTLCOLORBTN => {
            let brush = UI.with(|c| {
                c.try_borrow()
                    .ok()
                    .and_then(|g| g.as_ref().map(|u| u.bg_brush as LRESULT))
            });
            match brush {
                Some(b) => b,
                None => DefWindowProcW(hwnd, msg, wparam, lparam),
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
    let scale = win32::scale();
    let fonts = Fonts {
        title: Font::new("Segoe UI", 14.0, FW_SEMIBOLD, scale),
        body: Font::new("Segoe UI", 10.0, FW_NORMAL, scale),
        small: Font::new("Segoe UI", 9.0, FW_NORMAL, scale),
    };
    let r = |l: i32, t: i32, rt: i32, b: i32| {
        win32::rect(px(l, scale), px(t, scale), px(rt, scale), px(b, scale))
    };
    let btn_allow =
        win32::create_button(hwnd, BTN_ALLOW, "Dopusti", &r(24, 180, 144, 212), true, &fonts.body);
    let btn_deny =
        win32::create_button(hwnd, BTN_DENY, "Odbij", &r(156, 180, 276, 212), false, &fonts.body);
    let btn_disconnect = win32::create_button(
        hwnd,
        BTN_DISCONNECT,
        "Prekini vezu",
        &r(24, 180, 164, 212),
        false,
        &fonts.body,
    );
    ShowWindow(btn_allow, SW_HIDE);
    ShowWindow(btn_deny, SW_HIDE);
    ShowWindow(btn_disconnect, SW_HIDE);
    let ui = Ui {
        scale,
        fonts,
        bg_brush: CreateSolidBrush(BG),
        btn_allow,
        btn_deny,
        btn_disconnect,
        shown: None,
    };
    UI.with(|c| {
        if let Ok(mut g) = c.try_borrow_mut() {
            *g = Some(ui);
        }
    });
    SetTimer(hwnd, TIMER_REFRESH, 300, None);
    on_timer(hwnd);
}

unsafe fn on_timer(hwnd: HWND) {
    let peer = current_peer();
    let buttons = UI.with(|c| match c.try_borrow_mut() {
        Ok(mut g) => match g.as_mut() {
            Some(ui) if ui.shown != peer => {
                ui.shown = peer.clone();
                Some((ui.btn_allow, ui.btn_deny, ui.btn_disconnect))
            }
            _ => None,
        },
        Err(_) => None,
    });
    let Some((btn_allow, btn_deny, btn_disconnect)) = buttons else {
        return;
    };
    let (request, active) = match &peer {
        Some(p) => (!p.authorized, p.authorized),
        None => (false, false),
    };
    ShowWindow(btn_allow, if request { SW_SHOW } else { SW_HIDE });
    ShowWindow(btn_deny, if request { SW_SHOW } else { SW_HIDE });
    ShowWindow(btn_disconnect, if active { SW_SHOW } else { SW_HIDE });
    win32::redraw(hwnd);
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
    let hdc = BeginPaint(hwnd, &mut ps);
    UI.with(|c| {
        if let Ok(g) = c.try_borrow() {
            if let Some(ui) = g.as_ref() {
                paint(hdc, hwnd, ui);
            }
        }
    });
    EndPaint(hwnd, &ps);
}

unsafe fn paint(hdc: HDC, hwnd: HWND, ui: &Ui) {
    let s = ui.scale;
    let p = |v: i32| px(v, s);
    let client = win32::client_rect(hwnd);
    win32::fill_rect(hdc, &client, BG);

    let (title, body, tip) = match &ui.shown {
        None => (
            "Čekanje na zahtjev za spajanje…".to_owned(),
            String::new(),
            "",
        ),
        Some(peer) => {
            let who = if peer.name.trim().is_empty() {
                format!("Tehničar WiPS ({})", peer.peer_id)
            } else {
                format!("{} ({})", peer.name.trim(), peer.peer_id)
            };
            if peer.authorized {
                (
                    "Veza je aktivna".to_owned(),
                    format!("{} upravlja vašim računalom.", who),
                    TIP_ACTIVE,
                )
            } else {
                (
                    "Zahtjev za spajanje".to_owned(),
                    format!("{} traži pristup vašem računalu.", who),
                    TIP_REQUEST,
                )
            }
        }
    };

    let mut rc = win32::rect(p(24), p(16), p(356), p(42));
    win32::draw_text(
        hdc,
        &title,
        &mut rc,
        &ui.fonts.title,
        TEXT,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
    );
    let mut rc = win32::rect(p(24), p(52), p(356), p(96));
    win32::draw_text(
        hdc,
        &body,
        &mut rc,
        &ui.fonts.body,
        TEXT,
        DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
    );
    let mut rc = win32::rect(p(24), p(104), p(356), p(168));
    win32::draw_text(
        hdc,
        tip,
        &mut rc,
        &ui.fonts.small,
        MUTED,
        DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
    );
}
