// Zajednički okvir prozora: crtanje s temom, pamćenje područja za klik, prelazak mišem i klik.
// Klik vrijedi samo ako su pritisak i puštanje tipke na istom elementu: tehničarska strana pri
// uspostavi veze šalje sintetičko puštanje tipke miša na položaju pokazivača.
use std::ptr::null_mut;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, UINT};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    LoadCursorW, SetCursor, IDC_HAND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE,
};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::DirectWrite::IDWriteTextFormat;

use super::d2d::{inside, rect, Canvas, Color, Gfx};
use super::izgled::{ikona, Fonts};
use super::tema::{self, Pal};
use super::win32;

/// Poruka kojom prozor postavki javlja ostalim prozorima da ponovno pročitaju temu i jezik.
pub const WM_OSVJEZI: UINT = winapi::um::winuser::WM_APP + 1;

pub struct Prozor<A> {
    pub gfx: Gfx,
    pub fonts: Fonts,
    pub pal: Pal,
    hits: Vec<(A, D2D_RECT_F)>,
    pub hover: Option<A>,
    pressed: Option<A>,
}

impl<A: Copy + PartialEq> Prozor<A> {
    pub unsafe fn new(hwnd: HWND) -> windows::core::Result<Self> {
        let gfx = Gfx::new(windows::Win32::Foundation::HWND(hwnd as *mut _))?;
        let fonts = Fonts::new(&gfx)?;
        let pal = tema::pal();
        tema::naslovna(hwnd, pal.dark);
        Ok(Prozor {
            gfx,
            fonts,
            pal,
            hits: Vec::new(),
            hover: None,
            pressed: None,
        })
    }

    /// Područje elementa iz zadnjeg okvira (npr. za položaj izbornika ispod padajućeg polja).
    pub fn rect_of(&self, a: A) -> Option<D2D_RECT_F> {
        self.hits.iter().rev().find(|(h, _)| *h == a).map(|(_, r)| *r)
    }

    fn hit(&self, x: f32, y: f32) -> Option<A> {
        self.hits
            .iter()
            .rev()
            .find(|(_, r)| inside(r, x, y))
            .map(|(a, _)| *a)
    }

    /// Poruke miša; vraća akciju dovršenog klika.
    pub unsafe fn mis(&mut self, hwnd: HWND, msg: UINT, lparam: LPARAM) -> Option<A> {
        let (x, y) = win32::mouse_point(lparam, self.gfx.scale());
        match msg {
            WM_MOUSEMOVE => {
                let h = self.hit(x, y);
                if h != self.hover {
                    self.hover = h;
                    win32::redraw(hwnd);
                }
                win32::track_mouse_leave(hwnd);
                None
            }
            WM_MOUSELEAVE => {
                if self.hover.take().is_some() {
                    win32::redraw(hwnd);
                }
                None
            }
            WM_LBUTTONDOWN => {
                self.pressed = self.hit(x, y);
                None
            }
            WM_LBUTTONUP => {
                let h = self.hit(x, y);
                let p = self.pressed.take();
                h.filter(|h| Some(*h) == p)
            }
            _ => None,
        }
    }

    /// WM_SETCURSOR: ruka iznad elementa na koji se može kliknuti.
    pub unsafe fn kursor(&self) -> bool {
        if self.hover.is_some() {
            SetCursor(LoadCursorW(null_mut(), IDC_HAND));
            true
        } else {
            false
        }
    }

    /// Nakon promjene teme (postavke, Windows, glavni proces).
    pub unsafe fn osvjezi_temu(&mut self, hwnd: HWND) {
        self.pal = tema::pal();
        tema::naslovna(hwnd, self.pal.dark);
        win32::redraw(hwnd);
    }

    /// Područja za klik se ne dijele između okvira: elementi koji se ne nacrtaju nemaju ni klik.
    pub fn crtaj(&mut self, draw: impl FnOnce(&mut Ploha<A>)) {
        let pal = self.pal;
        let hover = self.hover;
        let fonts = &self.fonts;
        let mut hits = Vec::new();
        let res = self.gfx.paint(pal.bg, pal.dark, |c| {
            let mut p = Ploha {
                c,
                f: fonts,
                p: pal,
                hover,
                hits: Vec::new(),
                clip: None,
            };
            draw(&mut p);
            p.set_clip(None);
            hits = p.hits;
        });
        if let Err(e) = res {
            log::error!("lagani: crtanje nije uspjelo: {}", e);
        }
        self.hits = hits;
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Vrsta {
    Primarni,
    Obrub,
    Opasno,
}

pub struct Ploha<'a, A> {
    pub c: &'a Canvas<'a>,
    pub f: &'a Fonts,
    pub p: Pal,
    hover: Option<A>,
    hits: Vec<(A, D2D_RECT_F)>,
    /// Vidljivo područje sadržaja koji se pomiče: klik izvan njega ne vrijedi.
    clip: Option<D2D_RECT_F>,
}

impl<'a, A: Copy + PartialEq> Ploha<'a, A> {
    pub fn hit(&mut self, r: D2D_RECT_F, a: A) {
        let r = match self.clip {
            Some(c) => rect(r.left.max(c.left), r.top.max(c.top), r.right.min(c.right), r.bottom.min(c.bottom)),
            None => r,
        };
        if r.right > r.left && r.bottom > r.top {
            self.hits.push((a, r));
        }
    }

