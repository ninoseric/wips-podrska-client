// Sistemski dijelovi koje lagano sučelje ne crta samo: izbornik za padajuća polja, izbor mape,
// polja za unos teksta (GDI EDIT s fontom Manrope) i obrada poruka dok se čeka jezgra.
use std::path::Path;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicIsize, Ordering};

use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HFONT, HMENU, HWND, POINT};
use winapi::um::wingdi::{
    AddFontMemResourceEx, CreateFontW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
    FW_MEDIUM, OUT_DEFAULT_PRECIS,
};
use winapi::um::winuser::{
    AppendMenuW, CallWindowProcW, ClientToScreen, CreatePopupMenu, CreateWindowExW, DestroyMenu, DispatchMessageW,
    GetWindowTextLengthW, GetWindowTextW, PeekMessageW, PostMessageW, SendMessageW, SetWindowLongPtrW, SetWindowTextW,
    TrackPopupMenu, TranslateMessage, ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_MULTILINE, ES_NUMBER, ES_PASSWORD,
    GWLP_WNDPROC, MF_CHECKED, MF_STRING, MSG, PM_REMOVE, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_TOPALIGN, VK_ESCAPE,
    VK_RETURN, VK_TAB, WM_CHAR, WM_KEYDOWN, WM_SETFONT, WS_CHILD, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::HSTRING;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS,
    SIGDN_FILESYSPATH,
};

use super::win32;

const EM_SETCUEBANNER: UINT = 0x1501;

/// Izbornik ispod točke (x, y) u DIP-ovima klijentskog dijela; vraća indeks odabrane stavke.
pub unsafe fn izbornik(hwnd: HWND, items: &[String], selected: Option<usize>, x: f32, y: f32, scale: f32) -> Option<usize> {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return None;
    }
    for (i, item) in items.iter().enumerate() {
        let w = win32::wide(item);
        let flags = MF_STRING | if selected == Some(i) { MF_CHECKED } else { 0 };
        AppendMenuW(menu, flags, i + 1, w.as_ptr());
    }
    let mut p = POINT {
        x: (x * scale).round() as i32,
        y: (y * scale).round() as i32,
    };
    ClientToScreen(hwnd, &mut p);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN,
        p.x,
        p.y,
        0,
        hwnd,
        null(),
    );
    DestroyMenu(menu);
    if cmd > 0 {
        Some(cmd as usize - 1)
    } else {
        None
    }
}

/// Dijalog za izbor mape; `current` je početna mapa (ili njen roditelj ako ne postoji).
pub unsafe fn izaberi_mapu(owner: HWND, current: &str) -> Option<String> {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
    let opts = dlg.GetOptions().ok()?;
    dlg.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
    let start = Path::new(current);
    let start = if start.is_dir() { Some(start) } else { start.parent() };
    if let Some(dir) = start {
        let p = HSTRING::from(dir.as_os_str());
        if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&p, None) {
            let _ = dlg.SetFolder(&item);
        }
    }
    dlg.Show(Some(windows::Win32::Foundation::HWND(owner as *mut _))).ok()?;
    let item = dlg.GetResult().ok()?;
    let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
    let s = name.to_string().ok();
    CoTaskMemFree(Some(name.0 as *const _));
    s
}

static FONT: AtomicIsize = AtomicIsize::new(0);
static EDIT_PROC: AtomicIsize = AtomicIsize::new(0);

/// GDI font Manrope (13 pt) za polja za unos; stvara se jednom po procesu.
unsafe fn font(scale: f32) -> HFONT {
    let f = FONT.load(Ordering::SeqCst);
    if f != 0 {
        return f as HFONT;
    }
    let data = super::d2d::manrope_medium();
    let mut count: u32 = 0;
    AddFontMemResourceEx(data.as_ptr() as *mut _, data.len() as u32, null_mut(), &mut count);
    let face = win32::wide("Manrope");
    let font = CreateFontW(
        -(13.0 * scale).round() as i32,
        0,
        0,
        0,
        FW_MEDIUM,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH,
        face.as_ptr(),
    );
    FONT.store(font as isize, Ordering::SeqCst);
    font
}

