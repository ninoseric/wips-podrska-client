// Postavke korisničke verzije, kao desktop_setting_page.dart za verziju koja samo prima veze:
// Općenito (servis, tema, jezik, hardverski kodek, ulaz zvuka, snimanje, ostalo), Sigurnost
// (dopuštenja, lozinka, 2FA, ID, sigurnost) i O programu. Iste opcije i ista pravila (option2bool,
// bool2option) kao Flutter verzija; vrijednosti čita i upisuje jezgra (ui_interface).
// Otvara se iz početnog prozora, u istom procesu i istoj niti.
use std::cell::RefCell;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    BeginPaint, DefWindowProcW, EndPaint, PostMessageW, GET_WHEEL_DELTA_WPARAM, PAINTSTRUCT, WM_CREATE,
    WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT,
    WM_SETCURSOR, WM_SETTINGCHANGE, WM_SIZE,
};

use super::d2d::rect;
use super::dijalog::{self, Ishod, Polje, Spec, Uredjaj};
use super::izgled::{self, ikona};
use super::sustav;
use super::tr;
use super::ui::{Ploha, Prozor, Vrsta, WM_OSVJEZI};
use super::win32::{self, Pos, WindowSpec};
use crate::ui_interface as ui;

const CLASS: &str = "WiPSLaganiPostavke";
const W: f32 = 760.0;
const H: i32 = 620;
const NAV_W: f32 = 210.0;
const CX: f32 = NAV_W + 24.0;
const CR: f32 = W - 28.0;

#[derive(Clone, Copy, PartialEq)]
enum Kartica {
    Opcenito,
    Sigurnost,
    OProgramu,
}

#[derive(Clone, Copy, PartialEq)]
enum A {
    Nav(Kartica),
    Servis,
    Tema(usize),
    Jezik,
    Zvuk,
    Opcija(&'static str),
    TestPozadine,
    Mapa,
    OtvoriMapu,
    Otkljucaj,
    Pristup,
    Odobravanje,
    TwoFa,
    Bot,
    Uredjaji,
    Trajanje,
    PromijeniId,
    Rdp,
    Port,
    Isteka,
    IpPopis,
    IdPopis,
    Pin,
    Izvor,
    Web,
    Licenca,
}

/// Lokalne opcije (LocalConfig); ostale su opcije servera (Config preko IPC-a).
const LOKALNE: [&str; 2] = ["allow-monitor-switch-main-toolbar", "allow-monitor-switch-min-toolbar"];

/// Dopuštenja kao permissions() u Flutter verziji; zadnji element: samo ako je način privatnosti podržan.
const DOPUSTENJA: [(&str, &str); 13] = [
    ("enable-keyboard", "Enable keyboard/mouse"),
    ("enable-remote-printer", "Enable remote printer"),
    ("enable-clipboard", "Enable clipboard"),
    ("enable-file-transfer", "Enable file transfer"),
    ("enable-audio", "Enable audio"),
    ("enable-camera", "Enable camera"),
    ("enable-terminal", "Enable terminal"),
    ("enable-tunnel", "Enable TCP tunneling"),
    ("enable-remote-restart", "Enable remote restart"),
    ("enable-record-session", "Enable recording session"),
    ("enable-block-input", "Enable blocking user input"),
    ("enable-privacy-mode", "Enable privacy mode"),
    ("allow-remote-config-modification", "Enable remote configuration modification"),
];

const TEME: [&str; 3] = ["light", "dark", "system"];
const TRAJANJA: [&str; 5] = ["1", "7", "30", "90", "0"];

fn custom() -> bool {
    crate::common::is_custom_client()
}

/// option2bool iz common.dart.
fn opt2bool(key: &str, v: &str) -> bool {
    if key.starts_with("enable-") {
        v != "N"
    } else if key.starts_with("allow-") || key == "stop-service" || key == "direct-server" || key == "force-always-relay" {
        v == "Y"
    } else {
        v != "N"
    }
}

/// bool2option iz common.dart (prilagođeni klijent: zadane vrijednosti "Y" i "N").
fn bool2opt(key: &str, b: bool) -> String {
    let yes = if custom() { "Y" } else { "" };
    let no = if custom() { "N" } else { "" };
    let v = if key.starts_with("enable-")
        && key != "enable-udp-punch"
        && key != "enable-ipv6-punch"
        && key != "enable-webrtc"
    {
        if b {
            yes
        } else {
            "N"
        }
    } else if key.starts_with("allow-") || key == "stop-service" || key == "direct-server" || key == "force-always-relay" {
        if b {
            "Y"
        } else {
            no
        }
    } else if b {
        "Y"
    } else {
        "N"
    };
    v.to_owned()
}

fn lokalna(key: &str) -> bool {
    LOKALNE.contains(&key)
}

fn get_b(key: &str) -> bool {
    if lokalna(key) {
        opt2bool(key, &ui::get_local_option(key.to_owned()))
    } else {
        opt2bool(key, &ui::get_option(key))
    }
}

fn set_b(key: &str, b: bool) {
    if lokalna(key) {
        ui::set_local_option(key.to_owned(), bool2opt(key, b));
    } else {
        ui::set_option(key.to_owned(), bool2opt(key, b));
    }
}

fn fixed(key: &str) -> bool {
    ui::is_option_fixed(key)
}

fn default_whitelist() -> &'static str {
    if custom() {
        ","
    } else {
        ""
    }
}

fn popis_prazan(key: &str) -> bool {
    let v = ui::get_option(key);
    v.is_empty() || v == ","
}

fn default_lang() -> &'static str {
    if custom() {
        "default"
    } else {
        ""
    }
}

