// Lokalni testni okvir: prevodi i pokreće pravi wips/lagani/mod.rs sa zamjenama za RustDesk jezgru.
//   lagani-proba                 početni prozor (ID 121179531, spojeno, nije instaliran)
//   lagani-proba --cm            prozor za odobravanje (lažni zahtjev, nakon „Prihvati” poruka tehničara)
//   lagani-proba --install       instalacija
// Varijable okoline:
//   LAGANI_TEMA=dark|light       tema (lokalna opcija "theme")
//   LAGANI_JEZIK=en              jezik (lokalna opcija "lang")
//   LAGANI_KLIK=x,y;x,y;...      klikovi (DIP) u prozor nakon 2 s, razmak 1,5 s
//   LAGANI_DVA=1                 u --cm dvije veze
//   LAGANI_POZIV=1               u --cm dolazni glasovni poziv nakon prihvaćanja
//   LAGANI_TEKST=...             nakon klikova upiše tekst u polje za poruku i pritisne Enter
#![allow(dead_code, non_snake_case)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub const VERSION: &str = "1.5.1";
pub const BUILD_DATE: &str = "2026-11-04 12:00";

pub fn get_app_name() -> String {
    "WiPSPodrska".to_owned()
}

static OPCIJE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn opcija(key: &str) -> String {
    let mut g = OPCIJE.lock().unwrap();
    let m = g.get_or_insert_with(|| {
        let mut m = HashMap::new();
        m.insert("theme".to_owned(), std::env::var("LAGANI_TEMA").unwrap_or_default());
        m.insert("lang".to_owned(), std::env::var("LAGANI_JEZIK").unwrap_or_default());
        m
    });
    m.get(key).cloned().unwrap_or_default()
}

pub mod lang {
    pub fn translate(name: String) -> String {
        if super::opcija("lang") == "en" {
            return name;
        }
        let hr = [
            ("Ready for support", "Spremno za podršku"),
            ("Connecting to the server", "Povezivanje sa serverom"),
            ("Support is not available", "Podrška trenutno nije dostupna"),
            ("Connected to the WiPS server", "Povezano sa serverom WiPS"),
            ("not_ready_status", "Nije spremno. Provjerite vezu."),
            ("Copy ID", "Kopiraj ID"),
            ("Copied", "Kopirano"),
            ("wips_support_tip", "Nazovite podršku na 01/61 76 166 i pročitajte ove podatke tehničaru. Spajanje uvijek morate odobriti vi."),
            ("wips_install_tip", "Za puni pristup instalirajte aplikaciju."),
            ("wips_approve_tip", "Odobrite samo ako ste upravo razgovarali s podrškom WiPS. Vezu možete prekinuti u bilo kojem trenutku."),
            ("Install", "Instaliraj"),
            ("Settings", "Postavke"),
            ("Your installation is lower version.", "Vaša instalacija je niže verzije"),
            ("Click to upgrade", "Klik za nadogradnju"),
            ("Request access to your device", "Zahtjev za pristup vašem uređaju"),
            ("Connected", "Spojeno"),
            ("Disconnected", "Odspojeno"),
            ("Transfer file", "Prijenos datoteke"),
            ("Chat", "Dopisivanje"),
            ("Permissions", "Dopuštenja"),
            ("Enable keyboard/mouse", "Dopusti tipkovnicu/miša"),
            ("Enable clipboard", "Dopusti međuspremnik"),
            ("Enable audio", "Dopusti zvuk"),
            ("Enable file copy and paste", "Dopusti kopiranje i lijepljenje datoteka"),
            ("Enable remote restart", "Omogući daljinsko ponovno pokretanje"),
            ("Enable recording session", "Omogući snimanje sesije"),
            ("Enable blocking user input", "Omogući blokiranje korisničkog unosa"),
            ("Enable privacy mode", "Omogući način privatnosti"),
            ("Accept", "Prihvati"),
            ("Accept and Elevate", "Prihvati povećanje"),
            ("Cancel", "Otkaži"),
            ("Elevate", "Izdigni"),
            ("Disconnect", "Prekini vezu"),
            ("Close", "Zatvori"),
            ("Dismiss", "Odbaci"),
            ("Stop voice call", "Zaustavi glasovni poziv"),
            ("Write a message", "Napiši poruku"),
            ("Disconnect all devices?", "Odspojiti sve uređaje?"),
            ("Installation", "Instalacija"),
            ("Installation Path", "Putanja za instalaciju"),
            ("Change Path", "Promijeni putanju"),
            ("Create start menu shortcuts", "Stvori prečace u izborniku"),
            ("Create desktop icon", "Stvori ikonu na radnoj površini"),
            ("agreement_tip", "Pokretanjem instalacije prihvaćate ugovor o licenciranju."),
            ("End-user license agreement", "Ugovor sa krajnjim korisnikom"),
            ("Accept and Install", "Prihvati i instaliraj"),
            ("Run without install", "Pokreni bez instalacije"),
            ("Installing ...", "Instaliranje..."),
            ("install-directory-not-empty-tip", "Odabrana mapa nije prazna. Instalacija može izbrisati postojeće podatke. Razmislite o odabiru druge mape."),
            ("Theme", "Tema"),
            ("Follow System", "Tema sutava"),
            ("Light", "Svjetla"),
            ("Dark", "Tamna"),
            ("Language", "Jezik"),
            ("Default", "Zadano"),
            ("About", "O programu"),
            ("Version", "Verzija"),
            ("Build Date", "Datum izrade"),
            ("Fingerprint", "Otisak"),
            ("Website", "Web stranica"),
        ];
        hr.iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_string())
            .unwrap_or(name)
    }
}

