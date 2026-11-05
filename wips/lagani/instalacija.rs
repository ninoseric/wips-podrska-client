// Instalacija (proces `--install`, kao install_page.dart): putanja s izborom mape, prečaci u izborniku i na
// radnoj površini, napomena o licenci, „Prihvati i instaliraj”. Samu instalaciju (UAC, kopiranje, servis,
// pokretanje instaliranog programa) radi jezgra: ui_interface::install_me.
use std::cell::RefCell;
use std::path::Path;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, DestroyWindow, EndPaint, PostQuitMessage, PAINTSTRUCT, WM_CREATE, WM_DESTROY,
    WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR,
    WM_SETTINGCHANGE, WM_SIZE,
};

use super::d2d::rect;
use super::izgled;
use super::tr;
use super::ui::{Prozor, Vrsta};
use super::win32::{self, Pos, WindowSpec};

const CLASS: &str = "WiPSLaganiInstalacija";
const W: f32 = 460.0;
const H: i32 = 330;

#[derive(Clone, Copy, PartialEq)]
enum A {
    Putanja,
    Izbornik,
    RadnaPovrsina,
    Licenca,
    Odustani,
    Instaliraj,
    BezInstalacije,
}

struct Stanje {
    w: Prozor<A>,
    path: String,
    startmenu: bool,
    desktop: bool,
    installing: bool,
    bez_instalacije: bool,
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
    let hwnd = unsafe {
        win32::create_window(&WindowSpec {
            class: CLASS,
            title: izgled::NAZIV,
            pos: Pos::Center,
            owner: std::ptr::null_mut(),
            width: W as i32,
            height: H,
            scale: win32::scale(),
            background: super::tema::pal().bg.0,
            topmost: false,
            wndproc: Some(wndproc),
        })
    };
    if hwnd.is_null() {
        log::error!("lagani: prozor instalacije nije stvoren");
        return;
    }
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
        WM_SETTINGCHANGE => {
            with_ui(|s| s.w.osvjezi_temu(hwnd));
            0
        }
        WM_DESTROY => {
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
    // Kao install_page.dart: prečaci su uključeni osim ako ih prijašnja instalacija nije isključila.
    let opts = crate::ui_interface::install_options().replace(' ', "");
    let s = Stanje {
        w,
        path: crate::ui_interface::install_path(),
        startmenu: !opts.contains("\"STARTMENUSHORTCUTS\":\"0\""),
        desktop: !opts.contains("\"DESKTOPSHORTCUTS\":\"0\""),
        installing: false,
        bez_instalacije: crate::ui_interface::show_run_without_install(),
    };
    UI.with(|c| {
        if let Ok(mut g) = c.try_borrow_mut() {
            *g = Some(s);
        }
    });
}

unsafe fn on_action(hwnd: HWND, a: A) {
    if with_ui(|s| s.installing).unwrap_or(true) {
        return;
    }
    match a {
        A::Putanja => {
            let cur = with_ui(|s| s.path.clone()).unwrap_or_default();
            if let Some(dir) = super::sustav::izaberi_mapu(hwnd, &cur) {
                let path = Path::new(&dir).join(crate::get_app_name());
                with_ui(|s| s.path = path.to_string_lossy().into_owned());
            }
        }
        A::Izbornik => {
            with_ui(|s| s.startmenu = !s.startmenu);
        }
        A::RadnaPovrsina => {
            with_ui(|s| s.desktop = !s.desktop);
        }
        A::Licenca => win32::open_url(izgled::LICENSE_URL),
        A::Odustani => {
            DestroyWindow(hwnd);
            return;
        }
        A::BezInstalacije => crate::ui_interface::run_without_install(),
        A::Instaliraj => instaliraj(hwnd),
    }
    win32::redraw(hwnd);
}

unsafe fn instaliraj(hwnd: HWND) {
    let Some((path, startmenu, desktop)) = with_ui(|s| (s.path.clone(), s.startmenu, s.desktop)) else {
        return;
    };
    if mapa_nije_prazna(&path) {
        let text = format!("{}\n{}", tr("install-directory-not-empty-tip"), path);
        if !win32::ask(hwnd, &text) {
            return;
        }
    }
    let mut args = String::new();
    if startmenu {
        args.push_str(" startmenu");
    }
    if desktop {
        args.push_str(" desktopicon");
    }
    log::info!("lagani: instalacija u {} ({})", path, args.trim());
    with_ui(|s| s.installing = true);
    // Jezgra traži UAC, instalira, pokreće instalirani program i završava ovaj proces.
    crate::ui_interface::install_me(args, path, false, false);
}

/// Kao install_directory_confirmation.dart: upozorenje za postojeću mapu s datotekama, osim kad je to
/// mapa postojeće instalacije.
fn mapa_nije_prazna(path: &str) -> bool {
    if crate::ui_interface::is_installed() && Path::new(path) == Path::new(&crate::ui_interface::install_path()) {
        return false;
    }
    std::fs::read_dir(path)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false)
}

