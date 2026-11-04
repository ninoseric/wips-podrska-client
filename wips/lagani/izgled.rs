// Izgled prozora laganog sučelja, smjer C „Svijetli Fluent” (kao wips_incoming_home.dart i wips_cm_card.dart):
// raspored, boje i tekstovi. Crta po platnu iz d2d.rs; stanje dobiva kao obične vrijednosti, pa se isti kod
// koristi u programu i u probnom prozoru bez RustDesk jezgre. Mjere su u DIP-ovima.
use windows::core::Result;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::DirectWrite::IDWriteTextFormat;

use super::d2d::{inside, rect, Align, Canvas, Color, Gfx};

pub const PHONE: &str = "01/61 76 166";
pub const GLAVNI_W: i32 = 340;
pub const GLAVNI_H: i32 = 456;
pub const CM_W: i32 = 380;
pub const CM_H: i32 = 236;

pub const BG: Color = Color(0xFAFAFB);
pub const CARD: Color = Color(0xFFFFFF);
pub const BORDER: Color = Color(0xE5E7EB);
pub const CHIP: Color = Color(0xF3F4F6);
pub const TEXT: Color = Color(0x111827);
pub const MUTED: Color = Color(0x4B5563);
pub const PRIMARY: Color = Color(0x1F2937);
pub const PRIMARY_HOVER: Color = Color(0x374151);
pub const ON_PRIMARY: Color = Color(0xFFFFFF);
pub const OK: Color = Color(0x15803D);
pub const WAIT: Color = Color(0xB45309);
pub const ERR: Color = Color(0xB91C1C);

pub struct Fonts {
    pub title: IDWriteTextFormat,
    pub status: IDWriteTextFormat,
    pub label: IDWriteTextFormat,
    pub id: IDWriteTextFormat,
    pub button: IDWriteTextFormat,
    pub tip: IDWriteTextFormat,
    pub footer: IDWriteTextFormat,
    pub cm_title: IDWriteTextFormat,
    pub cm_body: IDWriteTextFormat,
    pub cm_tip: IDWriteTextFormat,
}

impl Fonts {
    pub fn new(g: &Gfx) -> Result<Fonts> {
        Ok(Fonts {
            title: g.font(800, 22.0, Align::Center)?,
            status: g.font_vcenter(600, 13.0, Align::Center)?,
            label: g.font(600, 12.5, Align::Left)?,
            id: g.font_vcenter(700, 24.0, Align::Center)?,
            button: g.font_vcenter(700, 14.0, Align::Center)?,
            tip: g.font(500, 13.0, Align::Center)?,
            footer: g.font_vcenter(500, 12.0, Align::Center)?,
            cm_title: g.font(800, 20.0, Align::Left)?,
            cm_body: g.font(500, 14.0, Align::Left)?,
            cm_tip: g.font(500, 12.5, Align::Left)?,
        })
    }
}

/// Stanje početnog prozora koje crtanje treba.
pub struct Glavni<'a> {
    pub id: &'a str,
    /// >0 spojeno, 0 spajanje, <0 nedostupno
    pub status: i32,
    pub version: &'a str,
    pub copied: bool,
    pub hover_copy: bool,
}

/// Gumb „Kopiraj ID” (za hit-test miša).
pub fn glavni_copy_rect() -> D2D_RECT_F {
    rect(34.0, 282.0, 306.0, 326.0)
}

pub fn crtaj_glavni(c: &Canvas, f: &Fonts, s: &Glavni) {
    let w = GLAVNI_W as f32;
    c.logo(&rect(w / 2.0 - 44.0, 18.0, w / 2.0 + 44.0, 106.0));

    let (title, status_color, status_text) = if s.status > 0 {
        ("Spremno za podršku", OK, "●  Povezano sa serverom WiPS")
    } else if s.status == 0 {
        ("Povezivanje sa serverom", WAIT, "●  Pričekajte trenutak")
    } else {
        ("Podrška nije dostupna", ERR, "●  Nema veze sa serverom")
    };
    c.text(title, &f.title, &rect(16.0, 114.0, w - 16.0, 146.0), TEXT);
    c.text(status_text, &f.status, &rect(16.0, 148.0, w - 16.0, 170.0), status_color);

    let card = rect(16.0, 188.0, w - 16.0, 344.0);
    c.fill_round(&card, 12.0, CARD);
    c.stroke_round(&card, 12.0, BORDER, 1.0);

    c.text("ID", &f.label, &rect(34.0, 204.0, 200.0, 220.0), MUTED);
    let id_text = if s.id.is_empty() {
        "— — —".to_owned()
    } else {
        format_id(s.id)
    };
    let chip = rect(34.0, 224.0, 206.0, 266.0);
    c.fill_round(&chip, 8.0, CHIP);
    c.text(&id_text, &f.id, &chip, TEXT);

    let btn = glavni_copy_rect();
    c.fill_round(&btn, 10.0, if s.hover_copy { PRIMARY_HOVER } else { PRIMARY });
    let label = if s.copied { "Kopirano  ✓" } else { "Kopiraj ID" };
    c.text(label, &f.button, &btn, ON_PRIMARY);

    c.text(
        "Nazovite podršku na 01/61 76 166 i pročitajte ID tehničaru. Spajanje uvijek morate odobriti vi.",
        &f.tip,
        &rect(38.0, 358.0, w - 38.0, 414.0),
        MUTED,
    );

    let footer = format!("{}  ·  wips.hr  ·  v{}", PHONE, s.version);
    c.text(&footer, &f.footer, &rect(16.0, 422.0, w - 16.0, 442.0), MUTED);
}