pub mod ipc {
    pub fn get_id() -> String {
        "121179531".to_owned()
    }
}

pub mod platform {
    pub fn quit_gui() {
        std::process::exit(0);
    }
}

pub mod privacy_mode {
    pub fn get_supported_privacy_mode_impl() -> Vec<(&'static str, &'static str)> {
        vec![("privacy_mode_impl_exclude_from_capture", "")]
    }
}

pub mod ui_interface {
    pub struct UiStatus {
        pub status_num: i32,
        pub id: String,
    }
    pub fn get_connect_status() -> UiStatus {
        UiStatus { status_num: 1, id: "121179531".to_owned() }
    }
    pub fn start_option_status_sync() {}
    pub fn get_local_option(key: String) -> String {
        super::opcija(&key)
    }
    pub fn set_local_option(key: String, value: String) {
        let mut g = super::OPCIJE.lock().unwrap();
        if let Some(m) = g.as_mut() {
            m.insert(key, value);
        }
    }
    pub fn get_builtin_option(_key: &str) -> String {
        String::new()
    }
    pub fn get_option<T: AsRef<str>>(key: T) -> String {
        if key.as_ref() == "approve-mode" { "click".to_owned() } else { String::new() }
    }
    pub fn get_fingerprint() -> String {
        "a1b2 c3d4 e5f6 0718 293a 4b5c 6d7e 8f90 a1b2 c3d4 e5f6 0718 293a 4b5c 6d7e 8f90".to_owned()
    }
    pub fn is_installed() -> bool {
        std::env::var("LAGANI_INSTALIRAN").is_ok()
    }
    pub fn is_installed_lower_version() -> bool {
        std::env::var("LAGANI_STARIJA").is_ok()
    }
    pub fn goto_install() {
        println!("proba: goto_install");
    }
    pub fn update_me(_path: String) {
        println!("proba: update_me");
    }
    pub fn install_path() -> String {
        "C:\\Program Files\\WiPSPodrska".to_owned()
    }
    pub fn install_options() -> String {
        "{}".to_owned()
    }
    pub fn show_run_without_install() -> bool {
        true
    }
    pub fn run_without_install() {
        println!("proba: run_without_install");
    }
    pub fn install_me(options: String, path: String, _silent: bool, _debug: bool) {
        println!("proba: install_me({:?}, {:?})", options, path);
    }
}

pub mod ui_cm_interface {
    use super::*;