fn crtaj(s: &mut Stanje) {
    let path = s.path.clone();
    let (startmenu, desktop, installing, bez) = (s.startmenu, s.desktop, s.installing, s.bez_instalacije);
    s.w.crtaj(|p| {
        let pal = p.p;
        let f = p.f;
        p.text(&tr("Installation"), &f.h1, &rect(28.0, 18.0, W - 28.0, 48.0), pal.text);

        p.text(
            &format!("{}:", tr("Installation Path")),
            &f.label,
            &rect(28.0, 62.0, W - 28.0, 80.0),
            pal.muted,
        );
        let change = tr("Change Path");
        let bw = p.width(&change, &f.btn) + 52.0;
        let br = rect(W - 28.0 - bw, 84.0, W - 28.0, 120.0);
        let field = rect(28.0, 84.0, br.left - 8.0, 120.0);
        p.c.fill_round(&field, 8.0, pal.chip);
        p.text(&path, &f.body_line, &rect(field.left + 12.0, field.top, field.right - 10.0, field.bottom), pal.text);
        p.gumb(br, A::Putanja, Vrsta::Obrub, &change, Some(izgled::ikona::FOLDER), false);

        p.kvacica(rect(28.0, 136.0, W - 28.0, 164.0), A::Izbornik, startmenu, &tr("Create start menu shortcuts"));
        p.kvacica(rect(28.0, 168.0, W - 28.0, 196.0), A::RadnaPovrsina, desktop, &tr("Create desktop icon"));

        let tip = tr("agreement_tip");
        let (_, th) = p.c.measure(&tip, &f.tip_l, W - 56.0);
        p.text(&tip, &f.tip_l, &rect(28.0, 212.0, W - 28.0, 212.0 + th), pal.muted);
        p.poveznica(28.0, 214.0 + th, 236.0 + th, A::Licenca, &tr("End-user license agreement"), &f.small);

        let y = H as f32 - 56.0;
        if installing {
            p.text(&tr("Installing ..."), &f.btn_l, &rect(28.0, y, W - 28.0, y + 36.0), pal.muted);
            return;
        }
        let ok = tr("Accept and Install");
        let cancel = tr("Cancel");
        let ow = p.width(&ok, &f.btn) + 32.0;
        let cw = p.width(&cancel, &f.btn) + 32.0;
        let okr = rect(W - 28.0 - ow, y, W - 28.0, y + 36.0);
        p.gumb(okr, A::Instaliraj, Vrsta::Primarni, &ok, None, false);
        p.gumb(rect(okr.left - 8.0 - cw, y, okr.left - 8.0, y + 36.0), A::Odustani, Vrsta::Obrub, &cancel, None, false);
        if bez {
            let run = tr("Run without install");
            let rw = p.width(&run, &f.btn) + 32.0;
            p.gumb(rect(28.0, y, 28.0 + rw, y + 36.0), A::BezInstalacije, Vrsta::Obrub, &run, None, false);
        }
    });
}