/// Jezici iz jezgre (get_langs): (kod, naziv), s „Zadano” na početku.
fn jezici() -> Vec<(String, String)> {
    let mut v = vec![(default_lang().to_owned(), tr("Default"))];
    if let Ok(serde_json::Value::Array(list)) = serde_json::from_str::<serde_json::Value>(&ui::get_langs()) {
        for item in list {
            if let (Some(code), Some(name)) = (item.get(0).and_then(|x| x.as_str()), item.get(1).and_then(|x| x.as_str())) {
                v.push((code.to_owned(), name.to_owned()));
            }
        }
    }
    v
}

/// Podaci koji se dohvaćaju u pozadini (IPC, popis zvučnih uređaja).
#[derive(Default, Clone)]
struct Pozadina {
    fingerprint: String,
    id: String,
    pin: String,
    zvuk: Vec<String>,
    gotovo: bool,
}

static POZADINA: Mutex<Option<Pozadina>> = Mutex::new(None);
static HWND_POSTAVKE: AtomicIsize = AtomicIsize::new(0);

fn pozadina() -> Pozadina {
    POZADINA.lock().ok().and_then(|g| g.clone()).unwrap_or_default()
}

fn sistemski_zvuk() -> String {
    tr("System Sound")
}

struct Stanje {
    w: Prozor<A>,
    owner: HWND,
    kartica: Kartica,
    scroll: f32,
    visina: f32,
    otkljucano: bool,
    installed: bool,
    wallpaper: bool,
    privacy: bool,
}

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
    ucitaj_pozadinu();
}