/// Stanje prozora za odobravanje veze.
pub struct Cm<'a> {
    /// None = čekanje na zahtjev
    pub peer: Option<CmPeer<'a>>,
    pub hover: Option<CmButton>,
}

pub struct CmPeer<'a> {
    pub name: &'a str,
    pub peer_id: &'a str,
    pub authorized: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CmButton {
    Allow,
    Deny,
    Disconnect,
}

pub fn cm_button_rect(b: CmButton) -> D2D_RECT_F {
    match b {
        CmButton::Allow => rect(24.0, 180.0, 180.0, 220.0),
        CmButton::Deny => rect(192.0, 180.0, 300.0, 220.0),
        CmButton::Disconnect => rect(24.0, 180.0, 180.0, 220.0),
    }
}

/// Koji je gumb na točki (DIP), s obzirom na stanje.
pub fn cm_hit(s: &Cm, x: f32, y: f32) -> Option<CmButton> {
    let peer = s.peer.as_ref()?;
    if peer.authorized {
        inside(&cm_button_rect(CmButton::Disconnect), x, y).then_some(CmButton::Disconnect)
    } else if inside(&cm_button_rect(CmButton::Allow), x, y) {
        Some(CmButton::Allow)
    } else if inside(&cm_button_rect(CmButton::Deny), x, y) {
        Some(CmButton::Deny)
    } else {
        None
    }
}

pub fn crtaj_cm(c: &Canvas, f: &Fonts, s: &Cm) {
    let w = CM_W as f32;
    let (title, body, tip) = match &s.peer {
        None => (
            "Čekanje na zahtjev za spajanje…".to_owned(),
            String::new(),
            "",
        ),
        Some(p) => {
            let who = if p.name.trim().is_empty() {
                format!("Tehničar WiPS ({})", format_id(p.peer_id))
            } else {
                format!("{} ({})", p.name.trim(), format_id(p.peer_id))
            };
            if p.authorized {
                (
                    "Veza je aktivna".to_owned(),
                    format!("{} upravlja vašim računalom.", who),
                    "Tehničar vidi vaš zaslon i upravlja mišem i tipkovnicom. Vezu možete prekinuti u bilo kojem trenutku.",
                )
            } else {
                (
                    "Zahtjev za spajanje".to_owned(),
                    format!("{} traži pristup vašem računalu.", who),
                    "Odobrite samo ako ste upravo razgovarali s podrškom WiPS. Vezu možete prekinuti u bilo kojem trenutku.",
                )
            }
        }
    };
    c.text(&title, &f.cm_title, &rect(24.0, 20.0, w - 24.0, 50.0), TEXT);
    c.text(&body, &f.cm_body, &rect(24.0, 60.0, w - 24.0, 104.0), TEXT);
    c.text(tip, &f.cm_tip, &rect(24.0, 108.0, w - 24.0, 168.0), MUTED);

    let Some(p) = &s.peer else {
        return;
    };
    let hover = |b: CmButton| s.hover == Some(b);
    if p.authorized {
        let r = cm_button_rect(CmButton::Disconnect);
        c.fill_round(&r, 10.0, if hover(CmButton::Disconnect) { CHIP } else { CARD });
        c.stroke_round(&r, 10.0, BORDER, 1.0);
        c.text("Prekini vezu", &f.button, &r, TEXT);
    } else {
        let r = cm_button_rect(CmButton::Allow);
        c.fill_round(&r, 10.0, if hover(CmButton::Allow) { PRIMARY_HOVER } else { PRIMARY });
        c.text("Dopusti", &f.button, &r, ON_PRIMARY);
        let r = cm_button_rect(CmButton::Deny);
        c.fill_round(&r, 10.0, if hover(CmButton::Deny) { CHIP } else { CARD });
        c.stroke_round(&r, 10.0, BORDER, 1.0);
        c.text("Odbij", &f.button, &r, TEXT);
    }
}

/// „123456789” → „123 456 789”; ostale ID-ove ostavlja kakvi jesu.
pub fn format_id(id: &str) -> String {
    if id.len() == 9 && id.chars().all(|c| c.is_ascii_digit()) {
        format!("{} {} {}", &id[0..3], &id[3..6], &id[6..9])
    } else {
        id.to_owned()
    }
}
