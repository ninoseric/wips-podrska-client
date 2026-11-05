// Crtanje sučelja Direct2D-om i DirectWrite-om (oba su dio Windowsa): zaobljeni pravokutnici s antialiasingom,
// font Manrope ugrađen u exe (wips/branding/fonts, OFL), logo iz wips/branding/generirano/logo*.png,
// ikone iz sistemskog fonta Segoe MDL2 Assets (Windows 10 i 11).
// Koordinate su u DIP-ovima (logički pikseli pri 96 dpi); Direct2D ih sam množi DPI-jem prozora.
use std::ffi::c_void;

use windows::core::{w, Result};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap, ID2D1Factory, ID2D1HwndRenderTarget, D2D1_ANTIALIAS_MODE_ALIASED,
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_BITMAP_PROPERTIES, D2D1_DRAW_TEXT_OPTIONS_NONE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory5, IDWriteFontCollection1, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TEXT_METRICS, DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWRITE_WORD_WRAPPING_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

const MANROPE: [&[u8]; 4] = [
    include_bytes!("../branding/fonts/Manrope-Medium.ttf"),
    include_bytes!("../branding/fonts/Manrope-SemiBold.ttf"),
    include_bytes!("../branding/fonts/Manrope-Bold.ttf"),
    include_bytes!("../branding/fonts/Manrope-ExtraBold.ttf"),
];
const LOGO_PNG: [&[u8]; 2] = [
    include_bytes!("../branding/generirano/logo.png"),
    include_bytes!("../branding/generirano/logo_dark.png"),
];

/// Manrope Medium za GDI kontrole (polje za poruku u dopisivanju).
pub fn manrope_medium() -> &'static [u8] {
    MANROPE[0]
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color(pub u32);

impl Color {
    fn f(self) -> D2D1_COLOR_F {
        let c = self.0;
        D2D1_COLOR_F {
            r: ((c >> 16) & 0xFF) as f32 / 255.0,
            g: ((c >> 8) & 0xFF) as f32 / 255.0,
            b: (c & 0xFF) as f32 / 255.0,
            a: 1.0,
        }
    }
}

pub fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

