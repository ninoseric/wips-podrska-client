// Prozor za odobravanje i tijek veze (proces `--cm`), kao wips_cm_card.dart i _CmControlPanel:
// zaglavlje (tehničar, ID, vrsta veze, stanje i trajanje), uputa, dozvole s prekidačima, gumbi
// (prihvati, prihvati i povisi prava, odustani, povisi prava, prekini vezu, zatvori, glasovni poziv),
// kartice za više veza i dopisivanje s tehničarom.
// Logika (IPC sa serverom, odobravanje, dozvole, poruke) je RustDeskova (ui_cm_interface).
use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use hbb_common::log;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HBRUSH, HFONT, HMENU, HWND};
use winapi::um::wingdi::{
    AddFontMemResourceEx, CreateFontW, CreateSolidBrush, DeleteObject, SetBkColor, SetTextColor, CLEARTYPE_QUALITY,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, FW_MEDIUM, OUT_DEFAULT_PRECIS, RGB,
};
use winapi::um::winuser::{
    BeginPaint, CallWindowProcW, CreateWindowExW, DefWindowProcW, EndPaint, FlashWindowEx, GetForegroundWindow,
    GetWindowTextLengthW, GetWindowTextW, IsIconic, KillTimer, MoveWindow, PostMessageW, PostQuitMessage, SendMessageW,
    SetFocus, SetTimer, SetWindowLongPtrW, ShowWindow, FLASHWINFO, FLASHW_ALL, FLASHW_TIMERNOFG, GET_WHEEL_DELTA_WPARAM,
    GWLP_WNDPROC, PAINTSTRUCT, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, VK_RETURN, WM_CHAR, WM_CLOSE, WM_CREATE,
    WM_CTLCOLOREDIT, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSELEAVE,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_SETCURSOR, WM_SETFONT, WM_SETTINGCHANGE, WM_SIZE, WM_TIMER,
    WS_CHILD, WS_TABSTOP,
};

use super::d2d::{rect, Color};
use super::izgled::{self, format_id, ikona};
use super::tema::{self, Pal};
use super::tr;
use super::ui::{Ploha, Prozor, Vrsta, WM_OSVJEZI};
use super::win32::{self, Pos, WindowSpec};
use crate::ui_cm_interface::{self, Client, ConnectionManager, InvokeUiCM};

const CLASS: &str = "WiPSLaganiCm";
const CM_W: f32 = 340.0;
const CM_H: i32 = 580;
/// Kartice za više istodobnih veza.
const TABS_H: i32 = 36;
const CHAT_W: f32 = 360.0;
const TIMER_SEC: usize = 1;
/// Jezgra je javila promjenu (nova veza, poruka, dozvola, ...): ponovno nacrtaj.
const WM_PROMJENA: UINT = winapi::um::winuser::WM_APP + 2;
/// Enter u polju za poruku.
const WM_POSALJI: UINT = winapi::um::winuser::WM_APP + 3;
const ES_AUTOHSCROLL: u32 = 0x0080;
const EM_SETCUEBANNER: UINT = 0x1501;
const ID_EDIT: usize = 100;

#[derive(Clone)]
struct Peer {
    id: i32,
    name: String,
    peer_id: String,
    authorized: bool,
    disconnected: bool,
    file_transfer: bool,
    camera: bool,
    terminal: bool,
    port_forward: String,
    perms: [bool; 8],
    in_voice_call: bool,
    incoming_voice_call: bool,
    authorized_at: Option<Instant>,
    ended_at: Option<Instant>,
}

/// Dozvole kao u _WipsPermissionList: naziv za switch_permission, natpis i ikona.
const PERMS: [(&str, &str, char); 8] = [
    ("keyboard", "Enable keyboard/mouse", ikona::KEYBOARD),
    ("clipboard", "Enable clipboard", ikona::CLIPBOARD),
    ("audio", "Enable audio", ikona::AUDIO),
    ("file", "Enable file copy and paste", ikona::FILE),
    ("restart", "Enable remote restart", ikona::RESTART),
    ("recording", "Enable recording session", ikona::RECORD),
    ("block_input", "Enable blocking user input", ikona::BLOCK),
    ("privacy_mode", "Enable privacy mode", ikona::PRIVACY),
];

