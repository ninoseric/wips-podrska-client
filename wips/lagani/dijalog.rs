// Modalni dijalozi postavki, kao dialog.dart u Flutter verziji: općeniti dijalog s tekstovima, QR kodom
// i poljima za unos (promjena ID-a, 2FA, Telegram bot, popisi pouzdanih adresa, PIN, port, vrijeme), te
// popis pouzdanih uređaja. Provjeru unosa radi pozivatelj; greška ostaje u dijalogu dok se ne ispravi.
use std::cell::RefCell;
use std::ptr::null_mut;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HBRUSH, HWND};
use winapi::um::wingdi::{CreateSolidBrush, DeleteObject, SetBkColor, SetTextColor, RGB};
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, DestroyWindow, DispatchMessageW, EnableWindow, EndPaint, GetMessageW, IsWindow,
    MoveWindow, PostQuitMessage, SetFocus, SetForegroundWindow, TranslateMessage, MSG, PAINTSTRUCT, WM_CLOSE,
    WM_CREATE, WM_CTLCOLOREDIT, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE,
    WM_PAINT, WM_SETCURSOR, WM_SIZE,
};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

use super::d2d::{rect, Color};
use super::sustav::{self, PoljeSpec, WM_POLJE_ENTER, WM_POLJE_ESCAPE, WM_POLJE_TAB};
use super::tr;
use super::ui::{Prozor, Vrsta};
use super::win32::{self, Pos, WindowSpec};

const W: f32 = 460.0;
const ID_POLJE: usize = 200;

#[derive(Default)]
pub struct Polje {
    pub label: String,
    pub value: String,
    pub cue: String,
    pub multiline: bool,
    pub password: bool,
    pub numeric: bool,
}

#[derive(Default)]
pub struct Spec {
    pub title: String,
    pub texts: Vec<String>,
    pub qr: Option<Vec<Vec<bool>>>,
    pub secret: Option<String>,
    pub fields: Vec<Polje>,
    /// Dodatni gumb lijevo od „Odustani” (npr. „Očisti” za popise).
    pub extra: Option<String>,
}

pub enum Ishod {
    Ok(Vec<String>),
    Extra,
    Odustani,
}

#[derive(Clone, Copy, PartialEq)]
enum A {
    Ok,
    Odustani,
    Extra,
}

struct Raspored {
    texts: Vec<D2D_RECT_F>,
    qr: Option<D2D_RECT_F>,
    secret: Option<D2D_RECT_F>,
    labels: Vec<Option<D2D_RECT_F>>,
    boxes: Vec<D2D_RECT_F>,
    error: D2D_RECT_F,
    buttons: f32,
    height: f32,
}

struct Stanje {
    w: Prozor<A>,
    spec: Spec,
    r: Raspored,
    edits: Vec<HWND>,
    error: String,
    zahtjev: Option<A>,
    brush: HBRUSH,
}

thread_local! {
    static UI: RefCell<Option<Stanje>> = RefCell::new(None);
    static NOVI: RefCell<Option<Spec>> = RefCell::new(None);
}

fn with_ui<R>(f: impl FnOnce(&mut Stanje) -> R) -> Option<R> {
    UI.with(|c| match c.try_borrow_mut() {
        Ok(mut g) => g.as_mut().map(f),
        Err(_) => None,
    })
}

fn colorref(c: Color) -> u32 {
    RGB(((c.0 >> 16) & 0xFF) as u8, ((c.0 >> 8) & 0xFF) as u8, (c.0 & 0xFF) as u8)
}

