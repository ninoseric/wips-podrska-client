// Početni prozor korisničke verzije: ID, stanje veze sa serverom, uputa za poziv podrške, gumb za
// kopiranje ID-a. Izgled slijedi smjer C „Svijetli Fluent” iz Flutter verzije (patch app/0004),
// nacrtan GDI-jem. ID i stanje dolaze iz jezgre preko IPC-a (ui_interface::get_connect_status).
use std::cell::RefCell;
use std::ptr::null;
use std::sync::Mutex;

use hbb_common::log;
use winapi::shared::minwindef::{LOWORD, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HBRUSH, HDC, HWND};
use winapi::um::wingdi::{CreateSolidBrush, DeleteObject, FW_BOLD, FW_NORMAL, FW_SEMIBOLD};
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, EndPaint, FindWindowW, KillTimer, PostQuitMessage, SetTimer,
    DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, PAINTSTRUCT,
    WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN, WM_DESTROY, WM_PAINT, WM_TIMER,
};

use super::win32::{self, px, rgb, Font, WindowSpec};

const CLASS: &str = "WiPSLaganiGlavni";
const TITLE: &str = "WiPS udaljena podrška";
const WIDTH: i32 = 340;
const HEIGHT: i32 = 336;
const TIP: &str = "Nazovite podršku na 01/61 76 166 i pročitajte ID tehničaru. \
                   Spajanje uvijek morate odobriti vi.";
const FOOTER: &str = "WiPS Systems  ·  01/61 76 166  ·  wips.hr";

// Paleta smjera C (svijetla tema), kao u wips_incoming_home.dart.
const BG: u32 = rgb(0xFA, 0xFA, 0xFB);
const CARD: u32 = rgb(0xFF, 0xFF, 0xFF);
const BORDER: u32 = rgb(0xE5, 0xE7, 0xEB);
const TEXT: u32 = rgb(0x11, 0x18, 0x27);
const MUTED: u32 = rgb(0x4B, 0x55, 0x63);
const OK: u32 = rgb(0x15, 0x80, 0x3D);
const WAIT: u32 = rgb(0xB4, 0x53, 0x09);
const ERR: u32 = rgb(0xB9, 0x1C, 0x1C);

const BTN_COPY: u16 = 1001;
const TIMER_STATUS: usize = 1;
const TIMER_COPIED: usize = 2;

/// ID dohvaćen od jezgre (ipc::get_id) u pozadinskoj niti, dok ga nit stanja još nije javila.
static IPC_ID: Mutex<String> = Mutex::new(String::new());

struct Fonts {
    title: Font,
    label: Font,
    id: Font,
    body: Font,
    small: Font,
}