impl Peer {
    fn from(c: &Client) -> Peer {
        Peer {
            id: c.id,
            name: c.name.clone(),
            peer_id: c.peer_id.clone(),
            authorized: c.authorized,
            disconnected: c.disconnected,
            file_transfer: c.is_file_transfer,
            camera: c.is_view_camera,
            terminal: c.is_terminal,
            port_forward: c.port_forward.clone(),
            perms: [
                c.keyboard,
                c.clipboard,
                c.audio,
                c.file,
                c.restart,
                c.recording,
                c.block_input,
                c.privacy_mode,
            ],
            in_voice_call: c.in_voice_call,
            incoming_voice_call: c.incoming_voice_call,
            authorized_at: if c.authorized { Some(Instant::now()) } else { None },
            ended_at: None,
        }
    }

    fn remote(&self) -> bool {
        !self.file_transfer && !self.camera && !self.terminal && self.port_forward.is_empty()
    }

    fn kind(&self) -> Option<String> {
        if self.file_transfer {
            Some(tr("Transfer file"))
        } else if self.camera {
            Some(tr("View camera"))
        } else if self.terminal {
            Some(tr("Terminal"))
        } else if !self.port_forward.is_empty() {
            Some(format!("Port Forward: {}", self.port_forward))
        } else {
            None
        }
    }

    /// Indeksi u PERMS koje ova veza prikazuje.
    fn perm_list(&self, privacy: bool) -> Vec<usize> {
        if self.camera {
            return vec![2, 5];
        }
        let mut v = vec![0, 1, 2, 3, 4, 5, 6];
        if privacy {
            v.push(7);
        }
        v
    }

    fn duration(&self) -> String {
        let secs = match self.authorized_at {
            Some(t) => self.ended_at.unwrap_or_else(Instant::now).duration_since(t).as_secs(),
            None => 0,
        };
        format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
    }
}

struct Poruka {
    od_mene: bool,
    tekst: String,
}

#[derive(Default)]
struct Podaci {
    peers: Vec<Peer>,
    chat: HashMap<i32, Vec<Poruka>>,
    selected: Option<i32>,
    show_elevation: bool,
    /// Stigla je poruka: otvori dopisivanje za tu vezu.
    otvori_chat: Option<i32>,
}

static PODACI: Mutex<Option<Podaci>> = Mutex::new(None);
static HWND_CM: AtomicIsize = AtomicIsize::new(0);
static EDIT_PROC: AtomicIsize = AtomicIsize::new(0);

fn podaci<R>(f: impl FnOnce(&mut Podaci) -> R) -> R {
    let mut g = PODACI.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Podaci::default))
}

fn javi() {
    let hwnd = HWND_CM.load(Ordering::SeqCst);
    if hwnd != 0 {
        unsafe {
            PostMessageW(hwnd as HWND, WM_PROMJENA, 0, 0);
        }
    }
}

#[derive(Clone)]
struct Handler;

impl InvokeUiCM for Handler {
    fn add_connection(&self, client: &Client) {
        log::info!(
            "lagani cm: veza {} od {} ({}), odobrena: {}",
            client.id,
            client.name,
            client.peer_id,
            client.authorized
        );
        podaci(|d| {
            d.peers.retain(|p| p.id != client.id);
            d.peers.push(Peer::from(client));
            d.selected = Some(client.id);
        });
        let hwnd = HWND_CM.load(Ordering::SeqCst);
        if hwnd != 0 {
            unsafe {
                win32::bring_to_front(hwnd as HWND);
            }
        }
        javi();
    }

    fn remove_connection(&self, id: i32, close: bool) {
        log::info!("lagani cm: veza {} završena (close: {})", id, close);
        // Kao ServerModel.onClientRemove: zatvorena veza nestaje, odspojena ostaje s gumbom „Zatvori”.
        let empty = podaci(|d| {
            if close {
                d.peers.retain(|p| p.id != id);
                d.chat.remove(&id);
            } else if let Some(p) = d.peers.iter_mut().find(|p| p.id == id) {
                p.disconnected = true;
                p.ended_at = Some(Instant::now());
            }
            d.peers.is_empty()
        });
        if empty {
            log::info!("lagani cm: nema više veza, izlaz");
            crate::platform::quit_gui();
        }
        javi();
    }

    fn new_message(&self, id: i32, text: String) {
        podaci(|d| {
            d.chat.entry(id).or_default().push(Poruka {
                od_mene: false,
                tekst: text,
            });
            d.selected = Some(id);
            d.otvori_chat = Some(id);
        });
        javi();
    }

    fn change_theme(&self, dark: String) {
        tema::nametni(&dark);
        let hwnd = HWND_CM.load(Ordering::SeqCst);
        if hwnd != 0 {
            unsafe {
                PostMessageW(hwnd as HWND, WM_OSVJEZI, 0, 0);
            }
        }
    }