/// Otisak, ID i PIN preko IPC-a, popis mikrofona preko cpal: u pozadinskoj niti, pa osvježi prozor.
fn ucitaj_pozadinu() {
    std::thread::spawn(|| {
        let p = Pozadina {
            fingerprint: ui::get_fingerprint(),
            id: crate::ipc::get_id(),
            pin: ui::get_unlock_pin(),
            zvuk: ui::get_sound_inputs(),
            gotovo: true,
        };
        if let Ok(mut g) = POZADINA.lock() {
            *g = Some(p);
        }
        let hwnd = HWND_POSTAVKE.load(Ordering::SeqCst);
        if hwnd != 0 {
            unsafe {
                PostMessageW(hwnd as HWND, WM_OSVJEZI, 0, 0);
            }
        }
    });
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            match Prozor::new(hwnd) {
                Ok(w) => {
                    let owner = OWNER.with(|o| *o.borrow()) as HWND;
                    let s = Stanje {
                        w,
                        owner,
                        kartica: Kartica::Opcenito,
                        scroll: 0.0,
                        visina: 0.0,
                        otkljucano: false,
                        installed: ui::is_installed(),
                        wallpaper: ui::support_remove_wallpaper(),
                        privacy: !crate::privacy_mode::get_supported_privacy_mode_impl().is_empty(),
                    };
                    UI.with(|c| {
                        if let Ok(mut g) = c.try_borrow_mut() {
                            *g = Some(s);
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
        WM_MOUSEWHEEL => {
            let delta = GET_WHEEL_DELTA_WPARAM(wparam) as f32 / 120.0 * 48.0;
            with_ui(|s| {
                let max = (s.visina - H as f32).max(0.0);
                s.scroll = (s.scroll - delta).clamp(0.0, max);
            });
            win32::redraw(hwnd);
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

unsafe fn osvjezi_sve(hwnd: HWND) {
    if let Some(owner) = with_ui(|s| s.owner) {
        if !owner.is_null() {
            PostMessageW(owner, WM_OSVJEZI, 0, 0);
        }
    }
    PostMessageW(hwnd, WM_OSVJEZI, 0, 0);
}

/// Izbornik ispod padajućeg polja; vraća indeks odabrane stavke.
unsafe fn izbornik_za(hwnd: HWND, a: A, items: &[String], selected: Option<usize>) -> Option<usize> {
    let (r, scale) = with_ui(|s| (s.w.rect_of(a), s.w.gfx.scale()))?;
    let r = r?;
    sustav::izbornik(hwnd, items, selected, r.left, r.bottom + 2.0, scale)
}

unsafe fn on_action(hwnd: HWND, a: A) {
    match a {
        A::Nav(k) => {
            with_ui(|s| {
                s.kartica = k;
                s.scroll = 0.0;
            });
        }
        A::Servis => {
            let stopped = get_b("stop-service");
            log::info!("lagani: servis {}", if stopped { "pokreni" } else { "zaustavi" });
            set_b("stop-service", !stopped);
        }
        A::Tema(i) => {
            ui::set_local_option("theme".to_owned(), TEME[i].to_owned());
            osvjezi_sve(hwnd);
        }
        A::Jezik => {
            let list = jezici();
            let cur = ui::get_local_option("lang".to_owned());
            let sel = list.iter().position(|(k, _)| *k == cur).unwrap_or(0);
            let names: Vec<String> = list.iter().map(|(_, n)| n.clone()).collect();
            if let Some(i) = izbornik_za(hwnd, a, &names, Some(sel)) {
                ui::set_local_option("lang".to_owned(), list[i].0.clone());
                osvjezi_sve(hwnd);
            }
        }
        A::Zvuk => {
            let mut list = vec![sistemski_zvuk()];
            list.extend(pozadina().zvuk);
            let cur = ui::get_option("audio-input");
            let cur = if cur.is_empty() { sistemski_zvuk() } else { cur };
            let sel = list.iter().position(|d| *d == cur);
            if let Some(i) = izbornik_za(hwnd, a, &list, sel) {
                let v = if i == 0 { String::new() } else { list[i].clone() };
                ui::set_option("audio-input".to_owned(), v);
            }
        }
        A::Opcija(key) => {
            let on = !get_b(key);
            log::info!("lagani: opcija {} = {}", key, on);
            set_b(key, on);
            if key == "allow-monitor-switch-main-toolbar" && !on {
                set_b("allow-monitor-switch-min-toolbar", false);
            }
            if key == "enable-hwcodec" && on {
                ui::check_hwcodec();
            }
        }
        A::TestPozadine => {
            std::thread::spawn(|| {
                if let Ok(_remover) = crate::platform::WallPaperRemover::new() {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            });
        }
        A::Mapa => {
            let cur = ui::video_save_directory(false);
            if let Some(dir) = sustav::izaberi_mapu(hwnd, &cur) {
                ui::set_local_option("video-save-directory".to_owned(), dir);
            }
        }
        A::OtvoriMapu => {
            let installed = with_ui(|s| s.installed).unwrap_or(false);
            win32::open_url(&ui::video_save_directory(installed));
        }
        A::Otkljucaj => otkljucaj(hwnd),
        A::Pristup => {
            let keys = [if custom() { "custom" } else { "" }, "full", "view"];
            let names = vec![tr("Custom"), tr("Full Access"), tr("Screen Share")];
            let cur = ui::get_option("access-mode");
            let sel = keys.iter().position(|k| *k == cur).unwrap_or(0);
            if let Some(i) = izbornik_za(hwnd, a, &names, Some(sel)) {
                ui::set_option("access-mode".to_owned(), keys[i].to_owned());
            }
        }
        A::Odobravanje => {
            let keys = ["password", "click", if custom() { "password-click" } else { "" }];
            let names = vec![
                tr("Accept sessions via password"),
                tr("Accept sessions via click"),
                tr("Accept sessions via both"),
            ];
            let cur = ui::get_option("approve-mode");
            let sel = keys.iter().position(|k| *k == cur).unwrap_or(2);
            if let Some(i) = izbornik_za(hwnd, a, &names, Some(sel)) {
                ui::set_option("approve-mode".to_owned(), keys[i].to_owned());
            }
        }
        A::TwoFa => dvofaktorska(hwnd),
        A::Bot => bot(hwnd),
        A::Uredjaji => uredjaji(hwnd),
        A::Trajanje => {
            let names: Vec<String> = TRAJANJA
                .iter()
                .map(|d| if *d == "0" { tr("Never") } else { d.to_string() })
                .collect();
            let cur = ui::get_option("trusted-device-expiration-days");
            let sel = TRAJANJA.iter().position(|d| *d == cur).unwrap_or(3);
            if let Some(i) = izbornik_za(hwnd, a, &names, Some(sel)) {
                ui::set_option("trusted-device-expiration-days".to_owned(), TRAJANJA[i].to_owned());
            }
        }
        A::PromijeniId => promijeni_id(hwnd),
        A::Rdp => ui::set_share_rdp(!ui::is_share_rdp()),
        A::Port => broj(hwnd, "direct-access-port", &tr("Port"), "21118"),
        A::Isteka => broj(hwnd, "auto-disconnect-timeout", &tr("Timeout in minutes"), "10"),
        A::IpPopis => ip_popis(hwnd),
        A::IdPopis => id_popis(hwnd),
        A::Pin => pin(hwnd),
        A::Izvor => win32::open_url(izgled::SOURCE_URL),
        A::Web => win32::open_url(izgled::WEB),
        A::Licenca => win32::open_url(izgled::LICENSE_URL),
    }
    win32::redraw(hwnd);
}

/// Kao _lock: PIN ako je postavljen, inače provjera administratorskih prava (UAC).
unsafe fn otkljucaj(hwnd: HWND) {
    let pin = pozadina().pin;
    let ok = if pin.is_empty() || ui::get_builtin_option("disable-unlock-pin") == "Y" {
        ui::check_super_user_permission()
    } else {
        let spec = Spec {
            title: tr("Unlock Security Settings"),
            fields: vec![Polje {
                label: "PIN".to_owned(),
                password: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut provjera = |v: &[String]| {
            if v.first().map(|s| s.trim()) == Some(pin.as_str()) {
                Ok(())
            } else {
                Err(tr("Wrong PIN"))
            }
        };
        matches!(dijalog::pokazi(hwnd, spec, &mut provjera), Ishod::Ok(_))
    };
    if ok {
        with_ui(|s| s.otkljucano = true);
    }
}

unsafe fn dvofaktorska(hwnd: HWND) {
    if ui::has_valid_2fa() {
        if win32::ask(hwnd, &tr("cancel-2fa-confirm-tip")) {
            ui::set_option("2fa".to_owned(), String::new());
        }
        return;
    }
    let url = ui::generate2fa();
    let secret = url
        .split("secret=")
        .nth(1)
        .map(|s| s.split('&').next().unwrap_or(s).to_owned());
    let qr = qrcode_generator::to_matrix(&url, qrcode_generator::QrCodeEcc::Low).ok();
    let spec = Spec {
        title: tr("enable-2fa-title"),
        texts: vec![tr("enable-2fa-desc")],
        qr,
        secret,
        fields: vec![Polje {
            label: tr("Verification code"),
            numeric: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut provjera = |v: &[String]| {
        let code = v.first().map(|s| s.trim().to_owned()).unwrap_or_default();
        if ui::verify2fa(code) {
            Ok(())
        } else {
            Err(tr("wrong-2fa-code"))
        }
    };
    dijalog::pokazi(hwnd, spec, &mut provjera);
}

unsafe fn bot(hwnd: HWND) {
    if ui::has_valid_bot() {
        if win32::ask(hwnd, &tr("cancel-bot-confirm-tip")) {
            ui::set_option("bot".to_owned(), String::new());
        }
        return;
    }
    let spec = Spec {
        title: tr("Telegram bot"),
        texts: vec![tr("enable-bot-desc")],
        fields: vec![Polje {
            cue: tr("Token"),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut provjera = |v: &[String]| {
        let token = v.first().map(|s| s.trim().to_owned()).unwrap_or_default();
        if token.is_empty() {
            return Err(String::new());
        }
        let err = ui::verify_bot(token);
        if err.is_empty() {
            Ok(())
        } else {
            Err(tr(&err))
        }
    };
    dijalog::pokazi(hwnd, spec, &mut provjera);
}

fn ipc_uredjaji(request: crate::trusted_devices::authority::Request) -> Result<String, String> {
    hbb_common::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?
        .block_on(crate::ipc::trusted_devices(request))
        .map_err(|e| e.to_string())
}

unsafe fn uredjaji(hwnd: HWND) {
    use crate::trusted_devices::authority::Request;
    let json = match ipc_uredjaji(Request::List) {
        Ok(j) => j,
        Err(e) => {
            win32::info(hwnd, &e);
            return;
        }
    };
    let mut list = Vec::new();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
        if let Some(devices) = v.get("devices").and_then(|d| d.as_array()) {
            let mut sorted: Vec<&serde_json::Value> = devices.iter().collect();
            sorted.sort_by_key(|d| std::cmp::Reverse(d.get("time").and_then(|t| t.as_i64()).unwrap_or(0)));
            for d in sorted {
                let record_id = d
                    .get("record_id")
                    .and_then(|r| r.as_array())
                    .map(|r| r.iter().filter_map(|b| b.as_u64()).map(|b| b as u8).collect())
                    .unwrap_or_default();
                let s = |k: &str| d.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_owned();
                list.push(Uredjaj {
                    record_id,
                    id: s("id"),
                    name: s("name"),
                    platform: s("platform"),
                    expires_at: d.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0),
                });
            }
        }
    }
    let mut obrisi = |ids: &[Vec<u8>], all: bool| -> Result<(), String> {
        let request = if all {
            Request::Clear
        } else {
            let mut records = Vec::new();
            for id in ids {
                let rec: [u8; crate::trusted_devices::TOKEN_BYTES] =
                    id.as_slice().try_into().map_err(|_| "record id".to_owned())?;
                records.push(rec);
            }
            Request::Remove(records)
        };
        ipc_uredjaji(request).map(|_| ())
    };
    dijalog::pouzdani_uredjaji(hwnd, list, &mut obrisi);
}

/// Kao changeIdDialog: pravila za ID, promjena preko jezgre, čekanje na rezultat.
unsafe fn promijeni_id(hwnd: HWND) {
    let spec = Spec {
        title: tr("Change ID"),
        texts: vec![tr("id_change_tip")],
        fields: vec![Polje {
            label: tr("Your new ID"),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut provjera = |v: &[String]| {
        let id = v.first().map(|s| s.trim().to_owned()).unwrap_or_default();
        let mut krsenja = Vec::new();
        if !id.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
            krsenja.push(tr("starts with a letter"));
        }
        if !(6..=16).contains(&id.chars().count()) {
            krsenja.push(tr("length %min% to %max%").replace("%min%", "6").replace("%max%", "16"));
        }
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            krsenja.push(tr("allowed characters"));
        }
        if !krsenja.is_empty() {
            return Err(format!("{}:  {}", tr("Prompt"), krsenja.join(", ")));
        }
        ui::change_id(id);
        let start = std::time::Instant::now();
        let mut status = ui::get_async_job_status();
        while status == " " && start.elapsed().as_secs() < 30 {
            sustav::obradi_poruke();
            std::thread::sleep(std::time::Duration::from_millis(50));
            status = ui::get_async_job_status();
        }
        if status.is_empty() {
            Ok(())
        } else {
            Err(format!("{}: {}", tr("Prompt"), tr(&status)))
        }
    };
    if matches!(dijalog::pokazi(hwnd, spec, &mut provjera), Ishod::Ok(_)) {
        ucitaj_pozadinu();
    }
}

/// Broj (port, minute) umjesto polja s „Primijeni” u Flutter verziji.
unsafe fn broj(hwnd: HWND, key: &'static str, label: &str, hint: &str) {
    let spec = Spec {
        title: label.to_owned(),
        fields: vec![Polje {
            value: ui::get_option(key),
            cue: hint.to_owned(),
            numeric: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut provjera = |v: &[String]| {
        let t = v.first().map(|s| s.trim().to_owned()).unwrap_or_default();
        if !t.is_empty() && t.parse::<u16>().is_err() {
            return Err(String::new());
        }
        ui::set_option(key.to_owned(), t);
        Ok(())
    };
    dijalog::pokazi(hwnd, spec, &mut provjera);
}

fn valid_ip(s: &str) -> bool {
    let (addr, prefix) = match s.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (s, None),
    };
    let ok_prefix = |max: u8| prefix.map_or(true, |p| p.parse::<u8>().map_or(false, |n| (1..=max).contains(&n)));
    if addr.parse::<std::net::Ipv4Addr>().is_ok() {
        ok_prefix(32)
    } else if addr.parse::<std::net::Ipv6Addr>().is_ok() {
        ok_prefix(128)
    } else {
        false
    }
}

fn razdvoji(t: &str) -> Vec<String> {
    t.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect()
}

fn popis_tekst(key: &str) -> String {
    let cur = ui::get_option(key);
    if cur == default_whitelist() || cur == "," {
        String::new()
    } else {
        cur.split(',').collect::<Vec<_>>().join("\n")
    }
}

/// Kao changeWhiteList.
unsafe fn ip_popis(hwnd: HWND) {
    let fixed = fixed("whitelist");
    let spec = Spec {
        title: tr("IP Whitelisting"),
        texts: vec![tr("whitelist_sep"), tr("whitelist_cidr_tip")],
        fields: vec![Polje {
            value: popis_tekst("whitelist"),
            multiline: true,
            ..Default::default()
        }],
        extra: if fixed { None } else { Some(tr("Clear")) },
        ..Default::default()
    };
    let mut provjera = |v: &[String]| {
        if fixed {
            return Ok(());
        }
        let ips = razdvoji(v.first().map(|s| s.as_str()).unwrap_or_default());
        if let Some(bad) = ips.iter().find(|ip| !valid_ip(ip)) {
            return Err(format!("{} {}", tr("Invalid IP"), bad));
        }
        let value = if ips.is_empty() { default_whitelist().to_owned() } else { ips.join(",") };
        ui::set_option("whitelist".to_owned(), value);
        Ok(())
    };
    if let Ishod::Extra = dijalog::pokazi(hwnd, spec, &mut provjera) {
        ui::set_option("whitelist".to_owned(), default_whitelist().to_owned());
    }
}

/// Kao changeIdWhiteList.
unsafe fn id_popis(hwnd: HWND) {
    let fixed = fixed("id-whitelist");
    let spec = Spec {
        title: tr("ID whitelisting"),
        texts: vec![
            tr("whitelist_sep"),
            tr("id_whitelist_wildcard_tip"),
            tr("id_whitelist_caveat_tip"),
        ],
        fields: vec![Polje {
            value: popis_tekst("id-whitelist"),
            multiline: true,
            ..Default::default()
        }],
        extra: if fixed { None } else { Some(tr("Clear")) },
        ..Default::default()
    };
    let mut provjera = |v: &[String]| {
        if fixed {
            return Ok(());
        }
        let ids = razdvoji(v.first().map(|s| s.as_str()).unwrap_or_default());
        if let Some(bad) = ids
            .iter()
            .find(|id| id.chars().any(|c| (c as u32) <= 0x1f || ((c as u32) >= 0x7f && (c as u32) <= 0x9f)))
        {
            return Err(format!("{} {}", tr("Invalid ID"), bad));
        }
        let value = if ids.is_empty() { default_whitelist().to_owned() } else { ids.join(",") };
        ui::set_option("id-whitelist".to_owned(), value);
        Ok(())
    };
    if let Ishod::Extra = dijalog::pokazi(hwnd, spec, &mut provjera) {
        ui::set_option("id-whitelist".to_owned(), default_whitelist().to_owned());
    }
}

/// Kao changeUnlockPinDialog.
unsafe fn pin(hwnd: HWND) {
    let old = pozadina().pin;
    let spec = Spec {
        title: tr("Set PIN"),
        fields: vec![
            Polje {
                label: "PIN".to_owned(),
                value: old.clone(),
                password: true,
                ..Default::default()
            },
            Polje {
                label: tr("Confirmation"),
                value: old,
                password: true,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let mut provjera = |v: &[String]| {
        let pin = v.first().map(|s| s.trim().to_owned()).unwrap_or_default();
        let confirm = v.get(1).map(|s| s.trim().to_owned()).unwrap_or_default();
        if pin != confirm {
            return Err(tr("The confirmation is not identical."));
        }
        let err = ui::set_unlock_pin(pin);
        if err.is_empty() {
            Ok(())
        } else {
            Err(tr(&err))
        }
    };
    if matches!(dijalog::pokazi(hwnd, spec, &mut provjera), Ishod::Ok(_)) {
        ucitaj_pozadinu();
    }
}

/// Redak u kartici.
enum Red {
    Kvacica {
        a: Option<A>,
        on: bool,
        label: String,
        uvlaka: f32,
        gumb: Option<(A, String)>,
    },
    Radio {
        a: Option<A>,
        on: bool,
        label: String,
    },
    Padajuci {
        a: Option<A>,
        value: String,
    },
    /// Natpis i manje padajuće polje u istom retku (trajanje povjerenja).
    PadajuciRed {
        label: String,
        a: Option<A>,
        value: String,
        uvlaka: f32,
    },
    Gumb {
        a: Option<A>,
        label: String,
        icon: Option<char>,
    },
    /// „Natpis: vrijednost [Gumb]”; vrijednost je poveznica ako ima akciju.
    Vrijednost {
        label: String,
        value: String,
        link: Option<A>,
        gumb: Option<(A, String)>,
        uvlaka: f32,
    },
}

impl Red {
    fn visina(&self) -> f32 {
        match self {
            Red::Kvacica { .. } | Red::Radio { .. } => 34.0,
            Red::Padajuci { .. } | Red::Gumb { .. } => 50.0,
            Red::PadajuciRed { .. } => 46.0,
            Red::Vrijednost { .. } => 40.0,
        }
    }
}

fn opcija(key: &'static str, label: &str, enabled: bool) -> Red {
    Red::Kvacica {
        a: if enabled && !fixed(key) { Some(A::Opcija(key)) } else { None },
        on: get_b(key),
        label: tr(label),
        uvlaka: 0.0,
        gumb: None,
    }
}

fn karta(p: &mut Ploha<A>, y: &mut f32, title: &str, rows: &[Red]) {
    if rows.is_empty() {
        return;
    }
    let pal = p.p;
    let f = p.f;
    let h = 52.0 + rows.iter().map(|r| r.visina()).sum::<f32>() + 12.0;
    let card = rect(CX, *y, CR, *y + h);
    p.c.fill_round(&card, 12.0, pal.card);
    p.c.stroke_round(&card, 12.0, pal.border, 1.0);
    p.text(title, &f.name, &rect(CX + 20.0, *y + 14.0, CR - 20.0, *y + 40.0), pal.text);
    let mut ry = *y + 52.0;
    let x = CX + 20.0;
    let right = CR - 20.0;
    for r in rows {
        let rh = r.visina();
        match r {
            Red::Kvacica { a, on, label, uvlaka, gumb } => {
                let bw = gumb.as_ref().map(|(_, l)| p.width(l, &f.btn) + 28.0).unwrap_or(0.0);
                p.kvacica_opt(rect(x + uvlaka, ry, right - bw - 8.0, ry + rh), *a, *on, label);
                if let Some((ga, gl)) = gumb {
                    p.gumb(rect(right - bw, ry + 3.0, right, ry + rh - 3.0), *ga, Vrsta::Obrub, gl, None, false);
                }
            }
            Red::Radio { a, on, label } => p.radio(rect(x, ry, right, ry + rh), *a, *on, label),
            Red::Padajuci { a, value } => p.padajuci(rect(x, ry + 4.0, right, ry + 44.0), *a, value),
            Red::PadajuciRed { label, a, value, uvlaka } => {
                let lw = p.width(label, &f.body) + 16.0;
                p.text(label, &f.body, &rect(x + uvlaka, ry, x + uvlaka + lw, ry + rh), pal.text);
                p.padajuci(rect(x + uvlaka + lw, ry + 5.0, x + uvlaka + lw + 150.0, ry + rh - 5.0), *a, value);
            }
            Red::Gumb { a, label, icon } => {
                let bw = p.width(label, &f.btn) + if icon.is_some() { 62.0 } else { 40.0 };
                let br = rect(x, ry + 6.0, x + bw, ry + 44.0);
                match a {
                    Some(a) => p.gumb(br, *a, Vrsta::Primarni, label, *icon, false),
                    None => {
                        p.c.fill_round(&br, 8.0, pal.chip);
                        p.text(label, &f.btn, &br, pal.muted);
                    }
                }
            }
            Red::Vrijednost { label, value, link, gumb, uvlaka } => {
                let lx = x + uvlaka;
                let lw = p.width(label, &f.body) + 10.0;
                p.text(label, &f.body, &rect(lx, ry, lx + lw, ry + rh), pal.text);
                let bw = gumb.as_ref().map(|(_, l)| p.width(l, &f.btn) + 32.0).unwrap_or(0.0);
                let vr = rect(lx + lw, ry, right - bw - 10.0, ry + rh);
                match link {
                    Some(la) => {
                        p.poveznica(vr.left, vr.top, vr.bottom, *la, value, &f.body_line);
                    }
                    None => p.text(value, &f.body_line, &vr, pal.muted),
                }
                if let Some((ga, gl)) = gumb {
                    p.gumb(rect(right - bw, ry + 4.0, right, ry + rh - 4.0), *ga, Vrsta::Primarni, gl, None, false);
                }
            }
        }
        ry += rh;
    }
    *y += h + 16.0;
}

fn crtaj(s: &mut Stanje) {
    let kartica = s.kartica;
    let scroll = s.scroll;
    let locked = s.installed && !s.otkljucano;
    let (installed, wallpaper, privacy) = (s.installed, s.wallpaper, s.privacy);
    let poz = pozadina();
    let mut visina = 0.0;
    s.w.crtaj(|p| {
        crtaj_izbornik(p, kartica);
        let area = rect(NAV_W + 1.0, 0.0, W, H as f32);
        p.set_clip(Some(area));
        let mut y = 20.0 - scroll;
        match kartica {
            Kartica::Opcenito => opcenito(p, &mut y, installed, wallpaper),
            Kartica::Sigurnost => sigurnost(p, &mut y, locked, installed, privacy, &poz),
            Kartica::OProgramu => o_programu(p, &mut y, &poz),
        }
        p.set_clip(None);
        visina = y + scroll + 8.0;
    });
    s.visina = visina;
    let max = (visina - H as f32).max(0.0);
    if s.scroll > max {
        s.scroll = max;
    }
}

fn crtaj_izbornik(p: &mut Ploha<A>, kartica: Kartica) {
    let pal = p.p;
    let f = p.f;
    p.c.fill_round(&rect(0.0, 0.0, NAV_W, H as f32), 0.0, pal.card);
    p.c.line(NAV_W, 0.0, NAV_W, H as f32, pal.border, 1.0);
    p.text(&tr("Settings"), &f.h1, &rect(24.0, 20.0, NAV_W - 12.0, 52.0), pal.text);
    let items = [
        (Kartica::Opcenito, ikona::SETTINGS, tr("General")),
        (Kartica::Sigurnost, ikona::LOCK, tr("Security")),
        (Kartica::OProgramu, ikona::INFO, tr("About")),
    ];
    let mut y = 76.0;
    for (k, icon, label) in items.iter() {
        let r = rect(0.0, y, NAV_W, y + 42.0);
        let sel = *k == kartica;
        if sel {
            p.c.fill_round(&rect(8.0, y + 2.0, NAV_W - 8.0, y + 40.0), 8.0, pal.chip);
            p.c.fill_round(&rect(8.0, y + 10.0, 11.0, y + 32.0), 1.5, pal.primary);
        } else if p.is_hover(A::Nav(*k)) {
            p.c.fill_round(&rect(8.0, y + 2.0, NAV_W - 8.0, y + 40.0), 8.0, pal.bg);
        }
        let color = if sel { pal.text } else { pal.muted };
        p.c.icon(*icon, &f.icon16, &rect(22.0, y, 46.0, y + 42.0), color);
        p.text(label, &f.btn_l, &rect(54.0, y, NAV_W - 12.0, y + 42.0), color);
        p.hit(r, A::Nav(*k));
        y += 46.0;
    }
}

fn opcenito(p: &mut Ploha<A>, y: &mut f32, installed: bool, wallpaper: bool) {
    let stopped = get_b("stop-service");
    if !(ui::get_builtin_option("hide-stop-service") == "Y" && !stopped) {
        let label = if stopped { tr("Start") } else { tr("Stop") };
        karta(p, y, &tr("Service"), &[Red::Gumb { a: Some(A::Servis), label, icon: None }]);
    }

    let theme = ui::get_local_option("theme".to_owned());
    let cur = TEME.iter().position(|t| *t == theme).unwrap_or(2);
    let tema_fixed = fixed("theme");
    let labels = ["Light", "Dark", "Follow System"];
    let rows: Vec<Red> = (0..3)
        .map(|i| Red::Radio {
            a: if tema_fixed { None } else { Some(A::Tema(i)) },
            on: cur == i,
            label: tr(labels[i]),
        })
        .collect();
    karta(p, y, &tr("Theme"), &rows);

    let langs = jezici();
    let lang = ui::get_local_option("lang".to_owned());
    let lang_name = langs
        .iter()
        .find(|(k, _)| *k == lang)
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| tr("Default"));
    let a = if fixed("lang") { None } else { Some(A::Jezik) };
    karta(p, y, &tr("Language"), &[Red::Padajuci { a, value: lang_name }]);

    if ui::has_hwcodec() {
        karta(p, y, &tr("Hardware Codec"), &[opcija("enable-hwcodec", "Enable hardware codec", true)]);
    }

    let zvuk = ui::get_option("audio-input");
    let zvuk = if zvuk.is_empty() { sistemski_zvuk() } else { zvuk };
    karta(p, y, &tr("Audio Input Device"), &[Red::Padajuci { a: Some(A::Zvuk), value: zvuk }]);

    let dir = ui::video_save_directory(installed);
    let exists = std::path::Path::new(&dir).is_dir();
    let gumb = if installed || fixed("video-save-directory") {
        None
    } else {
        Some((A::Mapa, tr("Change")))
    };
    karta(
        p,
        y,
        &tr("Recording"),
        &[
            opcija("allow-auto-record-incoming", "Automatically record incoming sessions", true),
            Red::Vrijednost {
                label: format!("{}:", tr("Directory")),
                value: dir,
                link: if exists { Some(A::OtvoriMapu) } else { None },
                gumb,
                uvlaka: 0.0,
            },
        ],
    );

    let mut rows = vec![opcija("enable-abr", "Adaptive bitrate", true)];
    if wallpaper {
        let on = get_b("allow-remove-wallpaper");
        rows.push(Red::Kvacica {
            a: if fixed("allow-remove-wallpaper") { None } else { Some(A::Opcija("allow-remove-wallpaper")) },
            on,
            label: tr("Remove wallpaper during incoming sessions"),
            uvlaka: 0.0,
            gumb: if on { Some((A::TestPozadine, tr("Test"))) } else { None },
        });
    }
    if installed {
        rows.push(opcija("allow-auto-update", "Auto update", true));
    }
    rows.push(opcija("enable-directx-capture", "Capture screen using DirectX", true));
    rows.push(opcija(
        "allow-monitor-switch-main-toolbar",
        "Show monitor switch button on the main toolbar",
        true,
    ));
    if get_b("allow-monitor-switch-main-toolbar") {
        let mut r = opcija("allow-monitor-switch-min-toolbar", "Show on the minimized toolbar", true);
        if let Red::Kvacica { uvlaka, .. } = &mut r {
            *uvlaka = 30.0;
        }
        rows.push(r);
    }
    karta(p, y, &tr("Other"), &rows);
}

fn sigurnost(p: &mut Ploha<A>, y: &mut f32, locked: bool, installed: bool, privacy: bool, poz: &Pozadina) {
    let en = !locked;
    if locked {
        let label = tr("Unlock Security Settings");
        let bw = p.width(&label, &p.f.btn) + 70.0;
        p.gumb(rect(CX, *y, CX + bw, *y + 40.0), A::Otkljucaj, Vrsta::Primarni, &label, Some(ikona::SHIELD), false);
        *y += 56.0;
    }

    let mode = ui::get_option("access-mode");
    let (mode_i, fake) = match mode.as_str() {
        "full" => (1, Some(true)),
        "view" => (2, Some(false)),
        _ => (0, None),
    };
    let names = [tr("Custom"), tr("Full Access"), tr("Screen Share")];
    let mut rows = vec![Red::Padajuci {
        a: if en && !fixed("access-mode") { Some(A::Pristup) } else { None },
        value: names[mode_i].clone(),
    }];
    for (i, (key, label)) in DOPUSTENJA.iter().enumerate() {
        if i == 11 && !privacy {
            continue;
        }
        let mut r = opcija(key, label, en && fake.is_none());
        if let (Some(v), Red::Kvacica { on, .. }) = (fake, &mut r) {
            *on = v;
        }
        rows.push(r);
    }
    karta(p, y, &tr("Permissions"), &rows);

    let approve = ui::get_option("approve-mode");
    let approve_name = match approve.as_str() {
        "password" => tr("Accept sessions via password"),
        "click" => tr("Accept sessions via click"),
        _ => tr("Accept sessions via both"),
    };
    let a = if en && !fixed("approve-mode") { Some(A::Odobravanje) } else { None };
    karta(p, y, &tr("Password"), &[Red::Padajuci { a, value: approve_name }]);

    let has2fa = ui::has_valid_2fa();
    let mut rows = vec![Red::Kvacica {
        a: if en { Some(A::TwoFa) } else { None },
        on: has2fa,
        label: tr("enable-2fa-title"),
        uvlaka: 0.0,
        gumb: None,
    }];
    if has2fa {
        rows.push(Red::Kvacica {
            a: if en { Some(A::Bot) } else { None },
            on: ui::has_valid_bot(),
            label: tr("Telegram bot"),
            uvlaka: 30.0,
            gumb: None,
        });
        let trusted = get_b("enable-trusted-devices");
        let mut r = opcija("enable-trusted-devices", "Enable trusted devices", en);
        if let Red::Kvacica { uvlaka, gumb, .. } = &mut r {
            *uvlaka = 30.0;
            if trusted && en {
                *gumb = Some((A::Uredjaji, tr("Manage trusted devices")));
            }
        }
        rows.push(r);
        if trusted {
            let days = ui::get_option("trusted-device-expiration-days");
            let days = if TRAJANJA.contains(&days.as_str()) { days } else { "90".to_owned() };
            let value = if days == "0" { tr("Never") } else { days };
            rows.push(Red::PadajuciRed {
                label: tr("Trust duration (days)"),
                a: if en && !fixed("trusted-device-expiration-days") { Some(A::Trajanje) } else { None },
                value,
                uvlaka: 58.0,
            });
        }
    }
    karta(p, y, "2FA", &rows);

    if ui::get_builtin_option("disable-change-id") != "Y" {
        let online = ui::get_connect_status().status_num > 0;
        karta(
            p,
            y,
            &tr("ID"),
            &[Red::Gumb {
                a: if en && online { Some(A::PromijeniId) } else { None },
                label: tr("Change ID"),
                icon: None,
            }],
        );
    }

    let mut rows = Vec::new();
    if installed {
        rows.push(Red::Kvacica {
            a: if en { Some(A::Rdp) } else { None },
            on: ui::is_share_rdp(),
            label: tr("Enable RDP session sharing"),
            uvlaka: 0.0,
            gumb: None,
        });
    }
    let mut lan = opcija("enable-lan-discovery", "Deny LAN discovery", en);
    if let Red::Kvacica { on, .. } = &mut lan {
        *on = !*on;
    }
    rows.push(lan);
    rows.push(opcija("direct-server", "Enable direct IP access", en));
    if get_b("direct-server") {
        let port = ui::get_option("direct-access-port");
        rows.push(Red::Vrijednost {
            label: format!("{}:", tr("Port")),
            value: if port.is_empty() { "21118".to_owned() } else { port },
            link: None,
            gumb: if en && !fixed("direct-access-port") { Some((A::Port, tr("Change"))) } else { None },
            uvlaka: 30.0,
        });
    }
    rows.push(Red::Kvacica {
        a: if en && !fixed("whitelist") { Some(A::IpPopis) } else { None },
        on: !popis_prazan("whitelist"),
        label: tr("Use IP Whitelisting"),
        uvlaka: 0.0,
        gumb: None,
    });
    rows.push(Red::Kvacica {
        a: if en && !fixed("id-whitelist") { Some(A::IdPopis) } else { None },
        on: !popis_prazan("id-whitelist"),
        label: tr("Use ID whitelisting"),
        uvlaka: 0.0,
        gumb: None,
    });
    rows.push(opcija("allow-auto-disconnect", "auto_disconnect_option_tip", en));
    if get_b("allow-auto-disconnect") {
        let t = ui::get_option("auto-disconnect-timeout");
        rows.push(Red::Vrijednost {
            label: format!("{}:", tr("Timeout in minutes")),
            value: if t.is_empty() { "10".to_owned() } else { t },
            link: None,
            gumb: if en && !fixed("auto-disconnect-timeout") { Some((A::Isteka, tr("Change"))) } else { None },
            uvlaka: 30.0,
        });
    }
    rows.push(opcija(
        "keep-awake-during-incoming-sessions",
        "keep-awake-during-incoming-sessions-label",
        en,
    ));
    if installed {
        rows.push(opcija("allow-only-conn-window-open", "allow-only-conn-window-open-tip", en));
        if ui::get_builtin_option("disable-unlock-pin") != "Y" {
            rows.push(Red::Kvacica {
                a: if en && !fixed("whitelist") { Some(A::Pin) } else { None },
                on: !poz.pin.is_empty(),
                label: tr("Unlock with PIN"),
                uvlaka: 0.0,
                gumb: None,
            });
        }
    }
    karta(p, y, &tr("Security"), &rows);
}

fn o_programu(p: &mut Ploha<A>, y: &mut f32, poz: &Pozadina) {
    let pal = p.p;
    let f = p.f;
    let top = *y;
    let redovi = [
        (tr("Version"), crate::VERSION.to_owned()),
        (tr("Build Date"), crate::BUILD_DATE.to_owned()),
        (tr("Fingerprint"), poz.fingerprint.clone()),
        (tr("ID"), izgled::format_id(&poz.id)),
    ];
    let vx = CX + 150.0;
    let vw = CR - 20.0 - vx;
    let mut h = 100.0;
    for (_, v) in redovi.iter() {
        h += p.c.measure(v, &f.tip_l, vw).1.max(18.0) + 8.0;
    }
    let licenca = "Temelji se na RustDesku, © Purslane Tech Pte. Ltd.\nLicenca: GNU AGPL-3.0, bez ikakvog jamstva.";
    let (_, lh) = p.c.measure(licenca, &f.tip_l, CR - CX - 40.0);
    h += 40.0 + lh + 34.0;
    let card = rect(CX, top, CR, top + h);
    p.c.fill_round(&card, 12.0, pal.card);
    p.c.stroke_round(&card, 12.0, pal.border, 1.0);
    p.c.logo(&rect(CX + 16.0, top + 16.0, CX + 72.0, top + 72.0));
    p.text(izgled::NAZIV, &f.h1, &rect(CX + 86.0, top + 20.0, CR - 16.0, top + 46.0), pal.text);
    p.text(
        &format!("{} · wips.hr", izgled::PHONE),
        &f.small,
        &rect(CX + 86.0, top + 46.0, CR - 16.0, top + 66.0),
        pal.muted,
    );
    let mut ry = top + 92.0;
    for (label, value) in redovi.iter() {
        let (_, vh) = p.c.measure(value, &f.tip_l, vw);
        p.text(label, &f.label, &rect(CX + 20.0, ry, vx - 8.0, ry + 20.0), pal.muted);
        p.text(value, &f.tip_l, &rect(vx, ry, CR - 20.0, ry + vh.max(18.0)), pal.text);
        ry += vh.max(18.0) + 8.0;
    }
    ry += 4.0;
    let x = p.poveznica(CX + 20.0, ry, ry + 22.0, A::Izvor, "Izvorni kod", &f.small);
    p.poveznica(x + 20.0, ry, ry + 22.0, A::Web, &tr("Website"), &f.small);
    ry += 34.0;
    p.text(licenca, &f.tip_l, &rect(CX + 20.0, ry, CR - 20.0, ry + lh), pal.muted);
    ry += lh + 4.0;
    p.poveznica(CX + 20.0, ry, ry + 22.0, A::Licenca, izgled::LICENSE_URL, &f.small);
    *y = top + h + 16.0;
}