struct Ui {
    scale: f32,
    fonts: Fonts,
    bg_brush: HBRUSH,
    btn_copy: HWND,
    id: String,
    status: i32,
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
    let scale = win32::scale();
    let hwnd = unsafe {
        win32::create_window(&WindowSpec {
            class: CLASS,
            title: TITLE,
            width: WIDTH,
            height: HEIGHT,
            scale,
            background: BG,
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
        WM_COMMAND => {
            if LOWORD(wparam as u32) == BTN_COPY {
                on_copy(hwnd);
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
    let scale = win32::scale();
    let fonts = Fonts {
        title: Font::new("Segoe UI", 14.0, FW_SEMIBOLD, scale),
        label: Font::new("Segoe UI", 9.0, FW_SEMIBOLD, scale),
        id: Font::new("Consolas", 24.0, FW_BOLD, scale),
        body: Font::new("Segoe UI", 9.5, FW_NORMAL, scale),
        small: Font::new("Segoe UI", 9.0, FW_NORMAL, scale),
    };
    let rc = win32::rect(px(38, scale), px(232, scale), px(158, scale), px(264, scale));
    let btn_copy = win32::create_button(hwnd, BTN_COPY, "Kopiraj ID", &rc, true, &fonts.body);
    let ui = Ui {
        scale,
        fonts,
        bg_brush: CreateSolidBrush(BG),
        btn_copy,
        id: String::new(),
        status: 0,
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
        let btn = UI.with(|c| c.try_borrow().ok().and_then(|g| g.as_ref().map(|u| u.btn_copy)));
        if let Some(btn) = btn {
            win32::set_text(btn, "Kopiraj ID");
        }
        return;
    }
    let st = crate::ui_interface::get_connect_status();
    let mut new_id = st.id.clone();
    if new_id.is_empty() {
        if let Ok(v) = IPC_ID.lock() {
            new_id = v.clone();
        }
    }
    let changed = UI.with(|c| match c.try_borrow_mut() {
        Ok(mut g) => match g.as_mut() {
            Some(ui) => {
                let changed = ui.id != new_id || ui.status != st.status_num;
                ui.id = new_id;
                ui.status = st.status_num;
                changed
            }
            None => false,
        },
        Err(_) => false,
    });
    if changed {
        win32::redraw(hwnd);
    }
}

unsafe fn on_copy(hwnd: HWND) {
    let Some((id, btn)) = UI.with(|c| {
        c.try_borrow()
            .ok()
            .and_then(|g| g.as_ref().map(|u| (u.id.clone(), u.btn_copy)))
    }) else {
        return;
    };
    if id.is_empty() {
        return;
    }
    match arboard::Clipboard::new().and_then(|mut c| c.set_text(id)) {
        Ok(()) => {
            win32::set_text(btn, "Kopirano");
            SetTimer(hwnd, TIMER_COPIED, 1500, None);
        }
        Err(e) => log::error!("lagani: kopiranje ID-a nije uspjelo: {}", e),
    }
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
    let line = DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX;

    let client = win32::client_rect(hwnd);
    win32::fill_rect(hdc, &client, BG);

    let mut rc = win32::rect(p(20), p(14), p(320), p(40));
    win32::draw_text(hdc, TITLE, &mut rc, &ui.fonts.title, TEXT, line);

    let card = win32::rect(p(20), p(50), p(320), p(282));
    win32::card(hdc, &card, CARD, BORDER, p(10));

    let mut rc = win32::rect(p(38), p(66), p(302), p(82));
    win32::draw_text(hdc, "VAŠ ID", &mut rc, &ui.fonts.label, MUTED, line);

    let id_text = if ui.id.is_empty() {
        "— — —".to_owned()
    } else {
        format_id(&ui.id)
    };
    let mut rc = win32::rect(p(38), p(84), p(302), p(122));
    win32::draw_text(hdc, &id_text, &mut rc, &ui.fonts.id, TEXT, line);

    let (color, text) = status_text(ui.status);
    win32::dot(hdc, p(43), p(139), p(4), color);
    let mut rc = win32::rect(p(52), p(130), p(302), p(148));
    win32::draw_text(hdc, text, &mut rc, &ui.fonts.body, MUTED, line);

    win32::fill_rect(hdc, &win32::rect(p(38), p(158), p(302), p(159)), BORDER);

    let mut rc = win32::rect(p(38), p(168), p(302), p(222));
    win32::draw_text(
        hdc,
        TIP,
        &mut rc,
        &ui.fonts.body,
        MUTED,
        DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
    );

    let mut rc = win32::rect(p(20), p(294), p(320), p(316));
    win32::draw_text(
        hdc,
        FOOTER,
        &mut rc,
        &ui.fonts.small,
        MUTED,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
    );
}

fn status_text(status: i32) -> (u32, &'static str) {
    if status > 0 {
        (OK, "Povezano sa serverom WiPS")
    } else if status == 0 {
        (WAIT, "Povezivanje sa serverom…")
    } else {
        (ERR, "Podrška trenutno nije dostupna")
    }
}

/// „123456789” → „123 456 789”; ostale ID-ove ostavlja kakvi jesu.
fn format_id(id: &str) -> String {
    if id.len() == 9 && id.chars().all(|c| c.is_ascii_digit()) {
        format!("{} {} {}", &id[0..3], &id[3..6], &id[6..9])
    } else {
        id.to_owned()
    }
}