pub fn inside(r: &D2D_RECT_F, x: f32, y: f32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct Gfx {
    d2d: ID2D1Factory,
    dw: IDWriteFactory5,
    fonts: IDWriteFontCollection1,
    hwnd: HWND,
    target: Option<ID2D1HwndRenderTarget>,
    logo: [Option<ID2D1Bitmap>; 2],
}

impl Gfx {
    pub fn new(hwnd: HWND) -> Result<Gfx> {
        unsafe {
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dw: IDWriteFactory5 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let loader = dw.CreateInMemoryFontFileLoader()?;
            dw.RegisterFontFileLoader(&loader)?;
            let builder = dw.CreateFontSetBuilder()?;
            for data in MANROPE {
                let file = loader.CreateInMemoryFontFileReference(
                    &dw,
                    data.as_ptr() as *const c_void,
                    data.len() as u32,
                    None,
                )?;
                builder.AddFontFile(&file)?;
            }
            let set = builder.CreateFontSet()?;
            let fonts = dw.CreateFontCollectionFromFontSet(&set)?;
            Ok(Gfx {
                d2d,
                dw,
                fonts,
                hwnd,
                target: None,
                logo: [None, None],
            })
        }
    }

    /// Mjerilo prozora (1,0 = 96 dpi), za pretvaranje koordinata miša u DIP-ove.
    pub fn scale(&self) -> f32 {
        unsafe { GetDpiForWindow(self.hwnd) as f32 / 96.0 }
    }

    pub fn font(&self, weight: u32, size: f32, align: Align) -> Result<IDWriteTextFormat> {
        unsafe {
            let f = self.dw.CreateTextFormat(
                w!("Manrope"),
                &self.fonts,
                DWRITE_FONT_WEIGHT(weight as i32),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                w!("hr-HR"),
            )?;
            set_align(&f, align)?;
            f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
            f.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            Ok(f)
        }
    }

    /// Font za jednoredne natpise okomito centrirane u pravokutniku (gumbi, redak stanja).
    pub fn font_vcenter(&self, weight: u32, size: f32, align: Align) -> Result<IDWriteTextFormat> {
        let f = self.font(weight, size, align)?;
        unsafe {
            f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
        }
        Ok(f)
    }

    /// Jednoredni natpis koji se, ako je predug, skraćuje trotočjem (ime tehničara, putanja).
    pub fn font_line(&self, weight: u32, size: f32, align: Align) -> Result<IDWriteTextFormat> {
        let f = self.font_vcenter(weight, size, align)?;
        unsafe {
            f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            let sign = self.dw.CreateEllipsisTrimmingSign(&f)?;
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            f.SetTrimming(&trimming, &sign)?;
        }
        Ok(f)
    }

    /// Ikone iz sistemskog fonta Segoe MDL2 Assets, centrirane u pravokutniku.
    pub fn icons(&self, size: f32) -> Result<IDWriteTextFormat> {
        unsafe {
            let f = self.dw.CreateTextFormat(
                w!("Segoe MDL2 Assets"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                w!("hr-HR"),
            )?;
            f.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            Ok(f)
        }
    }

    /// Širina i visina teksta prelomljenog na `max_w`, bez crtanja (raspored prije prvog okvira).
    pub fn measure(&self, text: &str, font: &IDWriteTextFormat, max_w: f32) -> (f32, f32) {
        measure_with(&self.dw, text, font, max_w)
    }

    pub fn resize(&mut self) {
        if let Some(t) = &self.target {
            unsafe {
                if t.Resize(&self.client_size()).is_err() {
                    self.target = None;
                }
            }
        }
    }

    fn client_size(&self) -> D2D_SIZE_U {
        let mut rc = windows::Win32::Foundation::RECT::default();
        unsafe {
            let _ = GetClientRect(self.hwnd, &mut rc);
        }
        D2D_SIZE_U {
            width: (rc.right - rc.left).max(1) as u32,
            height: (rc.bottom - rc.top).max(1) as u32,
        }
    }

    fn ensure_target(&mut self) -> Result<ID2D1HwndRenderTarget> {
        if let Some(t) = &self.target {
            return Ok(t.clone());
        }
        unsafe {
            let props = D2D1_RENDER_TARGET_PROPERTIES::default();
            let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.hwnd,
                pixelSize: self.client_size(),
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };
            let t = self.d2d.CreateHwndRenderTarget(&props, &hwnd_props)?;
            let dpi = GetDpiForWindow(self.hwnd) as f32;
            t.SetDpi(dpi, dpi);
            self.logo = [None, None];
            self.target = Some(t.clone());
            Ok(t)
        }
    }

    fn ensure_logo(&mut self, t: &ID2D1HwndRenderTarget, dark: bool) -> Option<ID2D1Bitmap> {
        let i = dark as usize;
        if self.logo[i].is_none() {
            let img = image::load_from_memory(LOGO_PNG[i]).ok()?.into_rgba8();
            let (w, h) = img.dimensions();
            let mut bgra: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
            for px in img.pixels() {
                let a = px[3] as u32;
                bgra.push((px[2] as u32 * a / 255) as u8);
                bgra.push((px[1] as u32 * a / 255) as u8);
                bgra.push((px[0] as u32 * a / 255) as u8);
                bgra.push(a as u8);
            }
            let props = D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            };
            let bmp = unsafe {
                t.CreateBitmap(
                    D2D_SIZE_U {
                        width: w,
                        height: h,
                    },
                    Some(bgra.as_ptr() as *const c_void),
                    w * 4,
                    &props,
                )
            }
            .ok()?;
            self.logo[i] = Some(bmp);
        }
        self.logo[i].clone()
    }

    /// Jedan okvir: `draw` dobiva platno; ako Direct2D traži novi render target, stvara se pri sljedećem crtanju.
    pub fn paint(&mut self, background: Color, dark: bool, draw: impl FnOnce(&Canvas)) -> Result<()> {
        let t = self.ensure_target()?;
        let logo = self.ensure_logo(&t, dark);
        unsafe {
            t.BeginDraw();
            t.Clear(Some(&background.f()));
            draw(&Canvas {
                t: &t,
                dw: &self.dw,
                logo,
            });
            if t.EndDraw(None, None).is_err() {
                self.target = None;
                self.logo = [None, None];
            }
        }
        Ok(())
    }
}

fn measure_with(dw: &IDWriteFactory5, text: &str, font: &IDWriteTextFormat, max_w: f32) -> (f32, f32) {
    let w: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        if let Ok(layout) = dw.CreateTextLayout(&w, font, max_w, 10_000.0) {
            let mut m = DWRITE_TEXT_METRICS::default();
            if layout.GetMetrics(&mut m).is_ok() {
                return (m.widthIncludingTrailingWhitespace, m.height);
            }
        }
    }
    (0.0, 0.0)
}

unsafe fn set_align(f: &IDWriteTextFormat, align: Align) -> Result<()> {
    f.SetTextAlignment(match align {
        Align::Left => DWRITE_TEXT_ALIGNMENT_LEADING,
        Align::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
        Align::Right => DWRITE_TEXT_ALIGNMENT_TRAILING,
    })
}

pub struct Canvas<'a> {
    t: &'a ID2D1HwndRenderTarget,
    dw: &'a IDWriteFactory5,
    logo: Option<ID2D1Bitmap>,
}