    fn change_language(&self) {
        javi();
    }

    fn show_elevation(&self, show: bool) {
        podaci(|d| d.show_elevation = show);
        javi();
    }

    fn update_voice_call_state(&self, client: &Client) {
        podaci(|d| {
            if let Some(p) = d.peers.iter_mut().find(|p| p.id == client.id) {
                p.in_voice_call = client.in_voice_call;
                p.incoming_voice_call = client.incoming_voice_call;
            }
        });
        javi();
    }

    fn file_transfer_log(&self, _action: &str, _log: &str) {}
}

#[derive(Clone, Copy, PartialEq)]
enum A {
    Tab(i32),
    Chat,
    ChatClose,
    Send,
    Perm(usize),
    Accept,
    AcceptElevate,
    Elevate,
    Cancel,
    Disconnect,
    Close,
    VoiceAccept,
    VoiceDismiss,
    VoiceStop,
}

struct Stanje {
    w: Prozor<A>,
    chat_open: bool,
    scroll: f32,
    edit: HWND,
    edit_brush: HBRUSH,
    /// Visina klijentskog dijela: CM_H, uz kartice CM_H + TABS_H.
    visina: i32,
    privacy: bool,
    can_modify: bool,
    hide_elevate: bool,
    show_accept: bool,
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
    let cm = ConnectionManager { ui_handler: Handler };
    std::thread::spawn(move || ui_cm_interface::start_ipc(cm));
    let hwnd = unsafe {
        win32::create_window(&WindowSpec {
            class: CLASS,
            title: izgled::NAZIV,
            pos: Pos::TopRight,
            owner: null_mut(),
            width: CM_W as i32,
            height: CM_H,
            scale: win32::scale(),
            background: tema::pal().bg.0,
            topmost: true,
            wndproc: Some(wndproc),
        })
    };
    if hwnd.is_null() {
        log::error!("lagani cm: prozor nije stvoren");
        return;
    }
    HWND_CM.store(hwnd as isize, Ordering::SeqCst);
    log::info!(
        "lagani cm: prozor prikazan {} ms nakon pokretanja procesa",
        win32::ms_since_process_start()
    );
    unsafe {
        win32::redraw(hwnd);
        win32::message_loop();
    }
}

