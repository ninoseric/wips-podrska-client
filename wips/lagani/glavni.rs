// Početni prozor korisničke verzije, kao wips_incoming_home.dart: logo, stanje veze, ID, „Kopiraj ID”,
// uputa za poziv podrške, redak za instalaciju ili nadogradnju, podnožje s poveznicom i postavkama.
// ID i stanje dolaze iz jezgre preko IPC-a (ui_interface::get_connect_status).
use std::cell::RefCell;
use std::ptr::null;
use std::sync::Mutex;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, EndPaint, FindWindowW, KillTimer, PostQuitMessage, SetTimer, PAINTSTRUCT,
    WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE, WM_PAINT,
    WM_SETCURSOR, WM_SETTINGCHANGE, WM_SIZE, WM_TIMER,
};

use super::d2d::rect;
use super::izgled::{self, format_id, ikona};
use super::ui::{Prozor, WM_OSVJEZI};
use super::win32::{self, Pos, WindowSpec};
use super::{postavke, tr};

const CLASS: &str = "WiPSLaganiGlavni";
const W: f32 = 340.0;
const H: i32 = 368;

const TIMER_STATUS: usize = 1;
const TIMER_COPIED: usize = 2;

/// ID dohvaćen od jezgre (ipc::get_id) u pozadinskoj niti, dok ga nit stanja još nije javila.
static IPC_ID: Mutex<String> = Mutex::new(String::new());

#[derive(Clone, Copy, PartialEq)]
enum A {
    Kopiraj,
    Instaliraj,
    Nadogradi,
    Web,
    Postavke,
}

/// Redak ispod upute, kao u Flutter verziji: instalacija dok program nije instaliran, inače nadogradnja
/// ako je instalirana starija verzija.
#[derive(Clone, Copy, PartialEq)]
enum Red {
    Nista,
    Instalacija,
    Nadogradnja,
}

fn red() -> Red {
    if hbb_common::config::is_disable_installation() {
        Red::Nista
    } else if !crate::ui_interface::is_installed() {
        Red::Instalacija
    } else if crate::ui_interface::is_installed_lower_version() {
        Red::Nadogradnja
    } else {
        Red::Nista
    }
}

struct Stanje {
    w: Prozor<A>,
    id: String,
    status: i32,
    copied: bool,
    red: Red,
}

thread_local! {
    static UI: RefCell<Option<Stanje>> = RefCell::new(None);
}