impl<'a> Canvas<'a> {
    pub fn fill_round(&self, r: &D2D_RECT_F, radius: f32, color: Color) {
        unsafe {
            if let Ok(b) = self.t.CreateSolidColorBrush(&color.f(), None) {
                let rr = D2D1_ROUNDED_RECT {
                    rect: *r,
                    radiusX: radius,
                    radiusY: radius,
                };
                self.t.FillRoundedRectangle(&rr, &b);
            }
        }
    }

    pub fn stroke_round(&self, r: &D2D_RECT_F, radius: f32, color: Color, width: f32) {
        unsafe {
            if let Ok(b) = self.t.CreateSolidColorBrush(&color.f(), None) {
                // Rub se crta po sredini linije, pa ga uvučemo za pola širine da ostane unutar pravokutnika.
                let h = width / 2.0;
                let rr = D2D1_ROUNDED_RECT {
                    rect: rect(r.left + h, r.top + h, r.right - h, r.bottom - h),
                    radiusX: radius,
                    radiusY: radius,
                };
                self.t.DrawRoundedRectangle(&rr, &b, width, None);
            }
        }
    }

    /// Krug kao zaobljeni kvadrat s polumjerom pola stranice (Direct2D točke bi tražile crate windows-numerics).
    pub fn circle(&self, cx: f32, cy: f32, radius: f32, color: Color) {
        self.fill_round(&rect(cx - radius, cy - radius, cx + radius, cy + radius), radius, color);
    }

    /// Vodoravna ili okomita crta debljine `width`.
    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        let h = width / 2.0;
        let r = if (y1 - y2).abs() < (x1 - x2).abs() {
            rect(x1.min(x2), y1 - h, x1.max(x2), y1 + h)
        } else {
            rect(x1 - h, y1.min(y2), x1 + h, y1.max(y2))
        };
        unsafe {
            if let Ok(b) = self.t.CreateSolidColorBrush(&color.f(), None) {
                self.t.FillRectangle(&r, &b);
            }
        }
    }

    pub fn text(&self, text: &str, font: &IDWriteTextFormat, r: &D2D_RECT_F, color: Color) {
        let w: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            if let Ok(b) = self.t.CreateSolidColorBrush(&color.f(), None) {
                self.t.DrawText(
                    &w,
                    font,
                    r,
                    &b,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
        }
    }

    pub fn icon(&self, glyph: char, font: &IDWriteTextFormat, r: &D2D_RECT_F, color: Color) {
        let mut buf = [0u8; 4];
        self.text(glyph.encode_utf8(&mut buf), font, r, color);
    }

    /// Širina i visina teksta prelomljenog na `max_w`.
    pub fn measure(&self, text: &str, font: &IDWriteTextFormat, max_w: f32) -> (f32, f32) {
        measure_with(self.dw, text, font, max_w)
    }

    /// Crni kvadratići QR koda na bijeloj podlozi s rubom od četiri modula.
    pub fn qr(&self, r: &D2D_RECT_F, matrix: &[Vec<bool>]) {
        let n = matrix.len();
        if n == 0 {
            return;
        }
        self.fill_round(r, 0.0, Color(0xFFFFFF));
        let cell = ((r.right - r.left) / (n as f32 + 8.0)).floor().max(1.0);
        let size = cell * n as f32;
        let x0 = (r.left + (r.right - r.left - size) / 2.0).round();
        let y0 = (r.top + (r.bottom - r.top - size) / 2.0).round();
        unsafe {
            // Bez zaglađivanja: susjedni moduli se inače razdvoje tankim svijetlim crtama.
            self.t.SetAntialiasMode(D2D1_ANTIALIAS_MODE_ALIASED);
            if let Ok(b) = self.t.CreateSolidColorBrush(&Color(0x000000).f(), None) {
                for (y, row) in matrix.iter().enumerate() {
                    for (x, on) in row.iter().enumerate() {
                        if *on {
                            let c = rect(
                                x0 + x as f32 * cell,
                                y0 + y as f32 * cell,
                                x0 + (x + 1) as f32 * cell,
                                y0 + (y + 1) as f32 * cell,
                            );
                            self.t.FillRectangle(&c, &b);
                        }
                    }
                }
            }
            self.t.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        }
    }

    pub fn push_clip(&self, r: &D2D_RECT_F) {
        unsafe {
            self.t.PushAxisAlignedClip(r, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        }
    }

    pub fn pop_clip(&self) {
        unsafe {
            self.t.PopAxisAlignedClip();
        }
    }

    pub fn logo(&self, r: &D2D_RECT_F) {
        if let Some(bmp) = &self.logo {
            unsafe {
                self.t
                    .DrawBitmap(bmp, Some(r), 1.0, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
            }
        }
    }
}
