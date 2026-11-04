// Tanki omotač oko Win32 API-ja (winapi) za prozore laganog sučelja: prozor, gumbi, fontovi, crtanje.
// Veličine su u logičkim pikselima (96 dpi) i množe se mjerilom `scale` (DPI sustava / 96).
use std::ptr::{null, null_mut};

use winapi::shared::minwindef::{FILETIME, HINSTANCE, UINT, WPARAM};
use winapi::shared::windef::{COLORREF, HDC, HFONT, HGDIOBJ, HICON, HMENU, HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::processthreadsapi::{GetCurrentProcess, GetProcessTimes};
use winapi::um::shellapi::ExtractIconW;
use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;
use winapi::um::wingdi::{
    CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, Ellipse, GetDeviceCaps, GetStockObject,
    RoundRect, SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DEFAULT_PITCH, FF_DONTCARE, LOGPIXELSY, NULL_PEN, OUT_DEFAULT_PRECIS, PS_SOLID,
    TRANSPARENT,
};
use winapi::um::winuser::{
    AdjustWindowRectEx, CreateWindowExW, DispatchMessageW, DrawTextW, FillRect, GetClientRect,
    GetDC, GetMessageW, GetSystemMetrics, InvalidateRect, LoadCursorW, LoadIconW, RegisterClassW,
    ReleaseDC, SendMessageW, SetForegroundWindow, SetWindowTextW, ShowWindow, TranslateMessage,
    UpdateWindow, BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CS_HREDRAW, CS_VREDRAW, IDC_ARROW,
    IDI_APPLICATION, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_RESTORE, SW_SHOW, WM_SETFONT, WNDCLASSW,
    WNDPROC, WS_CAPTION, WS_CHILD, WS_EX_TOPMOST, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
    WS_TABSTOP, WS_VISIBLE,
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub fn px(v: i32, scale: f32) -> i32 {
    (v as f32 * scale).round() as i32
}

pub fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

pub fn hinstance() -> HINSTANCE {
    unsafe { GetModuleHandleW(null()) }
}

/// Mjerilo prema DPI-ju sustava (96 dpi = 1,0).
pub fn scale() -> f32 {
    unsafe {
        let hdc = GetDC(null_mut());
        if hdc.is_null() {
            return 1.0;
        }
        let dpi = GetDeviceCaps(hdc, LOGPIXELSY);
        ReleaseDC(null_mut(), hdc);
        if dpi > 0 {
            dpi as f32 / 96.0
        } else {
            1.0
        }
    }
}

/// Milisekunde od stvaranja procesa, za mjerenje brzine pokretanja.
pub fn ms_since_process_start() -> u64 {
    unsafe {
        let mut creation: FILETIME = std::mem::zeroed();
        let mut exit: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        if GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        ) == 0
        {
            return 0;
        }
        let mut now: FILETIME = std::mem::zeroed();
        GetSystemTimeAsFileTime(&mut now);
        let to_u64 = |t: &FILETIME| ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;
        to_u64(&now).saturating_sub(to_u64(&creation)) / 10_000
    }
}

pub struct Font(pub HFONT);

impl Font {
    pub fn new(face: &str, size_pt: f32, weight: i32, scale: f32) -> Font {
        let height = -((size_pt * scale * 96.0 / 72.0).round() as i32);
        let face = wide(face);
        let h = unsafe {
            CreateFontW(
                height,
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                DEFAULT_PITCH | FF_DONTCARE,
                face.as_ptr(),
            )
        };
        Font(h)
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                DeleteObject(self.0 as HGDIOBJ);
            }
        }
    }
}

pub unsafe fn draw_text(
    hdc: HDC,
    text: &str,
    rc: &mut RECT,
    font: &Font,
    color: COLORREF,
    flags: UINT,
) -> i32 {
    let old = SelectObject(hdc, font.0 as HGDIOBJ);
    SetTextColor(hdc, color);
    SetBkMode(hdc, TRANSPARENT as i32);
    let w = wide(text);
    let h = DrawTextW(hdc, w.as_ptr(), (w.len() - 1) as i32, rc, flags);
    SelectObject(hdc, old);
    h
}

pub unsafe fn fill_rect(hdc: HDC, rc: &RECT, color: COLORREF) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, rc, brush);
    DeleteObject(brush as HGDIOBJ);
}