    pub fn set_clip(&mut self, clip: Option<D2D_RECT_F>) {
        if self.clip.is_some() {
            self.c.pop_clip();
        }
        if let Some(c) = &clip {
            self.c.push_clip(c);
        }
        self.clip = clip;
    }

    pub fn is_hover(&self, a: A) -> bool {
        self.hover == Some(a)
    }

    pub fn text(&self, s: &str, font: &IDWriteTextFormat, r: &D2D_RECT_F, color: Color) {
        self.c.text(s, font, r, color);
    }

    pub fn width(&self, s: &str, font: &IDWriteTextFormat) -> f32 {
        self.c.measure(s, font, 10_000.0).0
    }

    /// Gumb s natpisom i po želji ikonom; `velik` = 14 pt (početni prozor), inače 13 pt (CM).
    pub fn gumb(&mut self, r: D2D_RECT_F, a: A, vrsta: Vrsta, label: &str, icon: Option<char>, velik: bool) {
        let hov = self.is_hover(a);
        let p = self.p;
        let (bg, fg) = match vrsta {
            Vrsta::Primarni => (if hov { p.primary_hover } else { p.primary }, p.on_primary),
            Vrsta::Obrub => (if hov { p.chip } else { p.card }, p.text),
            Vrsta::Opasno => (if hov { p.danger_hover } else { p.danger }, Color(0xFFFFFF)),
        };
        let radius = if velik { 10.0 } else { 8.0 };
        self.c.fill_round(&r, radius, bg);
        if vrsta == Vrsta::Obrub {
            self.c.stroke_round(&r, radius, p.border, 1.0);
        }
        let (font_c, font_l) = if velik {
            (&self.f.button, &self.f.button_l)
        } else {
            (&self.f.btn, &self.f.btn_l)
        };
        match icon {
            None => self.c.text(label, font_c, &r, fg),
            Some(g) => {
                let tw = self.width(label, font_l);
                let total = 16.0 + 6.0 + tw;
                let x = ((r.left + r.right) / 2.0 - total / 2.0).max(r.left + 6.0);
                self.c.icon(g, &self.f.icon14, &rect(x, r.top, x + 16.0, r.bottom), fg);
                self.c
                    .text(label, font_l, &rect(x + 22.0, r.top, r.right - 4.0, r.bottom), fg);
            }
        }
        self.hit(r, a);
    }

    /// Okrugli gumb s ikonom (postavke, dopisivanje, slanje poruke).
    pub fn ikona_gumb(&mut self, r: D2D_RECT_F, a: A, glyph: char, color: Color) {
        if self.is_hover(a) {
            self.c.fill_round(&r, (r.bottom - r.top) / 2.0, self.p.chip);
        }
        self.c.icon(glyph, &self.f.icon16, &r, color);
        self.hit(r, a);
    }

    /// Podcrtana poveznica u retku; `x` je lijevi rub, vraća desni rub.
    pub fn poveznica(&mut self, x: f32, top: f32, bottom: f32, a: A, s: &str, font: &IDWriteTextFormat) -> f32 {
        let w = self.width(s, font);
        let color = if self.is_hover(a) { self.p.text } else { self.p.muted };
        let r = rect(x, top, x + w + 1.0, bottom);
        self.c.text(s, font, &r, color);
        let base = (top + bottom) / 2.0 + 7.0;
        self.c.line(x, base, x + w, base, color, 1.0);
        self.hit(r, a);
        x + w
    }

    /// Prekidač 30×17 (dozvole u CM prozoru); bez akcije se samo prikazuje.
    pub fn prekidac(&mut self, x: f32, cy: f32, on: bool, a: Option<A>) {
        let r = rect(x, cy - 8.5, x + 30.0, cy + 8.5);
        let bg = if on { self.p.ok } else { self.p.border };
        self.c.fill_round(&r, 8.5, bg);
        let kx = if on { r.right - 8.5 } else { r.left + 8.5 };
        self.c.circle(kx, cy, 6.5, Color(0xFFFFFF));
        if let Some(a) = a {
            self.hit(rect(r.left - 4.0, r.top - 6.0, r.right + 4.0, r.bottom + 6.0), a);
        }
    }