    #[derive(Clone, Default)]
    pub struct Client {
        pub id: i32,
        pub authorized: bool,
        pub disconnected: bool,
        pub is_file_transfer: bool,
        pub is_view_camera: bool,
        pub is_terminal: bool,
        pub port_forward: String,
        pub name: String,
        pub avatar: String,
        pub peer_id: String,
        pub keyboard: bool,
        pub clipboard: bool,
        pub audio: bool,
        pub file: bool,
        pub restart: bool,
        pub recording: bool,
        pub block_input: bool,
        pub privacy_mode: bool,
        pub from_switch: bool,
        pub in_voice_call: bool,
        pub incoming_voice_call: bool,
    }
    #[derive(Clone)]
    pub struct ConnectionManager<T: InvokeUiCM> {
        pub ui_handler: T,
    }
    pub trait InvokeUiCM: Send + Clone + 'static + Sized {
        fn add_connection(&self, client: &Client);
        fn remove_connection(&self, id: i32, close: bool);
        fn new_message(&self, id: i32, text: String);
        fn change_theme(&self, dark: String);
        fn change_language(&self);
        fn show_elevation(&self, show: bool);
        fn update_voice_call_state(&self, client: &Client);
        fn file_transfer_log(&self, action: &str, log: &str);
    }

    pub static AUTORIZIRAN: AtomicBool = AtomicBool::new(false);

    fn tehnicar(id: i32, name: &str, peer_id: &str) -> Client {
        Client {
            id,
            name: name.to_owned(),
            peer_id: peer_id.to_owned(),
            keyboard: true,
            clipboard: true,
            audio: true,
            file: true,
            restart: true,
            ..Default::default()
        }
    }

    pub fn start_ipc<T: InvokeUiCM>(cm: ConnectionManager<T>) {
        let ms = |v| std::thread::sleep(std::time::Duration::from_millis(v));
        ms(800);
        if std::env::var("LAGANI_DVA").is_ok() {
            cm.ui_handler.add_connection(&tehnicar(2, "Ivan", "123456789"));
        }
        cm.ui_handler.add_connection(&tehnicar(1, "Tehničar", "987654321"));
        cm.ui_handler.show_elevation(true);
        while !AUTORIZIRAN.load(Ordering::SeqCst) {
            ms(100);
        }
        ms(1500);
        cm.ui_handler.new_message(1, "Dobar dan, ovdje tehničar WiPS-a. Vidim vaš zaslon.".to_owned());
        if std::env::var("LAGANI_POZIV").is_ok() {
            let mut c = tehnicar(1, "Tehničar", "987654321");
            c.authorized = true;
            c.incoming_voice_call = true;
            cm.ui_handler.update_voice_call_state(&c);
        }
        loop {
            ms(60_000);
        }
    }
    pub fn authorize(id: i32) {
        println!("proba: authorize({})", id);
        AUTORIZIRAN.store(true, Ordering::SeqCst);
    }
    pub fn close(id: i32) {
        println!("proba: close({})", id);
        std::process::exit(0);
    }
    pub fn remove(id: i32) {
        println!("proba: remove({})", id);
    }
    pub fn switch_permission(id: i32, name: String, enabled: bool) {
        println!("proba: switch_permission({}, {}, {})", id, name, enabled);
    }
    pub fn can_elevate() -> bool {
        true
    }
    pub fn elevate_portable(id: i32) {
        println!("proba: elevate_portable({})", id);
    }
    pub fn send_chat(id: i32, text: String) {
        println!("proba: send_chat({}, {:?})", id, text);
    }
    pub fn handle_incoming_voice_call(id: i32, accept: bool) {
        println!("proba: handle_incoming_voice_call({}, {})", id, accept);
    }
    pub fn close_voice_call(id: i32) {
        println!("proba: close_voice_call({})", id);
    }
}

#[path = "../../mod.rs"]
mod lagani;

/// Simulirani klikovi (pritisak i puštanje) u prvi prozor ovog procesa, u DIP koordinatama.
fn klikovi() {
    let Ok(spec) = std::env::var("LAGANI_KLIK") else {
        return;
    };
    std::thread::spawn(move || unsafe {
        use winapi::um::winuser::*;
        std::thread::sleep(std::time::Duration::from_millis(2000));
        for par in spec.split(';') {
            let mut it = par.split(',').map(|v| v.trim().parse::<f32>().unwrap_or(0.0));
            let (x, y) = (it.next().unwrap_or(0.0), it.next().unwrap_or(0.0));
            let pid = std::process::id();
            let mut found: winapi::shared::windef::HWND = std::ptr::null_mut();
            let mut h = GetTopWindow(std::ptr::null_mut());
            while !h.is_null() {
                let mut wpid = 0u32;
                GetWindowThreadProcessId(h, &mut wpid);
                if wpid == pid && IsWindowVisible(h) != 0 {
                    found = h;
                    break;
                }
                h = GetWindow(h, GW_HWNDNEXT);
            }
            if !found.is_null() {
                let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(windows::Win32::Foundation::HWND(found as *mut _));
                let s = dpi as f32 / 96.0;
                let lp = (((y * s) as isize) << 16) | ((x * s) as isize & 0xFFFF);
                PostMessageW(found, WM_MOUSEMOVE, 0, lp);
                PostMessageW(found, WM_LBUTTONDOWN, 1, lp);
                PostMessageW(found, WM_LBUTTONUP, 0, lp);
                println!("proba: klik {},{}", x, y);
            }
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
        if let Ok(tekst) = std::env::var("LAGANI_TEKST") {
            let class: Vec<u16> = "WiPSLaganiCm ".encode_utf16().collect();
            let edit_class: Vec<u16> = "EDIT ".encode_utf16().collect();
            let cm = FindWindowW(class.as_ptr(), std::ptr::null());
            let edit = FindWindowExW(cm, std::ptr::null_mut(), edit_class.as_ptr(), std::ptr::null());
            let t: Vec<u16> = tekst.encode_utf16().chain(std::iter::once(0)).collect();
            SetWindowTextW(edit, t.as_ptr());
            PostMessageW(edit, WM_KEYDOWN, VK_RETURN as usize, 0);
            println!("proba: upisano {:?}", tekst);
        }
    });
}

fn main() {
    unsafe {
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
    klikovi();
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    lagani::start(&mut args);
}