/// Kartica: ispuna s tankim rubom i zaobljenim kutovima.
pub unsafe fn card(hdc: HDC, rc: &RECT, fill: COLORREF, border: COLORREF, radius: i32) {
    let pen = CreatePen(PS_SOLID as i32, 1, border);
    let brush = CreateSolidBrush(fill);
    let old_pen = SelectObject(hdc, pen as HGDIOBJ);
    let old_brush = SelectObject(hdc, brush as HGDIOBJ);
    RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, radius, radius);
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    DeleteObject(pen as HGDIOBJ);
    DeleteObject(brush as HGDIOBJ);
}

/// Puni krug (točka stanja).
pub unsafe fn dot(hdc: HDC, cx: i32, cy: i32, r: i32, color: COLORREF) {
    let brush = CreateSolidBrush(color);
    let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN as i32));
    let old_brush = SelectObject(hdc, brush as HGDIOBJ);
    Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    DeleteObject(brush as HGDIOBJ);
}

pub struct WindowSpec<'a> {
    pub class: &'a str,
    pub title: &'a str,
    /// Veličina klijentskog dijela u logičkim pikselima.
    pub width: i32,
    pub height: i32,
    pub scale: f32,
    pub background: COLORREF,
    pub topmost: bool,
    pub wndproc: WNDPROC,
}

/// Registrira klasu, stvara prozor fiksne veličine na sredini zaslona i prikazuje ga.
pub unsafe fn create_window(spec: &WindowSpec) -> HWND {
    let class = wide(spec.class);
    let title = wide(spec.title);
    let hinst = hinstance();
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: spec.wndproc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinst,
        hIcon: app_icon(hinst),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: CreateSolidBrush(spec.background),
        lpszMenuName: null(),
        lpszClassName: class.as_ptr(),
    };
    RegisterClassW(&wc);
    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let ex_style = if spec.topmost { WS_EX_TOPMOST } else { 0 };
    let mut rc = rect(0, 0, px(spec.width, spec.scale), px(spec.height, spec.scale));
    AdjustWindowRectEx(&mut rc, style, 0, ex_style);
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    let x = (GetSystemMetrics(SM_CXSCREEN) - w) / 2;
    let y = (GetSystemMetrics(SM_CYSCREEN) - h) / 2;
    let hwnd = CreateWindowExW(
        ex_style,
        class.as_ptr(),
        title.as_ptr(),
        style,
        x,
        y,
        w,
        h,
        null_mut(),
        null_mut(),
        hinst,
        null_mut(),
    );
    if hwnd.is_null() {
        return hwnd;
    }
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
    hwnd
}

/// Ikona iz samog exe-a (ugrađuje je build.rs preko winres); ako je nema, standardna ikona programa.
unsafe fn app_icon(hinst: HINSTANCE) -> HICON {
    if let Ok(exe) = std::env::current_exe() {
        let path = wide(&exe.to_string_lossy());
        let icon = ExtractIconW(hinst, path.as_ptr(), 0);
        if !icon.is_null() && icon as isize != 1 {
            return icon;
        }
    }
    LoadIconW(null_mut(), IDI_APPLICATION)
}

pub unsafe fn create_button(
    parent: HWND,
    id: u16,
    text: &str,
    rc: &RECT,
    default: bool,
    font: &Font,
) -> HWND {
    let class = wide("BUTTON");
    let text = wide(text);
    let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
    style |= if default {
        BS_DEFPUSHBUTTON
    } else {
        BS_PUSHBUTTON
    };
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        text.as_ptr(),
        style,
        rc.left,
        rc.top,
        rc.right - rc.left,
        rc.bottom - rc.top,
        parent,
        id as usize as HMENU,
        hinstance(),
        null_mut(),
    );
    if !hwnd.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font.0 as WPARAM, 1);
    }
    hwnd
}

pub unsafe fn set_text(hwnd: HWND, text: &str) {
    let t = wide(text);
    SetWindowTextW(hwnd, t.as_ptr());
}

pub unsafe fn client_rect(hwnd: HWND) -> RECT {
    let mut rc = rect(0, 0, 0, 0);
    GetClientRect(hwnd, &mut rc);
    rc
}

pub unsafe fn redraw(hwnd: HWND) {
    InvalidateRect(hwnd, null(), 1);
}

pub unsafe fn bring_to_front(hwnd: HWND) {
    ShowWindow(hwnd, SW_RESTORE);
    SetForegroundWindow(hwnd);
}

pub unsafe fn message_loop() {
    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}