/// Veza koja se prikazuje: odabrana, inače prva neodobrena, inače zadnja.
fn shown(d: &Podaci) -> Option<Peer> {
    d.selected
        .and_then(|id| d.peers.iter().find(|p| p.id == id))
        .or_else(|| d.peers.iter().find(|p| !p.authorized))
        .or_else(|| d.peers.last())
        .cloned()
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            on_create(hwnd);
            0
        }
        WM_TIMER => {
            // Trajanje veze u zaglavlju.
            win32::redraw(hwnd);
            0
        }
        WM_PROMJENA => {
            prilagodi_visinu(hwnd);
            if let Some(id) = podaci(|d| d.otvori_chat.take()) {
                log::info!("lagani cm: poruka od veze {}", id);
                otvori_chat(hwnd, true);
            }
            win32::redraw(hwnd);
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
            let delta = GET_WHEEL_DELTA_WPARAM(wparam) as f32 / 120.0 * 40.0;
            with_ui(|s| s.scroll = (s.scroll + delta).max(0.0));
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
        WM_CTLCOLOREDIT => {
            let hdc = wparam as winapi::shared::windef::HDC;
            with_ui(|s| {
                let p = s.w.pal;
                SetTextColor(hdc, colorref(p.text));
                SetBkColor(hdc, colorref(p.chip));
                s.edit_brush as LRESULT
            })
            .unwrap_or(0)
        }
        WM_POSALJI => {
            posalji(hwnd);
            0
        }
        WM_SETTINGCHANGE | WM_OSVJEZI => {
            with_ui(|s| {
                s.w.osvjezi_temu(hwnd);
                DeleteObject(s.edit_brush as _);
                s.edit_brush = CreateSolidBrush(colorref(s.w.pal.chip));
                win32::redraw(s.edit);
            });
            0
        }
        WM_CLOSE => {
            // Kao Flutter verzija: zatvaranje prozora prekida sve veze, uz potvrdu ako je neka aktivna.
            let active: Vec<i32> = podaci(|d| d.peers.iter().filter(|p| !p.disconnected).map(|p| p.id).collect());
            if !active.is_empty() && !win32::ask(hwnd, &tr("Disconnect all devices?")) {
                return 0;
            }
            for id in active {
                ui_cm_interface::close(id);
            }
            log::info!("lagani cm: korisnik zatvorio prozor, izlaz");
            crate::platform::quit_gui();
            0
        }
        WM_DESTROY => {
            KillTimer(hwnd, TIMER_SEC);
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

fn colorref(c: Color) -> u32 {
    RGB(((c.0 >> 16) & 0xFF) as u8, ((c.0 >> 8) & 0xFF) as u8, (c.0 & 0xFF) as u8)
}

unsafe fn on_create(hwnd: HWND) {
    let w = match Prozor::new(hwnd) {
        Ok(w) => w,
        Err(e) => {
            log::error!("lagani cm: Direct2D nije dostupan: {}", e);
            return;
        }
    };
    let edit = stvori_polje(hwnd, w.gfx.scale());
    let edit_brush = CreateSolidBrush(colorref(w.pal.chip));
    let s = Stanje {
        w,
        chat_open: false,
        scroll: 0.0,
        edit,
        edit_brush,
        visina: CM_H,
        privacy: !crate::privacy_mode::get_supported_privacy_mode_impl().is_empty(),
        can_modify: crate::ui_interface::get_builtin_option("enable-perm-change-in-accept-window") != "N",
        hide_elevate: crate::ui_interface::get_builtin_option("hide-elevate-button-in-accept-window") == "Y",
        show_accept: crate::ui_interface::get_option("approve-mode") != "password",
    };
    UI.with(|c| {
        if let Ok(mut g) = c.try_borrow_mut() {
            *g = Some(s);
        }
    });
    SetTimer(hwnd, TIMER_SEC, 1000, None);
}

/// Polje za poruku (GDI EDIT kontrola s fontom Manrope), skriveno dok dopisivanje nije otvoreno.
unsafe fn stvori_polje(hwnd: HWND, scale: f32) -> HWND {
    let data = super::d2d::manrope_medium();
    let mut count: u32 = 0;
    AddFontMemResourceEx(data.as_ptr() as *mut _, data.len() as u32, null_mut(), &mut count);
    let face = win32::wide("Manrope");
    let font: HFONT = CreateFontW(
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
    let class = win32::wide("EDIT");
    let edit = CreateWindowExW(
        0,
        class.as_ptr(),
        null(),
        WS_CHILD | WS_TABSTOP | ES_AUTOHSCROLL,
        0,
        0,
        10,
        10,
        hwnd,
        ID_EDIT as HMENU,
        win32::hinstance(),
        null_mut(),
    );
    if edit.is_null() {
        log::error!("lagani cm: polje za poruku nije stvoreno");
        return edit;
    }
    SendMessageW(edit, WM_SETFONT, font as WPARAM, 1);
    let cue = win32::wide(&tr("Write a message"));
    SendMessageW(edit, EM_SETCUEBANNER, 1, cue.as_ptr() as LPARAM);
    let old = SetWindowLongPtrW(edit, GWLP_WNDPROC, edit_proc as usize as isize);
    EDIT_PROC.store(old, Ordering::SeqCst);
    edit
}

unsafe extern "system" fn edit_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_KEYDOWN && wparam == VK_RETURN as WPARAM {
        let parent = HWND_CM.load(Ordering::SeqCst);
        PostMessageW(parent as HWND, WM_POSALJI, 0, 0);
        return 0;
    }
    if msg == WM_CHAR && wparam == '\r' as WPARAM {
        return 0;
    }
    let old = EDIT_PROC.load(Ordering::SeqCst);
    CallWindowProcW(std::mem::transmute(old), hwnd, msg, wparam, lparam)
}

unsafe fn prilagodi_visinu(hwnd: HWND) {
    let need = if podaci(|d| d.peers.len()) > 1 { CM_H + TABS_H } else { CM_H };
    let Some((changed, chat_open, scale)) = with_ui(|s| {
        let changed = s.visina != need;
        s.visina = need;
        (changed, s.chat_open, s.w.gfx.scale())
    }) else {
        return;
    };
    if changed {
        let width = if chat_open { CM_W + CHAT_W } else { CM_W };
        win32::set_client_size(hwnd, width as i32, need, scale, true);
        polozaj_polja(hwnd);
    }
}

/// Polje za poruku uz donji rub dopisivanja.
unsafe fn polozaj_polja(_hwnd: HWND) {
    if let Some((edit, scale, h)) = with_ui(|s| (s.edit, s.w.gfx.scale(), s.visina as f32)) {
        let px = |v: f32| (v * scale).round() as i32;
        MoveWindow(edit, px(26.0), px(h - 43.0), px(CHAT_W - 26.0 - 64.0), px(18.0), 1);
    }
}

/// Otvara ili zatvara dopisivanje: prozor se širi ulijevo, desni rub ostaje (kao Flutter, 300 → 700).
unsafe fn otvori_chat(hwnd: HWND, open: bool) {
    // Kao toggleCMSidePage (windowManager.show): smanjeni prozor se prvo vraća, pa tek onda širi.
    if open && IsIconic(hwnd) != 0 {
        ShowWindow(hwnd, SW_RESTORE);
    }
    let Some((changed, edit, scale, visina)) = with_ui(|s| {
        let changed = s.chat_open != open;
        s.chat_open = open;
        s.scroll = 0.0;
        (changed, s.edit, s.w.gfx.scale(), s.visina)
    }) else {
        return;
    };
    if changed {
        let width = if open { CM_W + CHAT_W } else { CM_W };
        win32::set_client_size(hwnd, width as i32, visina, scale, true);
    }
    if open {
        polozaj_polja(hwnd);
        ShowWindow(edit, SW_SHOW);
        SetFocus(edit);
        if GetForegroundWindow() != hwnd {
            let mut fi = FLASHWINFO {
                cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                hwnd,
                dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
                uCount: 0,
                dwTimeout: 0,
            };
            FlashWindowEx(&mut fi);
        }
    } else {
        ShowWindow(edit, SW_HIDE);
    }
    win32::redraw(hwnd);
}

unsafe fn posalji(hwnd: HWND) {
    let Some(edit) = with_ui(|s| s.edit) else {
        return;
    };
    let len = GetWindowTextLengthW(edit);
    if len <= 0 {
        return;
    }
    let mut buf = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(edit, buf.as_mut_ptr(), buf.len() as i32);
    let text = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
    if text.trim().is_empty() {
        return;
    }
    let Some(peer) = podaci(|d| shown(d)) else {
        return;
    };
    ui_cm_interface::send_chat(peer.id, text.clone());
    podaci(|d| {
        d.chat.entry(peer.id).or_default().push(Poruka {
            od_mene: true,
            tekst: text,
        })
    });
    win32::set_title(edit, "");
    with_ui(|s| s.scroll = 0.0);
    win32::redraw(hwnd);
}

unsafe fn on_action(hwnd: HWND, a: A) {
    let Some(p) = podaci(|d| shown(d)) else {
        return;
    };
    match a {
        A::Tab(id) => {
            podaci(|d| d.selected = Some(id));
            with_ui(|s| s.scroll = 0.0);
        }
        A::Chat => {
            let open = with_ui(|s| s.chat_open).unwrap_or(false);
            otvori_chat(hwnd, !open);
        }
        A::ChatClose => otvori_chat(hwnd, false),
        A::Send => posalji(hwnd),
        A::Perm(i) => {
            if p.disconnected {
                return;
            }
            let on = !p.perms[i];
            log::info!("lagani cm: dozvola {} = {} za vezu {}", PERMS[i].0, on, p.id);
            ui_cm_interface::switch_permission(p.id, PERMS[i].0.to_owned(), on);
            podaci(|d| {
                if let Some(q) = d.peers.iter_mut().find(|q| q.id == p.id) {
                    q.perms[i] = on;
                }
            });
        }
        A::Accept | A::AcceptElevate => {
            log::info!("lagani cm: korisnik prihvatio {} ({})", p.name, p.peer_id);
            ui_cm_interface::authorize(p.id);
            podaci(|d| {
                if let Some(q) = d.peers.iter_mut().find(|q| q.id == p.id) {
                    q.authorized = true;
                    q.authorized_at = Some(Instant::now());
                }
            });
            if a == A::AcceptElevate {
                log::info!("lagani cm: povišenje prava za vezu {}", p.id);
                ui_cm_interface::elevate_portable(p.id);
            }
            ShowWindow(hwnd, SW_MINIMIZE);
        }
        A::Elevate => {
            log::info!("lagani cm: povišenje prava za vezu {}", p.id);
            ui_cm_interface::elevate_portable(p.id);
            ShowWindow(hwnd, SW_MINIMIZE);
        }
        A::Cancel | A::Disconnect => {
            log::info!("lagani cm: korisnik prekinuo/odbio {} ({})", p.name, p.peer_id);
            ui_cm_interface::close(p.id);
        }
        A::Close => {
            ui_cm_interface::remove(p.id);
            let empty = podaci(|d| {
                d.peers.retain(|q| q.id != p.id);
                d.chat.remove(&p.id);
                d.peers.is_empty()
            });
            if empty {
                crate::platform::quit_gui();
            }
        }
        A::VoiceAccept => ui_cm_interface::handle_incoming_voice_call(p.id, true),
        A::VoiceDismiss => ui_cm_interface::handle_incoming_voice_call(p.id, false),
        A::VoiceStop => ui_cm_interface::close_voice_call(p.id),
    }
    win32::redraw(hwnd);
}

struct Slika {
    peers: Vec<(i32, String)>,
    peer: Option<Peer>,
    chat: Vec<(bool, String)>,
    show_elevation: bool,
}

fn crtaj(s: &mut Stanje) {
    let slika = podaci(|d| {
        let peer = shown(d);
        let chat = peer
            .as_ref()
            .and_then(|p| d.chat.get(&p.id))
            .map(|v| v.iter().map(|m| (m.od_mene, m.tekst.clone())).collect())
            .unwrap_or_default();
        Slika {
            peers: d.peers.iter().map(|p| (p.id, p.name.clone())).collect(),
            peer,
            chat,
            show_elevation: d.show_elevation,
        }
    });
    let chat_open = s.chat_open;
    let h = s.visina as f32;
    let (privacy, can_modify, hide_elevate, show_accept) = (s.privacy, s.can_modify, s.hide_elevate, s.show_accept);
    let mut scroll = s.scroll;
    s.w.crtaj(|p| {
        let Some(peer) = &slika.peer else {
            return;
        };
        let x0 = if chat_open { CHAT_W } else { 0.0 };
        if chat_open {
            scroll = crtaj_chat(p, peer, &slika.chat, scroll, h);
        }
        let mut y = 12.0;
        if slika.peers.len() > 1 {
            y = crtaj_kartice(p, x0, y, &slika.peers, peer.id);
        }
        y = crtaj_zaglavlje(p, x0, y, peer);
        if !peer.authorized && !peer.disconnected {
            y = crtaj_uputu(p, x0, y);
        }
        let gumbi_top = crtaj_gumbe(p, x0, h, peer, slika.show_elevation, hide_elevate, show_accept);
        let show_perms = !(peer.file_transfer || !peer.port_forward.is_empty() || peer.terminal || peer.disconnected);
        if show_perms {
            crtaj_dozvole(p, x0, y, gumbi_top - 8.0, peer, privacy, can_modify);
        }
    });
    s.scroll = scroll;
}

fn crtaj_kartice(p: &mut Ploha<A>, x0: f32, y: f32, peers: &[(i32, String)], selected: i32) -> f32 {
    let pal = p.p;
    let f = p.f;
    let mut x = x0 + 12.0;
    for (id, name) in peers {
        let label = if name.trim().is_empty() { format!("#{}", id) } else { name.trim().to_owned() };
        let w = (p.width(&label, &f.btn) + 24.0).min(140.0);
        let r = rect(x, y, x + w, y + 28.0);
        if *id == selected {
            p.c.fill_round(&r, 8.0, pal.card);
            p.c.stroke_round(&r, 8.0, pal.border, 1.0);
        } else if p.is_hover(A::Tab(*id)) {
            p.c.fill_round(&r, 8.0, pal.chip);
        }
        let color = if *id == selected { pal.text } else { pal.muted };
        p.text(&label, &f.btn, &r, color);
        p.hit(r, A::Tab(*id));
        x += w + 6.0;
        if x > x0 + CM_W - 40.0 {
            break;
        }
    }
    y + 36.0
}

fn crtaj_zaglavlje(p: &mut Ploha<A>, x0: f32, y: f32, peer: &Peer) -> f32 {
    let pal = p.p;
    let f = p.f;
    let card = rect(x0 + 12.0, y, x0 + CM_W - 12.0, y + 96.0);
    p.c.fill_round(&card, 12.0, pal.card);
    p.c.stroke_round(&card, 12.0, pal.border, 1.0);

    let av = rect(card.left + 12.0, y + 12.0, card.left + 56.0, y + 56.0);
    p.c.fill_round(&av, 10.0, pal.primary);
    let initial = peer
        .name
        .trim()
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_owned());
    p.text(&initial, &f.avatar, &av, pal.on_primary);

    let side_page = peer.authorized && !peer.disconnected && (peer.remote() || peer.camera);
    let right = if side_page { card.right - 48.0 } else { card.right - 12.0 };
    let tx = av.right + 10.0;
    p.text(&peer.name, &f.name, &rect(tx, y + 10.0, right, y + 30.0), pal.text);
    p.text(
        &format!("ID {}", format_id(&peer.peer_id)),
        &f.muted_line,
        &rect(tx, y + 30.0, right, y + 46.0),
        pal.muted,
    );
    if let Some(kind) = peer.kind() {
        p.text(&kind, &f.muted_line, &rect(tx, y + 46.0, right, y + 62.0), pal.muted);
    }
    if side_page {
        p.ikona_gumb(
            rect(card.right - 44.0, y + 16.0, card.right - 12.0, y + 48.0),
            A::Chat,
            ikona::CHAT,
            pal.text,
        );
    }

    let (color, text) = if !peer.authorized {
        (pal.warn, tr("Request access to your device"))
    } else if peer.disconnected {
        (pal.muted, tr("Disconnected"))
    } else {
        (pal.ok, tr("Connected"))
    };
    let sy = y + 70.0;
    p.c.circle(card.left + 16.0, sy + 9.0, 4.0, color);
    let tw = p.width(&text, &f.btn_l);
    let tx = card.left + 28.0;
    p.text(&text, &f.btn_l, &rect(tx, sy, card.right - 12.0, sy + 18.0), color);
    if peer.authorized {
        p.text(
            &format!("  ·  {}", peer.duration()),
            &f.body,
            &rect(tx + tw, sy, card.right - 12.0, sy + 18.0),
            pal.muted,
        );
    }
    y + 104.0
}

fn crtaj_uputu(p: &mut Ploha<A>, x0: f32, y: f32) -> f32 {
    let pal = p.p;
    let f = p.f;
    let text = tr("wips_approve_tip");
    let tx = x0 + 44.0;
    let tw = CM_W - 24.0 - 44.0;
    let (_, th) = p.c.measure(&text, &f.tip_l, tw);
    let r = rect(x0 + 12.0, y, x0 + CM_W - 12.0, y + th + 16.0);
    p.c.fill_round(&r, 8.0, pal.chip);
    p.c.icon(ikona::SHIELD, &f.icon16, &rect(r.left + 8.0, y + 8.0, r.left + 28.0, y + 26.0), pal.muted);
    p.text(&text, &f.tip_l, &rect(tx, y + 8.0, r.right - 10.0, y + 8.0 + th), pal.muted);
    r.bottom + 10.0
}

fn crtaj_dozvole(p: &mut Ploha<A>, x0: f32, y: f32, bottom: f32, peer: &Peer, privacy: bool, can_modify: bool) {
    let pal = p.p;
    let f = p.f;
    p.text(
        &tr("Permissions").to_uppercase(),
        &f.section,
        &rect(x0 + 16.0, y, x0 + CM_W - 16.0, y + 20.0),
        pal.muted,
    );
    let mut ry = y + 22.0;
    for i in peer.perm_list(privacy) {
        if ry + 30.0 > bottom {
            break;
        }
        let (_, label, glyph) = PERMS[i];
        let cy = ry + 15.0;
        p.c.icon(glyph, &f.icon16, &rect(x0 + 14.0, ry, x0 + 36.0, ry + 30.0), pal.muted);
        p.text(&tr(label), &f.body_line, &rect(x0 + 44.0, ry, x0 + CM_W - 60.0, ry + 30.0), pal.text);
        let action = if can_modify && !peer.disconnected { Some(A::Perm(i)) } else { None };
        p.prekidac(x0 + CM_W - 48.0, cy, peer.perms[i], action);
        ry += 30.0;
    }
}

/// Gumbi uz donji rub, po stanju veze kao _CmControlPanel; vraća vrh prvog reda.
fn crtaj_gumbe(
    p: &mut Ploha<A>,
    x0: f32,
    h: f32,
    peer: &Peer,
    show_elevation: bool,
    hide_elevate: bool,
    show_accept: bool,
) -> f32 {
    let can_elevate = ui_cm_interface::can_elevate() && show_elevation && peer.remote();
    let mut rows: Vec<Vec<(A, Vrsta, String, Option<char>)>> = Vec::new();
    if peer.in_voice_call {
        rows.push(vec![(A::VoiceStop, Vrsta::Opasno, tr("Stop voice call"), Some(ikona::PHONE))]);
    }
    if peer.incoming_voice_call {
        rows.push(vec![
            (A::VoiceAccept, Vrsta::Primarni, tr("Accept"), Some(ikona::PHONE)),
            (A::VoiceDismiss, Vrsta::Opasno, tr("Dismiss"), None),
        ]);
    }
    if peer.disconnected {
        rows.push(vec![(A::Close, Vrsta::Primarni, tr("Close"), None)]);
    } else if peer.authorized {
        if can_elevate {
            rows.push(vec![(A::Elevate, Vrsta::Primarni, tr("Elevate"), Some(ikona::SHIELD))]);
        }
        rows.push(vec![(A::Disconnect, Vrsta::Opasno, tr("Disconnect"), Some(ikona::DISCONNECT))]);
    } else {
        if can_elevate && show_accept && !hide_elevate {
            rows.push(vec![(
                A::AcceptElevate,
                Vrsta::Primarni,
                tr("Accept and Elevate"),
                Some(ikona::SHIELD),
            )]);
        }
        let mut row = Vec::new();
        if show_accept {
            row.push((A::Accept, Vrsta::Primarni, tr("Accept"), None));
        }
        row.push((A::Cancel, Vrsta::Obrub, tr("Cancel"), None));
        rows.push(row);
    }
    let row_h = 36.0;
    let gap = 6.0;
    let top = h - 12.0 - rows.len() as f32 * (row_h + gap) + gap;
    let mut y = top;
    for row in rows {
        let n = row.len() as f32;
        let w = (CM_W - 24.0 - gap * (n - 1.0)) / n;
        for (i, (a, vrsta, label, icon)) in row.into_iter().enumerate() {
            let x = x0 + 12.0 + i as f32 * (w + gap);
            p.gumb(rect(x, y, x + w, y + row_h), a, vrsta, &label, icon, false);
        }
        y += row_h + gap;
    }
    top
}

/// Dopisivanje s lijeve strane prozora; vraća ograničeni pomak (kotačić miša).
fn crtaj_chat(p: &mut Ploha<A>, peer: &Peer, poruke: &[(bool, String)], scroll: f32, h: f32) -> f32 {
    let pal: Pal = p.p;
    let f = p.f;
    let panel = rect(0.0, 0.0, CHAT_W, h);
    p.c.fill_round(&panel, 0.0, pal.card);
    p.c.line(CHAT_W - 0.5, 0.0, CHAT_W - 0.5, h, pal.border, 1.0);

    let title = format!("{}  ·  {}", tr("Chat"), peer.name.trim());
    p.text(&title, &f.name, &rect(16.0, 12.0, CHAT_W - 52.0, 40.0), pal.text);
    p.ikona_gumb(rect(CHAT_W - 44.0, 10.0, CHAT_W - 12.0, 42.0), A::ChatClose, ikona::DISCONNECT, pal.muted);
    p.c.line(0.0, 52.0, CHAT_W, 52.0, pal.border, 1.0);

    let area = rect(12.0, 60.0, CHAT_W - 12.0, h - 64.0);
    let max_w = 240.0;
    let sizes: Vec<(f32, f32)> = poruke
        .iter()
        .map(|(_, t)| {
            let (w, th) = p.c.measure(t, &f.chat, max_w);
            (w.ceil() + 24.0, th.ceil() + 16.0)
        })
        .collect();
    let total: f32 = sizes.iter().map(|(_, bh)| bh + 8.0).sum();
    let scroll = scroll.min((total - (area.bottom - area.top)).max(0.0));
    p.c.push_clip(&area);
    let mut y = area.bottom + scroll;
    for ((mine, text), (bw, bh)) in poruke.iter().zip(sizes.iter()).rev() {
        y -= bh;
        if y + bh < area.top {
            break;
        }
        if y < area.bottom {
            let r = if *mine {
                rect(area.right - bw, y, area.right, y + bh)
            } else {
                rect(area.left, y, area.left + bw, y + bh)
            };
            let (bg, fg) = if *mine { (pal.primary, pal.on_primary) } else { (pal.chip, pal.text) };
            p.c.fill_round(&r, 10.0, bg);
            p.text(text, &f.chat, &rect(r.left + 12.0, r.top + 8.0, r.right - 10.0, r.bottom), fg);
        }
        y -= 8.0;
    }
    p.c.pop_clip();

    let input = rect(12.0, h - 52.0, CHAT_W - 56.0, h - 16.0);
    p.c.fill_round(&input, 8.0, pal.chip);
    p.c.stroke_round(&input, 8.0, pal.border, 1.0);
    p.ikona_gumb(rect(CHAT_W - 50.0, h - 52.0, CHAT_W - 14.0, h - 16.0), A::Send, ikona::SEND, pal.text);
    scroll
}
