// Svijetla i tamna tema, iste boje kao wips_incoming_home.dart i wips_cm_card.dart.
// Odabir kao u Flutter verziji: lokalna opcija "theme" ("light" / "dark"), inače tema sustava.
use std::ffi::c_void;
use std::sync::Mutex;

use winapi::shared::windef::HWND;
use windows::core::w;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

use super::d2d::Color;

#[derive(Clone, Copy, PartialEq)]
pub struct Pal {
    pub dark: bool,
    pub bg: Color,
    pub card: Color,
    pub border: Color,
    pub chip: Color,
    pub text: Color,
    pub muted: Color,
    pub primary: Color,
    pub primary_hover: Color,
    pub on_primary: Color,
    pub ok: Color,
    pub warn: Color,
    pub danger: Color,
    pub danger_hover: Color,
}

const LIGHT: Pal = Pal {
    dark: false,
    bg: Color(0xFAFAFB),
    card: Color(0xFFFFFF),
    border: Color(0xE5E7EB),
    chip: Color(0xF3F4F6),
    text: Color(0x111827),
    muted: Color(0x4B5563),
    primary: Color(0x1F2937),
    primary_hover: Color(0x374151),
    on_primary: Color(0xFFFFFF),
    ok: Color(0x15803D),
    warn: Color(0xB45309),
    danger: Color(0xB42318),
    danger_hover: Color(0x912018),
};

const DARK: Pal = Pal {
    dark: true,
    bg: Color(0x1B1C20),
    card: Color(0x25272C),
    border: Color(0x34363C),
    chip: Color(0x30333A),
    text: Color(0xF3F4F6),
    muted: Color(0xB4B9C2),
    primary: Color(0xE5E7EB),
    primary_hover: Color(0xD1D5DB),
    on_primary: Color(0x111827),
    ok: Color(0x4ADE80),
    warn: Color(0xFBBF24),
    danger: Color(0xF87171),
    danger_hover: Color(0xFCA5A5),
};

/// Tema koju je poslao glavni proces (change_theme u procesu `--cm`); prazno = čitaj opciju.
static NAMETNUTA: Mutex<String> = Mutex::new(String::new());

pub fn nametni(mode: &str) {
    if let Ok(mut t) = NAMETNUTA.lock() {
        *t = mode.to_owned();
    }
}

pub fn pal() -> Pal {
    if je_tamna() {
        DARK
    } else {
        LIGHT
    }
}

fn je_tamna() -> bool {
    let mut mode = NAMETNUTA.lock().map(|t| t.clone()).unwrap_or_default();
    if mode.is_empty() {
        mode = crate::ui_interface::get_local_option("theme".to_owned());
    }
    match mode.as_str() {
        "dark" => true,
        "light" => false,
        _ => sustav_tamni(),
    }
}

fn sustav_tamni() -> bool {
    let mut value: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };
    r == ERROR_SUCCESS && value == 0
}

/// Tamna naslovna traka prozora (Windows 10 20H1 i noviji; na starijima nema učinka).
pub unsafe fn naslovna(hwnd: HWND, dark: bool) {
    let v: i32 = dark as i32;
    let _ = DwmSetWindowAttribute(
        windows::Win32::Foundation::HWND(hwnd as *mut _),
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        &v as *const i32 as *const c_void,
        std::mem::size_of::<i32>() as u32,
    );
}
