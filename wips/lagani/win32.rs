// Tanki omotač oko Win32 API-ja (winapi) za prozore laganog sučelja: klasa i prozor, petlja poruka, sitnice.
// Crtanje je u d2d.rs (Direct2D), raspored u izgled.rs. Veličine su u logičkim pikselima (96 dpi) i množe
// se mjerilom `scale` (DPI sustava / 96).
use std::ptr::{null, null_mut};

use winapi::shared::minwindef::{FILETIME, HINSTANCE, LPARAM};
use winapi::shared::windef::{HICON, HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::processthreadsapi::{GetCurrentProcess, GetProcessTimes};
use winapi::um::shellapi::{ExtractIconW, ShellExecuteW};
use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;
use winapi::um::wingdi::{CreateSolidBrush, GetDeviceCaps, LOGPIXELSY};
use winapi::um::winuser::{
    AdjustWindowRectEx, CreateWindowExW, DispatchMessageW, GetDC, GetMessageW, GetSystemMetrics,
    GetWindowLongW, GetWindowRect, InvalidateRect, LoadCursorW, LoadIconW,
    MessageBoxW, RegisterClassW, ReleaseDC, SetForegroundWindow, SetWindowPos, SetWindowTextW,
    ShowWindow, SystemParametersInfoW, TrackMouseEvent, TranslateMessage, UpdateWindow, CS_HREDRAW,
    CS_VREDRAW, GWL_EXSTYLE, GWL_STYLE, IDC_ARROW, IDI_APPLICATION, IDYES, MB_ICONINFORMATION,
    MB_ICONQUESTION, MB_OK, MB_YESNO, MSG, SM_CXSCREEN, SM_CYSCREEN, SPI_GETWORKAREA, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_RESTORE, SW_SHOW, TME_LEAVE, TRACKMOUSEEVENT, WNDCLASSW, WNDPROC, WS_CAPTION,
    WS_EX_TOPMOST, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn px(v: i32, scale: f32) -> i32 {
    (v as f32 * scale).round() as i32
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

/// Točka miša iz LPARAM poruke, u logičkim pikselima (DIP) uz zadano mjerilo.
pub fn mouse_point(lparam: LPARAM, scale: f32) -> (f32, f32) {
    let x = (lparam & 0xFFFF) as u16 as i16 as f32;
    let y = ((lparam >> 16) & 0xFFFF) as u16 as i16 as f32;
    (x / scale, y / scale)
}

/// Traži WM_MOUSELEAVE kad miš izađe iz prozora.
pub unsafe fn track_mouse_leave(hwnd: HWND) {
    let mut t = TRACKMOUSEEVENT {
        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    TrackMouseEvent(&mut t);
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pos {
    Center,
    /// Gore desno u radnom području (prozor za odobravanje, kao u Flutter verziji).
    TopRight,
}

pub struct WindowSpec<'a> {
    pub class: &'a str,
    pub title: &'a str,
    pub pos: Pos,
    /// Vlasnik (npr. početni prozor za postavke) ili null.
    pub owner: HWND,
    /// Veličina klijentskog dijela u logičkim pikselima.
    pub width: i32,
    pub height: i32,
    pub scale: f32,
    pub background: u32,
    pub topmost: bool,
    pub wndproc: WNDPROC,
}

/// Registrira klasu, stvara prozor fiksne veličine na sredini zaslona i prikazuje ga.
pub unsafe fn create_window(spec: &WindowSpec) -> HWND {
    let class = wide(spec.class);
    let title = wide(spec.title);
    let hinst = hinstance();
    let c = spec.background;
    let bgr = ((c & 0xFF) << 16) | (c & 0xFF00) | ((c >> 16) & 0xFF);
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: spec.wndproc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinst,
        hIcon: app_icon(hinst),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: CreateSolidBrush(bgr),
        lpszMenuName: null(),
        lpszClassName: class.as_ptr(),
    };
    RegisterClassW(&wc);
    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let ex_style = if spec.topmost { WS_EX_TOPMOST } else { 0 };
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: px(spec.width, spec.scale),
        bottom: px(spec.height, spec.scale),
    };
    AdjustWindowRectEx(&mut rc, style, 0, ex_style);
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    let (x, y) = match spec.pos {
        Pos::Center => (
            (GetSystemMetrics(SM_CXSCREEN) - w) / 2,
            (GetSystemMetrics(SM_CYSCREEN) - h) / 2,
        ),
        Pos::TopRight => {
            let wa = work_area();
            let m = px(12, spec.scale);
            (wa.right - w - m, wa.top + m)
        }
    };
    let hwnd = CreateWindowExW(
        ex_style,
        class.as_ptr(),
        title.as_ptr(),
        style,
        x,
        y,
        w,
        h,
        spec.owner,
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

pub unsafe fn redraw(hwnd: HWND) {
    InvalidateRect(hwnd, null(), 0);
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

unsafe fn work_area() -> RECT {
    let mut wa: RECT = std::mem::zeroed();
    if SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut wa as *mut RECT as *mut _, 0) == 0 {
        wa.right = GetSystemMetrics(SM_CXSCREEN);
        wa.bottom = GetSystemMetrics(SM_CYSCREEN);
    }
    wa
}

/// Nova veličina klijentskog dijela (logički pikseli); `keep_right` zadržava desni rub (CM uz dopisivanje).
pub unsafe fn set_client_size(hwnd: HWND, width: i32, height: i32, scale: f32, keep_right: bool) {
    let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: px(width, scale),
        bottom: px(height, scale),
    };
    AdjustWindowRectEx(&mut rc, style, 0, ex_style);
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    let mut cur: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut cur);
    let x = if keep_right { cur.right - w } else { cur.left };
    SetWindowPos(hwnd, null_mut(), x, cur.top, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
}

pub unsafe fn set_title(hwnd: HWND, title: &str) {
    let t = wide(title);
    SetWindowTextW(hwnd, t.as_ptr());
}

/// Otvara adresu u zadanom pregledniku.
pub fn open_url(url: &str) {
    let op = wide("open");
    let u = wide(url);
    unsafe {
        ShellExecuteW(null_mut(), op.as_ptr(), u.as_ptr(), null(), null(), SW_SHOW);
    }
}

pub unsafe fn ask(hwnd: HWND, text: &str) -> bool {
    let t = wide(text);
    let c = wide(super::izgled::NAZIV);
    MessageBoxW(hwnd, t.as_ptr(), c.as_ptr(), MB_YESNO | MB_ICONQUESTION) == IDYES
}

pub unsafe fn info(hwnd: HWND, text: &str) {
    let t = wide(text);
    let c = wide(super::izgled::NAZIV);
    MessageBoxW(hwnd, t.as_ptr(), c.as_ptr(), MB_OK | MB_ICONINFORMATION);
}
