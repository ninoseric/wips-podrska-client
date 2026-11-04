// Postavke korisničke verzije: tema i jezik (iste lokalne opcije "theme" i "lang" kao Flutter verzija)
// i „O programu” (verzija, datum izrade, otisak, ID, izvorni kod, licenca, kao zakrpa 0002).
// Otvara se iz početnog prozora, u istom procesu i istoj niti.
use std::cell::RefCell;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, EndPaint, PostMessageW, PAINTSTRUCT, WM_CREATE, WM_DESTROY, WM_ERASEBKGND,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WM_SETTINGCHANGE, WM_SIZE,
};

use super::d2d::rect;
use super::izgled;
use super::tr;
use super::ui::{Prozor, WM_OSVJEZI};
use super::win32::{self, Pos, WindowSpec};

const CLASS: &str = "WiPSLaganiPostavke";
const W: f32 = 400.0;
const H: i32 = 560;

const TEME: [&str; 3] = ["system", "light", "dark"];
const JEZICI: [&str; 3] = ["", "hr", "en"];

#[derive(Clone, Copy, PartialEq)]
enum A {
    Tema(usize),
    Jezik(usize),
    Izvor,
    Web,
    Licenca,
}

struct Stanje {
    w: Prozor<A>,
    owner: HWND,
}

static HWND_POSTAVKE: AtomicIsize = AtomicIsize::new(0);
/// Otisak i ID se dohvaćaju u pozadini (ID ide preko IPC-a).
static PODACI: Mutex<(String, String)> = Mutex::new((String::new(), String::new()));

thread_local! {
    static UI: RefCell<Option<Stanje>> = RefCell::new(None);
    static OWNER: RefCell<usize> = RefCell::new(0);
}

fn with_ui<R>(f: impl FnOnce(&mut Stanje) -> R) -> Option<R> {
    UI.with(|c| match c.try_borrow_mut() {
        Ok(mut g) => g.as_mut().map(f),
        Err(_) => None,
    })
}