    /// Kvačica s natpisom; klik je na cijelom retku.
    pub fn kvacica(&mut self, r: D2D_RECT_F, a: A, on: bool, label: &str) {
        let cy = (r.top + r.bottom) / 2.0;
        let b = rect(r.left, cy - 9.0, r.left + 18.0, cy + 9.0);
        if on {
            self.c.fill_round(&b, 4.0, self.p.primary);
            self.c.icon(ikona::CHECK, &self.f.icon14, &b, self.p.on_primary);
        } else {
            self.c.fill_round(&b, 4.0, self.p.card);
            let border = if self.is_hover(a) { self.p.muted } else { self.p.border };
            self.c.stroke_round(&b, 4.0, border, 1.5);
        }
        self.c
            .text(label, &self.f.body, &rect(r.left + 28.0, r.top, r.right, r.bottom), self.p.text);
        self.hit(r, a);
    }

    /// Kvačica koja može biti onemogućena (None): tada je siva i bez klika.
    pub fn kvacica_opt(&mut self, r: D2D_RECT_F, a: Option<A>, on: bool, label: &str) {
        let Some(a) = a else {
            let cy = (r.top + r.bottom) / 2.0;
            let b = rect(r.left, cy - 9.0, r.left + 18.0, cy + 9.0);
            if on {
                self.c.fill_round(&b, 4.0, self.p.border);
                self.c.icon(ikona::CHECK, &self.f.icon14, &b, self.p.muted);
            } else {
                self.c.fill_round(&b, 4.0, self.p.chip);
                self.c.stroke_round(&b, 4.0, self.p.border, 1.5);
            }
            self.c
                .text(label, &self.f.body, &rect(r.left + 28.0, r.top, r.right, r.bottom), self.p.muted);
            return;
        };
        self.kvacica(r, a, on, label);
    }

    /// Radio gumb s natpisom; klik je na cijelom retku.
    pub fn radio(&mut self, r: D2D_RECT_F, a: Option<A>, on: bool, label: &str) {
        let cy = (r.top + r.bottom) / 2.0;
        let cx = r.left + 9.0;
        let enabled = a.is_some();
        let ring = if !enabled {
            self.p.border
        } else if on {
            self.p.primary
        } else if a.map(|a| self.is_hover(a)).unwrap_or(false) {
            self.p.muted
        } else {
            self.p.border
        };
        self.c.circle(cx, cy, 9.0, ring);
        self.c.circle(cx, cy, 7.5, self.p.card);
        if on {
            self.c.circle(cx, cy, 4.5, if enabled { self.p.primary } else { self.p.muted });
        }
        let color = if enabled { self.p.text } else { self.p.muted };
        self.c
            .text(label, &self.f.body, &rect(r.left + 28.0, r.top, r.right, r.bottom), color);
        if let Some(a) = a {
            self.hit(r, a);
        }
    }

    /// Padajuće polje: vrijednost i strelica; klik otvara izbornik (poziva ga prozor).
    pub fn padajuci(&mut self, r: D2D_RECT_F, a: Option<A>, value: &str) {
        let enabled = a.is_some();
        let hov = a.map(|a| self.is_hover(a)).unwrap_or(false);
        self.c.fill_round(&r, 8.0, if enabled { self.p.card } else { self.p.chip });
        self.c
            .stroke_round(&r, 8.0, if hov { self.p.muted } else { self.p.border }, 1.0);
        let color = if enabled { self.p.text } else { self.p.muted };
        self.c.text(
            value,
            &self.f.body_line,
            &rect(r.left + 14.0, r.top, r.right - 36.0, r.bottom),
            color,
        );
        self.c.icon(
            ikona::CHEVRON,
            &self.f.icon14,
            &rect(r.right - 34.0, r.top, r.right - 10.0, r.bottom),
            self.p.muted,
        );
        if let Some(a) = a {
            self.hit(r, a);
        }
    }

    /// Izbor jedne od mogućnosti (tema, jezik).
    pub fn segment(&mut self, r: D2D_RECT_F, items: &[(A, String)], selected: A) {
        self.c.fill_round(&r, 9.0, self.p.chip);
        if items.is_empty() {
            return;
        }
        let w = (r.right - r.left - 6.0) / items.len() as f32;
        for (i, (a, label)) in items.iter().enumerate() {
            let x = r.left + 3.0 + w * i as f32;
            let cell = rect(x, r.top + 3.0, x + w, r.bottom - 3.0);
            let sel = *a == selected;
            if sel {
                self.c.fill_round(&cell, 7.0, self.p.card);
                self.c.stroke_round(&cell, 7.0, self.p.border, 1.0);
            } else if self.is_hover(*a) {
                self.c.fill_round(&cell, 7.0, self.p.border);
            }
            let color = if sel { self.p.text } else { self.p.muted };
            self.c.text(label, &self.f.btn, &cell, color);
            self.hit(cell, *a);
        }
    }
}
