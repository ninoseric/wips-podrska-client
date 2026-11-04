// Početni prozor korisničke verzije: logo, stanje veze, ID, „Kopiraj ID”, uputa za poziv podrške.
// Izgled je u izgled.rs (smjer C „Svijetli Fluent”), crtanje u d2d.rs; ovdje su prozor, miš i timeri.
// ID i stanje dolaze iz jezgre preko IPC-a (ui_interface::get_connect_status).
use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::sync::Mutex;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, EndPaint, FindWindowW, KillTimer, LoadCursorW, PostQuitMessage,
    SetCursor, SetTimer, IDC_HAND, PAINTSTRUCT, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONUP,
    WM_MOUSELEAVE, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WM_SIZE, WM_TIMER,
};

use super::d2d::{inside, Gfx};
use super::izgled::{self, Fonts};
use super::win32::{self, WindowSpec};

const CLASS: &str = "WiPSLaganiGlavni";
const TITLE: &str = "WiPS udaljena podrška";

const TIMER_STATUS: usize = 1;
const TIMER_COPIED: usize = 2;

/// ID dohvaćen od jezgre (ipc::get_id) u pozadinskoj niti, dok ga nit stanja još nije javila.
static IPC_ID: Mutex<String> = Mutex::new(String::new());

struct Ui {
    gfx: Gfx,
    fonts: Fonts,
    id: String,
    status: i32,
    copied: bool,
    hover: bool,
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

pub fn run() {
    unsafe {
        // Jedna instanca: ako prozor već postoji, samo ga dovedi u prvi plan.
        let class = win32::wide(CLASS);
        let existing = FindWindowW(class.as_ptr(), null());
        if !existing.is_null() {
            win32::bring_to_front(existing);
            return;
        }
    }
    // Pokreće nit koja preko IPC-a prati stanje veze sa serverom i ID (ui_interface::SENDER).
    crate::ui_interface::start_option_status_sync();
    std::thread::spawn(|| {
        let id = crate::ipc::get_id();
        if let Ok(mut v) = IPC_ID.lock() {
            *v = id;
        }
    });
    let hwnd = unsafe {
        win32::create_window(&WindowSpec {
            class: CLASS,
            title: TITLE,
            width: izgled::GLAVNI_W,
            height: izgled::GLAVNI_H,
            scale: win32::scale(),
            background: izgled::BG.0,
            topmost: false,
            wndproc: Some(wndproc),
        })
    };
    if hwnd.is_null() {
        log::error!("lagani: početni prozor nije stvoren");
        return;
    }
    log::info!(
        "lagani: početni prozor prikazan {} ms nakon pokretanja procesa",
        win32::ms_since_process_start()
    );
    unsafe {
        win32::message_loop();
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            on_create(hwnd);
            0
        }
        WM_TIMER => {
            on_timer(hwnd, wparam);
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
                let h = inside(&izgled::glavni_copy_rect(), x, y);
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
            if with_ui(|ui| std::mem::replace(&mut ui.hover, false)).unwrap_or(false) {
                win32::redraw(hwnd);
            }
            0
        }
        WM_LBUTTONUP => {
            let hit = with_ui(|ui| {
                let (x, y) = win32::mouse_point(lparam, ui.gfx.scale());
                inside(&izgled::glavni_copy_rect(), x, y)
            })
            .unwrap_or(false);
            if hit {
                on_copy(hwnd);
            }
            0
        }
        WM_SETCURSOR => {
            if with_ui(|ui| ui.hover).unwrap_or(false) {
                SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                1
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_DESTROY => {
            KillTimer(hwnd, TIMER_STATUS);
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
            log::error!("lagani: Direct2D nije dostupan: {}", e);
            return;
        }
    };
    let fonts = match Fonts::new(&gfx) {
        Ok(f) => f,
        Err(e) => {
            log::error!("lagani: fontovi nisu stvoreni: {}", e);
            return;
        }
    };
    let ui = Ui {
        gfx,
        fonts,
        id: String::new(),
        status: 0,
        copied: false,
        hover: false,
    };
    UI.with(|c| {
        if let Ok(mut g) = c.try_borrow_mut() {
            *g = Some(ui);
        }
    });
    SetTimer(hwnd, TIMER_STATUS, 400, None);
    on_timer(hwnd, TIMER_STATUS);
}

unsafe fn on_timer(hwnd: HWND, id: WPARAM) {
    if id == TIMER_COPIED {
        KillTimer(hwnd, TIMER_COPIED);
        with_ui(|ui| ui.copied = false);
        win32::redraw(hwnd);
        return;
    }
    let st = crate::ui_interface::get_connect_status();
    let mut new_id = st.id.clone();
    if new_id.is_empty() {
        if let Ok(v) = IPC_ID.lock() {
            new_id = v.clone();
        }
    }
    let changed = with_ui(|ui| {
        let changed = ui.id != new_id || ui.status != st.status_num;
        ui.id = new_id;
        ui.status = st.status_num;
        changed
    })
    .unwrap_or(false);
    if changed {
        win32::redraw(hwnd);
    }
}

unsafe fn on_copy(hwnd: HWND) {
    let Some(id) = with_ui(|ui| ui.id.clone()) else {
        return;
    };
    if id.is_empty() {
        return;
    }
    match arboard::Clipboard::new().and_then(|mut c| c.set_text(id)) {
        Ok(()) => {
            with_ui(|ui| ui.copied = true);
            win32::redraw(hwnd);
            SetTimer(hwnd, TIMER_COPIED, 1500, None);
        }
        Err(e) => log::error!("lagani: kopiranje ID-a nije uspjelo: {}", e),
    }
}

unsafe fn on_paint(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    BeginPaint(hwnd, &mut ps);
    with_ui(|ui| {
        let Ui {
            gfx,
            fonts,
            id,
            status,
            copied,
            hover,
        } = ui;
        let s = izgled::Glavni {
            id,
            status: *status,
            version: crate::VERSION,
            copied: *copied,
            hover_copy: *hover,
        };
        if let Err(e) = gfx.paint(izgled::BG, |c| izgled::crtaj_glavni(c, fonts, &s)) {
            log::error!("lagani: crtanje nije uspjelo: {}", e);
        }
    });
    EndPaint(hwnd, &ps);
}