pub struct PoljeSpec<'a> {
    pub id: usize,
    pub multiline: bool,
    pub password: bool,
    pub numeric: bool,
    pub cue: &'a str,
    pub value: &'a str,
}

/// Polje za unos (EDIT) u roditeljskom prozoru; Enter, Escape i Tab javlja roditelju porukama
/// WM_POLJE_ENTER / WM_POLJE_ESCAPE / WM_POLJE_TAB (wparam = id polja).
pub unsafe fn polje(parent: HWND, spec: &PoljeSpec, scale: f32) -> HWND {
    let class = win32::wide("EDIT");
    let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
    if spec.multiline {
        style |= ES_MULTILINE | ES_AUTOVSCROLL | WS_VSCROLL;
    } else {
        style |= ES_AUTOHSCROLL;
    }
    if spec.password {
        style |= ES_PASSWORD;
    }
    if spec.numeric {
        style |= ES_NUMBER;
    }
    let edit = CreateWindowExW(
        0,
        class.as_ptr(),
        null(),
        style,
        0,
        0,
        10,
        10,
        parent,
        spec.id as HMENU,
        win32::hinstance(),
        null_mut(),
    );
    if edit.is_null() {
        return edit;
    }
    SendMessageW(edit, WM_SETFONT, font(scale) as WPARAM, 1);
    if !spec.cue.is_empty() {
        let cue = win32::wide(spec.cue);
        SendMessageW(edit, EM_SETCUEBANNER, 1, cue.as_ptr() as LPARAM);
    }
    if !spec.value.is_empty() {
        let v = win32::wide(spec.value);
        SetWindowTextW(edit, v.as_ptr());
    }
    let old = SetWindowLongPtrW(edit, GWLP_WNDPROC, edit_proc as usize as isize);
    EDIT_PROC.store(old, Ordering::SeqCst);
    edit
}

pub const WM_POLJE_ENTER: UINT = winapi::um::winuser::WM_APP + 20;
pub const WM_POLJE_ESCAPE: UINT = winapi::um::winuser::WM_APP + 21;
pub const WM_POLJE_TAB: UINT = winapi::um::winuser::WM_APP + 22;

unsafe extern "system" fn edit_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let parent = winapi::um::winuser::GetParent(hwnd);
    let id = winapi::um::winuser::GetDlgCtrlID(hwnd) as WPARAM;
    let multiline = (winapi::um::winuser::GetWindowLongW(hwnd, winapi::um::winuser::GWL_STYLE) as u32 & ES_MULTILINE) != 0;
    if msg == WM_KEYDOWN {
        if wparam == VK_RETURN as WPARAM && !multiline {
            PostMessageW(parent, WM_POLJE_ENTER, id, 0);
            return 0;
        }
        if wparam == VK_ESCAPE as WPARAM {
            PostMessageW(parent, WM_POLJE_ESCAPE, id, 0);
            return 0;
        }
        if wparam == VK_TAB as WPARAM {
            PostMessageW(parent, WM_POLJE_TAB, id, 0);
            return 0;
        }
    }
    if msg == WM_CHAR
        && (wparam == VK_TAB as WPARAM || wparam == 0x1B || (wparam == '\r' as WPARAM && !multiline))
    {
        return 0;
    }
    let old = EDIT_PROC.load(Ordering::SeqCst);
    CallWindowProcW(std::mem::transmute(old), hwnd, msg, wparam, lparam)
}

pub unsafe fn tekst(edit: HWND) -> String {
    let len = GetWindowTextLengthW(edit);
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(edit, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// Obrađuje poruke na čekanju (dok se čeka odgovor jezgre, npr. promjena ID-a), da prozor ne zamrzne.
pub unsafe fn obradi_poruke() {
    let mut msg: MSG = std::mem::zeroed();
    while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}