/// Prikazuje dijalog i čeka. `provjera` se poziva na „U redu” s vrijednostima polja; Err ostavlja dijalog
/// otvoren s porukom.
pub unsafe fn pokazi(owner: HWND, spec: Spec, provjera: &mut dyn FnMut(&[String]) -> Result<(), String>) -> Ishod {
    let title = spec.title.clone();
    NOVI.with(|n| *n.borrow_mut() = Some(spec));
    let hwnd = win32::create_window(&WindowSpec {
        class: "WiPSLaganiDijalog",
        title: &title,
        pos: Pos::Center,
        owner,
        width: W as i32,
        height: 200,
        scale: win32::scale(),
        background: super::tema::pal().bg.0,
        topmost: false,
        wndproc: Some(wndproc),
    });
    if hwnd.is_null() {
        log::error!("lagani: dijalog nije stvoren");
        return Ishod::Odustani;
    }
    if !owner.is_null() {
        EnableWindow(owner, 0);
    }
    if let Some(first) = with_ui(|s| s.edits.first().copied()).flatten() {
        SetFocus(first);
    }
    let mut ishod = Ishod::Odustani;
    let mut msg: MSG = std::mem::zeroed();
    while IsWindow(hwnd) != 0 {
        if GetMessageW(&mut msg, null_mut(), 0, 0) <= 0 {
            PostQuitMessage(0);
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
        match with_ui(|s| s.zahtjev.take()).flatten() {
            Some(A::Ok) => {
                let values: Vec<String> =
                    with_ui(|s| s.edits.iter().map(|e| sustav::tekst(*e)).collect()).unwrap_or_default();
                match provjera(&values) {
                    Ok(()) => {
                        ishod = Ishod::Ok(values);
                        DestroyWindow(hwnd);
                    }
                    Err(e) => {
                        with_ui(|s| s.error = e);
                        win32::redraw(hwnd);
                    }
                }
            }
            Some(A::Extra) => {
                ishod = Ishod::Extra;
                DestroyWindow(hwnd);
            }
            Some(A::Odustani) => {
                DestroyWindow(hwnd);
            }
            None => {}
        }
    }
    if !owner.is_null() {
        EnableWindow(owner, 1);
        SetForegroundWindow(owner);
    }
    ishod
}

fn raspored(w: &Prozor<A>, spec: &Spec) -> Raspored {
    let f = &w.fonts;
    let inner = W - 48.0;
    let mut y = 64.0;
    let mut texts = Vec::new();
    for t in &spec.texts {
        let (_, h) = w.gfx.measure(t, &f.tip_l, inner);
        texts.push(rect(24.0, y, W - 24.0, y + h + 2.0));
        y += h + 10.0;
    }
    let qr = spec.qr.as_ref().map(|_| {
        let r = rect(24.0, y, 204.0, y + 180.0);
        y += 188.0;
        r
    });
    let secret = spec.secret.as_ref().map(|_| {
        let r = rect(24.0, y, W - 24.0, y + 20.0);
        y += 28.0;
        r
    });
    let mut labels = Vec::new();
    let mut boxes = Vec::new();
    for p in &spec.fields {
        if p.label.is_empty() {
            labels.push(None);
        } else {
            labels.push(Some(rect(24.0, y, W - 24.0, y + 18.0)));
            y += 22.0;
        }
        let h = if p.multiline { 120.0 } else { 38.0 };
        boxes.push(rect(24.0, y, W - 24.0, y + h));
        y += h + 10.0;
    }
    let error = rect(24.0, y, W - 24.0, y + 20.0);
    y += 26.0;
    let buttons = y;
    y += 36.0 + 20.0;
    Raspored {
        texts,
        qr,
        secret,
        labels,
        boxes,
        error,
        buttons,
        height: y,
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
                with_ui(|s| s.zahtjev = Some(a));
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
        WM_CTLCOLOREDIT => {
            let hdc = wparam as winapi::shared::windef::HDC;
            with_ui(|s| {
                SetTextColor(hdc, colorref(s.w.pal.text));
                SetBkColor(hdc, colorref(s.w.pal.chip));
                s.brush as LRESULT
            })
            .unwrap_or(0)
        }
        WM_POLJE_ENTER => {
            with_ui(|s| s.zahtjev = Some(A::Ok));
            0
        }
        WM_POLJE_ESCAPE | WM_CLOSE => {
            with_ui(|s| s.zahtjev = Some(A::Odustani));
            0
        }
        WM_POLJE_TAB => {
            let i = wparam.saturating_sub(ID_POLJE);
            if let Some(next) = with_ui(|s| s.edits.get((i + 1) % s.edits.len().max(1)).copied()).flatten() {
                SetFocus(next);
            }
            0
        }
        WM_DESTROY => {
            UI.with(|c| {
                if let Ok(mut g) = c.try_borrow_mut() {
                    if let Some(s) = g.take() {
                        DeleteObject(s.brush as _);
                    }
                }
            });
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_create(hwnd: HWND) {
    let Some(spec) = NOVI.with(|n| n.borrow_mut().take()) else {
        return;
    };
    let w = match Prozor::new(hwnd) {
        Ok(w) => w,
        Err(e) => {
            log::error!("lagani: Direct2D nije dostupan: {}", e);
            return;
        }
    };
    let r = raspored(&w, &spec);
    let scale = w.gfx.scale();
    win32::set_client_size(hwnd, W as i32, r.height.ceil() as i32, scale, false);
    let px = |v: f32| (v * scale).round() as i32;
    let mut edits = Vec::new();
    for (i, (p, b)) in spec.fields.iter().zip(r.boxes.iter()).enumerate() {
        let e = sustav::polje(
            hwnd,
            &PoljeSpec {
                id: ID_POLJE + i,
                multiline: p.multiline,
                password: p.password,
                numeric: p.numeric,
                cue: &p.cue,
                value: &p.value,
            },
            scale,
        );
        if p.multiline {
            MoveWindow(e, px(b.left + 10.0), px(b.top + 8.0), px(b.right - b.left - 20.0), px(b.bottom - b.top - 16.0), 1);
        } else {
            let top = b.top + (b.bottom - b.top - 18.0) / 2.0;
            MoveWindow(e, px(b.left + 12.0), px(top), px(b.right - b.left - 24.0), px(18.0), 1);
        }
        edits.push(e);
    }
    let brush = CreateSolidBrush(colorref(w.pal.chip));
    UI.with(|c| {
        if let Ok(mut g) = c.try_borrow_mut() {
            *g = Some(Stanje {
                w,
                spec,
                r,
                edits,
                error: String::new(),
                zahtjev: None,
                brush,
            });
        }
    });
}

fn crtaj(s: &mut Stanje) {
    let Stanje { w, spec, r, error, .. } = s;
    w.crtaj(|p| {
        let pal = p.p;
        let f = p.f;
        p.text(&spec.title, &f.h1, &rect(24.0, 18.0, W - 24.0, 50.0), pal.text);
        for (t, rr) in spec.texts.iter().zip(r.texts.iter()) {
            p.text(t, &f.tip_l, rr, pal.muted);
        }
        if let (Some(m), Some(rr)) = (&spec.qr, &r.qr) {
            p.c.qr(rr, m);
        }
        if let (Some(t), Some(rr)) = (&spec.secret, &r.secret) {
            p.text(t, &f.small, rr, pal.text);
        }
        for ((pl, lr), b) in spec.fields.iter().zip(r.labels.iter()).zip(r.boxes.iter()) {
            if let Some(lr) = lr {
                p.text(&pl.label, &f.label, lr, pal.muted);
            }
            p.c.fill_round(b, 8.0, pal.chip);
            p.c.stroke_round(b, 8.0, pal.border, 1.0);
        }
        if !error.is_empty() {
            p.text(error, &f.small, &r.error, pal.danger);
        }
        let y = r.buttons;
        let ok = tr("OK");
        let cancel = tr("Cancel");
        let ow = p.width(&ok, &f.btn) + 40.0;
        let cw = p.width(&cancel, &f.btn) + 32.0;
        let okr = rect(W - 24.0 - ow, y, W - 24.0, y + 36.0);
        p.gumb(okr, A::Ok, Vrsta::Primarni, &ok, None, false);
        let cr = rect(okr.left - 8.0 - cw, y, okr.left - 8.0, y + 36.0);
        p.gumb(cr, A::Odustani, Vrsta::Obrub, &cancel, None, false);
        if let Some(extra) = &spec.extra {
            let ew = p.width(extra, &f.btn) + 32.0;
            p.gumb(rect(cr.left - 8.0 - ew, y, cr.left - 8.0, y + 36.0), A::Extra, Vrsta::Obrub, extra, None, false);
        }
    });
}

/// Pouzdani uređaj (trusted_devices u jezgri).
pub struct Uredjaj {
    pub record_id: Vec<u8>,
    pub id: String,
    pub name: String,
    pub platform: String,
    pub expires_at: i64,
}

#[derive(Clone, Copy, PartialEq)]
enum U {
    Odaberi(usize),
    Sve,
    Obrisi,
    Zatvori,
}

struct UStanje {
    w: Prozor<U>,
    uredjaji: Vec<Uredjaj>,
    odabrani: Vec<bool>,
    zahtjev: Option<U>,
}

thread_local! {
    static UUI: RefCell<Option<UStanje>> = RefCell::new(None);
    static UNOVI: RefCell<Option<Vec<Uredjaj>>> = RefCell::new(None);
}

fn with_u<R>(f: impl FnOnce(&mut UStanje) -> R) -> Option<R> {
    UUI.with(|c| match c.try_borrow_mut() {
        Ok(mut g) => g.as_mut().map(f),
        Err(_) => None,
    })
}

const UW: f32 = 560.0;
const UH: i32 = 420;

/// Kao manageTrustedDeviceDialog: popis s odabirom i brisanjem. `obrisi(odabrani, svi)` briše u jezgri.
pub unsafe fn pouzdani_uredjaji(
    owner: HWND,
    uredjaji: Vec<Uredjaj>,
    obrisi: &mut dyn FnMut(&[Vec<u8>], bool) -> Result<(), String>,
) {
    UNOVI.with(|n| *n.borrow_mut() = Some(uredjaji));
    let hwnd = win32::create_window(&WindowSpec {
        class: "WiPSLaganiUredjaji",
        title: &tr("Manage trusted devices"),
        pos: Pos::Center,
        owner,
        width: UW as i32,
        height: UH,
        scale: win32::scale(),
        background: super::tema::pal().bg.0,
        topmost: false,
        wndproc: Some(uproc),
    });
    if hwnd.is_null() {
        return;
    }
    if !owner.is_null() {
        EnableWindow(owner, 0);
    }
    let mut msg: MSG = std::mem::zeroed();
    while IsWindow(hwnd) != 0 {
        if GetMessageW(&mut msg, null_mut(), 0, 0) <= 0 {
            PostQuitMessage(0);
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
        match with_u(|s| s.zahtjev.take()).flatten() {
            Some(U::Odaberi(i)) => {
                with_u(|s| {
                    if let Some(v) = s.odabrani.get_mut(i) {
                        *v = !*v;
                    }
                });
                win32::redraw(hwnd);
            }
            Some(U::Sve) => {
                with_u(|s| {
                    let all = s.odabrani.iter().all(|v| *v);
                    s.odabrani.iter_mut().for_each(|v| *v = !all);
                });
                win32::redraw(hwnd);
            }
            Some(U::Obrisi) => {
                let (ids, all) = with_u(|s| {
                    let ids: Vec<Vec<u8>> = s
                        .uredjaji
                        .iter()
                        .zip(s.odabrani.iter())
                        .filter(|(_, o)| **o)
                        .map(|(u, _)| u.record_id.clone())
                        .collect();
                    let all = !ids.is_empty() && ids.len() == s.uredjaji.len();
                    (ids, all)
                })
                .unwrap_or_default();
                if ids.is_empty() || !win32::ask(hwnd, &format!("{}?", tr("Confirm Delete"))) {
                    continue;
                }
                match obrisi(&ids, all) {
                    Ok(()) => {
                        with_u(|s| {
                            let keep: Vec<bool> = s.odabrani.iter().map(|o| !*o).collect();
                            let mut i = 0;
                            s.uredjaji.retain(|_| {
                                let k = keep[i];
                                i += 1;
                                k
                            });
                            s.odabrani = vec![false; s.uredjaji.len()];
                        });
                        win32::redraw(hwnd);
                    }
                    Err(e) => win32::info(hwnd, &e),
                }
            }
            Some(U::Zatvori) => {
                DestroyWindow(hwnd);
            }
            None => {}
        }
    }
    if !owner.is_null() {
        EnableWindow(owner, 1);
        SetForegroundWindow(owner);
    }
}

unsafe extern "system" fn uproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let uredjaji = UNOVI.with(|n| n.borrow_mut().take()).unwrap_or_default();
            if let Ok(w) = Prozor::new(hwnd) {
                let n = uredjaji.len();
                UUI.with(|c| {
                    if let Ok(mut g) = c.try_borrow_mut() {
                        *g = Some(UStanje {
                            w,
                            uredjaji,
                            odabrani: vec![false; n],
                            zahtjev: None,
                        });
                    }
                });
            }
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            with_u(ucrtaj);
            EndPaint(hwnd, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            with_u(|s| s.w.gfx.resize());
            0
        }
        WM_MOUSEMOVE | WM_MOUSELEAVE | WM_LBUTTONDOWN | WM_LBUTTONUP => {
            if let Some(Some(a)) = with_u(|s| s.w.mis(hwnd, msg, lparam)) {
                with_u(|s| s.zahtjev = Some(a));
            }
            0
        }
        WM_SETCURSOR => {
            if with_u(|s| s.w.kursor()).unwrap_or(false) {
                1
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_CLOSE => {
            with_u(|s| s.zahtjev = Some(U::Zatvori));
            0
        }
        WM_DESTROY => {
            UUI.with(|c| {
                if let Ok(mut g) = c.try_borrow_mut() {
                    *g = None;
                }
            });
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn dana(expires_at: i64) -> String {
    if expires_at == 0 {
        return tr("Never");
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let left = expires_at - now;
    if left < 0 {
        "0".to_owned()
    } else {
        format!("{:.0}", left as f64 / (24.0 * 60.0 * 60.0 * 1000.0))
    }
}

fn ucrtaj(s: &mut UStanje) {
    let UStanje { w, uredjaji, odabrani, .. } = s;
    w.crtaj(|p| {
        let pal = p.p;
        let f = p.f;
        let all = !odabrani.is_empty() && odabrani.iter().all(|v| *v);
        let head = rect(20.0, 16.0, UW - 20.0, 44.0);
        p.kvacica(rect(head.left + 8.0, head.top, head.left + 40.0, head.bottom), U::Sve, all, "");
        let cols = [56.0, 230.0, 380.0, 470.0];
        let hdr = [tr("Name"), tr("ID"), tr("Platform"), tr("Days remaining")];
        for (x, t) in cols.iter().zip(hdr.iter()) {
            p.text(t, &f.label, &rect(*x, head.top + 6.0, *x + 150.0, head.bottom), pal.muted);
        }
        p.c.line(20.0, 46.0, UW - 20.0, 46.0, pal.border, 1.0);
        let mut y = 50.0;
        if uredjaji.is_empty() {
            p.text(&tr("Empty"), &f.body, &rect(56.0, y, UW - 20.0, y + 32.0), pal.muted);
        }
        for (i, u) in uredjaji.iter().enumerate() {
            if y + 32.0 > UH as f32 - 70.0 {
                break;
            }
            p.kvacica(rect(28.0, y, UW - 20.0, y + 32.0), U::Odaberi(i), odabrani[i], "");
            let vals = [u.name.clone(), u.id.clone(), u.platform.clone(), dana(u.expires_at)];
            for (x, t) in cols.iter().zip(vals.iter()) {
                p.text(t, &f.body_line, &rect(*x, y, *x + 145.0, y + 32.0), pal.text);
            }
            y += 32.0;
        }
        let by = UH as f32 - 56.0;
        let close = tr("Close");
        let del = tr("Delete");
        let cw = p.width(&close, &f.btn) + 32.0;
        let dw = p.width(&del, &f.btn) + 32.0;
        let cr = rect(UW - 20.0 - cw, by, UW - 20.0, by + 36.0);
        p.gumb(cr, U::Zatvori, Vrsta::Obrub, &close, None, false);
        let dr = rect(cr.left - 8.0 - dw, by, cr.left - 8.0, by + 36.0);
        if odabrani.iter().any(|v| *v) {
            p.gumb(dr, U::Obrisi, Vrsta::Primarni, &del, None, false);
        } else {
            p.c.fill_round(&dr, 8.0, pal.chip);
            p.text(&del, &f.btn, &dr, pal.muted);
        }
    });
}