pub unsafe fn otvori(owner: HWND) {
    let existing = HWND_POSTAVKE.load(Ordering::SeqCst);
    if existing != 0 {
        win32::bring_to_front(existing as HWND);
        return;
    }
    OWNER.with(|o| *o.borrow_mut() = owner as usize);
    let hwnd = win32::create_window(&WindowSpec {
        class: CLASS,
        title: &tr("Settings"),
        pos: Pos::Center,
        owner,
        width: W as i32,
        height: H,
        scale: win32::scale(),
        background: super::tema::pal().bg.0,
        topmost: false,
        wndproc: Some(wndproc),
    });
    if hwnd.is_null() {
        log::error!("lagani: prozor postavki nije stvoren");
        return;
    }
    HWND_POSTAVKE.store(hwnd as isize, Ordering::SeqCst);
    let target = hwnd as usize;
    std::thread::spawn(move || {
        let fingerprint = crate::ui_interface::get_fingerprint();
        let id = crate::ipc::get_id();
        if let Ok(mut d) = PODACI.lock() {
            *d = (fingerprint, id);
        }
        PostMessageW(target as HWND, WM_OSVJEZI, 0, 0);
    });
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            match Prozor::new(hwnd) {
                Ok(w) => {
                    let owner = OWNER.with(|o| *o.borrow()) as HWND;
                    UI.with(|c| {
                        if let Ok(mut g) = c.try_borrow_mut() {
                            *g = Some(Stanje { w, owner });
                        }
                    });
                }
                Err(e) => log::error!("lagani: Direct2D nije dostupan: {}", e),
            }
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
            win32::set_title(hwnd, &tr("Settings"));
            0
        }
        WM_DESTROY => {
            HWND_POSTAVKE.store(0, Ordering::SeqCst);
            UI.with(|c| {
                if let Ok(mut g) = c.try_borrow_mut() {
                    *g = None;
                }
            });
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_action(hwnd: HWND, a: A) {
    match a {
        A::Tema(i) => {
            log::info!("lagani: tema {}", TEME[i]);
            crate::ui_interface::set_local_option("theme".to_owned(), TEME[i].to_owned());
            osvjezi_sve(hwnd);
        }
        A::Jezik(i) => {
            log::info!("lagani: jezik {:?}", JEZICI[i]);
            crate::ui_interface::set_local_option("lang".to_owned(), JEZICI[i].to_owned());
            osvjezi_sve(hwnd);
        }
        A::Izvor => win32::open_url(izgled::SOURCE_URL),
        A::Web => win32::open_url(izgled::WEB),
        A::Licenca => win32::open_url(izgled::LICENSE_URL),
    }
}

unsafe fn osvjezi_sve(hwnd: HWND) {
    if let Some(owner) = with_ui(|s| s.owner) {
        if !owner.is_null() {
            PostMessageW(owner, WM_OSVJEZI, 0, 0);
        }
    }
    PostMessageW(hwnd, WM_OSVJEZI, 0, 0);
}

fn odabir(opcija: &str, mogucnosti: &[&str]) -> usize {
    let v = crate::ui_interface::get_local_option(opcija.to_owned());
    mogucnosti.iter().position(|m| !m.is_empty() && *m == v).unwrap_or(0)
}

fn crtaj(s: &mut Stanje) {
    let tema = odabir("theme", &TEME);
    let jezik = odabir("lang", &JEZICI);
    let (fingerprint, id) = PODACI.lock().map(|d| d.clone()).unwrap_or_default();
    s.w.crtaj(|p| {
        let pal = p.p;
        let f = p.f;
        p.text(&tr("Theme"), &f.btn_l, &rect(20.0, 16.0, W - 20.0, 40.0), pal.text);
        let teme = [
            (A::Tema(0), tr("Follow System")),
            (A::Tema(1), tr("Light")),
            (A::Tema(2), tr("Dark")),
        ];
        p.segment(rect(20.0, 42.0, W - 20.0, 80.0), &teme, A::Tema(tema));

        p.text(&tr("Language"), &f.btn_l, &rect(20.0, 96.0, W - 20.0, 120.0), pal.text);
        let jezici = [
            (A::Jezik(0), tr("Default")),
            (A::Jezik(1), "Hrvatski".to_owned()),
            (A::Jezik(2), "English".to_owned()),
        ];
        p.segment(rect(20.0, 122.0, W - 20.0, 160.0), &jezici, A::Jezik(jezik));

        p.text(&tr("About"), &f.btn_l, &rect(20.0, 176.0, W - 20.0, 200.0), pal.text);
        let card = rect(20.0, 202.0, W - 20.0, H as f32 - 20.0);
        p.c.fill_round(&card, 12.0, pal.card);
        p.c.stroke_round(&card, 12.0, pal.border, 1.0);
        p.c.logo(&rect(36.0, 218.0, 84.0, 266.0));
        p.text(izgled::NAZIV, &f.h1, &rect(98.0, 220.0, W - 36.0, 246.0), pal.text);
        p.text(
            &format!("{} · wips.hr", izgled::PHONE),
            &f.small,
            &rect(98.0, 246.0, W - 36.0, 266.0),
            pal.muted,
        );

        let mut y = 284.0;
        let redovi = [
            (tr("Version"), crate::VERSION.to_owned()),
            (tr("Build Date"), crate::BUILD_DATE.to_owned()),
            (tr("Fingerprint"), fingerprint),
            (tr("ID"), izgled::format_id(&id)),
        ];
        for (label, value) in redovi.iter() {
            let vx = 150.0;
            let vw = W - 36.0 - vx;
            let (_, vh) = p.c.measure(value, &f.tip_l, vw);
            p.text(label, &f.label, &rect(36.0, y, vx - 8.0, y + 20.0), pal.muted);
            p.text(value, &f.tip_l, &rect(vx, y, W - 36.0, y + vh.max(18.0)), pal.text);
            y += vh.max(18.0) + 8.0;
        }

        y += 4.0;
        let x = p.poveznica(36.0, y, y + 22.0, A::Izvor, "Izvorni kod", &f.small);
        p.poveznica(x + 20.0, y, y + 22.0, A::Web, &tr("Website"), &f.small);
        y += 34.0;

        let licenca = "Temelji se na RustDesku, © Purslane Tech Pte. Ltd.\nLicenca: GNU AGPL-3.0, bez ikakvog jamstva.";
        let (_, lh) = p.c.measure(licenca, &f.tip_l, W - 72.0);
        p.text(licenca, &f.tip_l, &rect(36.0, y, W - 36.0, y + lh), pal.muted);
        y += lh + 4.0;
        p.poveznica(36.0, y, y + 22.0, A::Licenca, izgled::LICENSE_URL, &f.small);
    });
}