fn with_ui<R>(f: impl FnOnce(&mut Stanje) -> R) -> Option<R> {
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
    let height = H;
    let hwnd = unsafe {
        win32::create_window(&WindowSpec {
            class: CLASS,
            title: izgled::NAZIV,
            pos: Pos::Center,
            owner: std::ptr::null_mut(),
            width: W as i32,
            height,
            scale: win32::scale(),
            background: super::tema::pal().bg.0,
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
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            with_ui(crtaj);
            EndPaint(hwnd, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            with_ui(|s| s.w.gfx.resize());
            0
        }
        WM_MOUSEMOVE | WM_MOUSELEAVE | WM_LBUTTONDOWN | WM_LBUTTONUP => {
            if let Some(Some(a)) = with_ui(|s| s.w.mis(hwnd, msg, lparam)) {
                on_action(hwnd, a);
            }
            0
        }
        WM_SETCURSOR => {
            if with_ui(|s| s.w.kursor()).unwrap_or(false) {
                1
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_SETTINGCHANGE | WM_OSVJEZI => {
            with_ui(|s| s.w.osvjezi_temu(hwnd));
            0
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
    let w = match Prozor::new(hwnd) {
        Ok(w) => w,
        Err(e) => {
            log::error!("lagani: Direct2D nije dostupan: {}", e);
            return;
        }
    };
    let s = Stanje {
        w,
        id: String::new(),
        status: 0,
        copied: false,
        red: red(),
    };
    UI.with(|c| {
        if let Ok(mut g) = c.try_borrow_mut() {
            *g = Some(s);
        }
    });
    SetTimer(hwnd, TIMER_STATUS, 400, None);
    on_timer(hwnd, TIMER_STATUS);
}

unsafe fn on_timer(hwnd: HWND, id: WPARAM) {
    if id == TIMER_COPIED {
        KillTimer(hwnd, TIMER_COPIED);
        with_ui(|s| s.copied = false);
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
    let changed = with_ui(|s| {
        let changed = s.id != new_id || s.status != st.status_num;
        s.id = new_id;
        s.status = st.status_num;
        changed
    })
    .unwrap_or(false);
    if changed {
        win32::redraw(hwnd);
    }
}

unsafe fn on_action(hwnd: HWND, a: A) {
    match a {
        A::Kopiraj => on_copy(hwnd),
        A::Instaliraj => {
            log::info!("lagani: korisnik pokrenuo instalaciju");
            crate::ui_interface::goto_install();
        }
        A::Nadogradi => {
            log::info!("lagani: korisnik pokrenuo nadogradnju");
            crate::ui_interface::update_me(String::new());
        }
        A::Web => win32::open_url(izgled::WEB),
        A::Postavke => postavke::otvori(hwnd),
    }
}

unsafe fn on_copy(hwnd: HWND) {
    let Some(id) = with_ui(|s| s.id.replace(' ', "")) else {
        return;
    };
    if id.is_empty() {
        return;
    }
    // Isti tekst kao Flutter verzija („ID: 123456789”).
    let text = format!("{}: {}", tr("ID"), id);
    match arboard::Clipboard::new().and_then(|mut c| c.set_text(text)) {
        Ok(()) => {
            with_ui(|s| s.copied = true);
            win32::redraw(hwnd);
            SetTimer(hwnd, TIMER_COPIED, 1500, None);
        }
        Err(e) => log::error!("lagani: kopiranje ID-a nije uspjelo: {}", e),
    }
}

fn crtaj(s: &mut Stanje) {
    let id = s.id.clone();
    let status = s.status;
    let copied = s.copied;
    let red = s.red;
    s.w.crtaj(|p| {
        let pal = p.p;
        let f = p.f;
        p.c.logo(&rect(W / 2.0 - 44.0, 18.0, W / 2.0 + 44.0, 106.0));

        let title = if status > 0 {
            tr("Ready for support")
        } else if status == 0 {
            tr("Connecting to the server")
        } else {
            tr("Support is not available")
        };
        let title_font = if p.width(&title, &f.title) > W - 32.0 { &f.title_sm } else { &f.title };
        p.text(&title, title_font, &rect(16.0, 116.0, W - 16.0, 146.0), pal.text);
        let line = if status > 0 {
            Some((tr("Connected to the WiPS server"), pal.ok))
        } else if status < 0 {
            Some((tr("not_ready_status"), pal.danger))
        } else {
            None
        };
        if let Some((t, color)) = line {
            let tw = p.width(&t, &f.status);
            let x = W / 2.0 - (14.0 + tw) / 2.0;
            p.c.circle(x + 4.0, 159.0, 4.0, color);
            p.text(&t, &f.status, &rect(x + 14.0, 148.0, x + 16.0 + tw, 170.0), color);
        }

        let card = rect(16.0, 188.0, W - 16.0, 256.0);
        p.c.fill_round(&card, 12.0, pal.card);
        p.c.stroke_round(&card, 12.0, pal.border, 1.0);
        let label = tr("ID");
        let lw = p.width(&label, &f.status);
        p.text(&label, &f.status, &rect(34.0, card.top, 36.0 + lw, card.bottom), pal.muted);
        let id_text = if id.is_empty() {
            "— — —".to_owned()
        } else {
            format_id(&id)
        };
        let copy = rect(W - 66.0, 204.0, W - 30.0, 240.0);
        p.text(&id_text, &f.id, &rect(46.0 + lw, card.top, copy.left - 4.0, card.bottom), pal.text);
        if copied {
            p.c.icon(ikona::CHECK, &f.icon16, &copy, pal.ok);
        } else if !id.is_empty() {
            p.ikona_gumb(copy, A::Kopiraj, ikona::COPY, pal.muted);
        }
        let tip = if copied {
            Some(tr("Copied"))
        } else if p.is_hover(A::Kopiraj) {
            Some(tr("Copy ID"))
        } else {
            None
        };
        if let Some(tip) = tip {
            let tw = p.width(&tip, &f.small);
            let right = ((copy.left + copy.right) / 2.0 + tw / 2.0 + 10.0).min(W - 8.0);
            let r = rect(right - tw - 20.0, 172.0, right, 198.0);
            p.c.fill_round(&r, 6.0, pal.text);
            p.text(&tip, &f.small, &rect(r.left + 10.0, r.top, r.right, r.bottom), pal.card);
        }

        p.text(&tr("wips_support_tip"), &f.tip, &rect(30.0, 268.0, W - 30.0, 326.0), pal.muted);

        let y = 334.0;
        let top = y + 4.0;
        let bottom = top + 22.0;
        let lead = format!("{}  ·  ", izgled::PHONE);
        let lw = p.width(&lead, &f.small);
        p.text(&lead, &f.small, &rect(20.0, top, 22.0 + lw, bottom), pal.muted);
        let x = p.poveznica(20.0 + lw, top, bottom, A::Web, "wips.hr", &f.small);
        let ver = format!("  ·  v{}", crate::VERSION);
        p.text(&ver, &f.small, &rect(x, top, W - 44.0, bottom), pal.muted);
        let poziv = match red {
            Red::Instalacija => Some((tr("Install"), tr("wips_install_tip"), A::Instaliraj)),
            Red::Nadogradnja => Some((tr("Update"), tr("Your installation is lower version."), A::Nadogradi)),
            Red::Nista => None,
        };
        if let Some((label, tip, a)) = poziv {
            let lx = W - 48.0 - p.width(&label, &f.small);
            p.poveznica(lx, top, bottom, a, &label, &f.small);
            if p.is_hover(a) {
                let tw = p.width(&tip, &f.small).min(W - 36.0);
                let right = W - 46.0;
                let r = rect((right - tw - 20.0).max(8.0), top - 32.0, right, top - 6.0);
                p.c.fill_round(&r, 6.0, pal.text);
                p.text(&tip, &f.small, &rect(r.left + 10.0, r.top, r.right, r.bottom), pal.card);
            }
        }
        p.ikona_gumb(
            rect(W - 40.0, top - 5.0, W - 8.0, bottom + 5.0),
            A::Postavke,
            ikona::SETTINGS,
            pal.muted,
        );
    });
}
