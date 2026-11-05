// Zajednički izgled prozora laganog sučelja, smjer C „Svijetli Fluent” (kao wips_incoming_home.dart i
// wips_cm_card.dart): fontovi, ikone i stalni podaci. Raspored svakog prozora je u njegovom modulu.
use windows::core::Result;
use windows::Win32::Graphics::DirectWrite::IDWriteTextFormat;

use super::d2d::{Align, Gfx};

pub const PHONE: &str = "01/61 76 166";
pub const WEB: &str = "https://www.wips.hr";
/// Izvorni kod (AGPL-3.0); oznake @@WIPS_*@@ se u wips/ ne zamjenjuju, pa je adresa ovdje upisana.
pub const SOURCE_URL: &str = "https://github.com/ninoseric/wips-podrska-client";
pub const LICENSE_URL: &str = "https://www.gnu.org/licenses/agpl-3.0.html";
pub const NAZIV: &str = "WiPS udaljena podrška";

/// Ikone iz fonta Segoe MDL2 Assets.
pub mod ikona {
    pub const SETTINGS: char = '\u{E713}';
    pub const CHAT: char = '\u{E8BD}';
    pub const SHIELD: char = '\u{EA18}';
    pub const INFO: char = '\u{E946}';
    pub const COPY: char = '\u{E8C8}';
    pub const KEYBOARD: char = '\u{E765}';
    pub const CLIPBOARD: char = '\u{E77F}';
    pub const AUDIO: char = '\u{E767}';
    pub const FILE: char = '\u{E898}';
    pub const RESTART: char = '\u{E777}';
    pub const RECORD: char = '\u{E714}';
    pub const BLOCK: char = '\u{E733}';
    pub const PRIVACY: char = '\u{ED1A}';
    pub const SEND: char = '\u{E724}';
    pub const CHECK: char = '\u{E73E}';
    pub const DISCONNECT: char = '\u{E711}';
    pub const PHONE: char = '\u{E717}';
    pub const FOLDER: char = '\u{E838}';
    pub const CHEVRON: char = '\u{E70D}';
    pub const LOCK: char = '\u{E72E}';
}

pub struct Fonts {
    pub title: IDWriteTextFormat,
    /// Naslov koji ne stane u jedan redak (npr. „Podrška trenutno nije dostupna”).
    pub title_sm: IDWriteTextFormat,
    pub status: IDWriteTextFormat,
    pub label: IDWriteTextFormat,
    pub id: IDWriteTextFormat,
    pub button: IDWriteTextFormat,
    pub button_l: IDWriteTextFormat,
    pub btn: IDWriteTextFormat,
    pub btn_l: IDWriteTextFormat,
    pub tip: IDWriteTextFormat,
    pub tip_l: IDWriteTextFormat,
    pub small: IDWriteTextFormat,
    pub body: IDWriteTextFormat,
    pub section: IDWriteTextFormat,
    pub h1: IDWriteTextFormat,
    pub name: IDWriteTextFormat,
    pub muted_line: IDWriteTextFormat,
    pub body_line: IDWriteTextFormat,
    pub avatar: IDWriteTextFormat,
    pub chat: IDWriteTextFormat,
    pub icon14: IDWriteTextFormat,
    pub icon16: IDWriteTextFormat,
    pub icon20: IDWriteTextFormat,
}

impl Fonts {
    pub fn new(g: &Gfx) -> Result<Fonts> {
        Ok(Fonts {
            title: g.font(800, 22.0, Align::Center)?,
            title_sm: g.font(800, 18.0, Align::Center)?,
            status: g.font_vcenter(600, 13.0, Align::Left)?,
            label: g.font(600, 12.5, Align::Left)?,
            id: g.font_vcenter(700, 24.0, Align::Left)?,
            button: g.font_vcenter(700, 14.0, Align::Center)?,
            button_l: g.font_vcenter(700, 14.0, Align::Left)?,
            btn: g.font_vcenter(600, 13.0, Align::Center)?,
            btn_l: g.font_vcenter(600, 13.0, Align::Left)?,
            tip: g.font(500, 13.0, Align::Center)?,
            tip_l: g.font(500, 12.5, Align::Left)?,
            small: g.font_vcenter(500, 12.0, Align::Left)?,
            body: g.font_vcenter(500, 13.0, Align::Left)?,
            section: g.font_vcenter(700, 11.0, Align::Left)?,
            h1: g.font_vcenter(800, 18.0, Align::Left)?,
            name: g.font_line(700, 15.0, Align::Left)?,
            muted_line: g.font_line(500, 12.0, Align::Left)?,
            body_line: g.font_line(500, 13.0, Align::Left)?,
            avatar: g.font_vcenter(700, 20.0, Align::Center)?,
            chat: g.font(500, 13.0, Align::Left)?,
            icon14: g.icons(14.0)?,
            icon16: g.icons(16.0)?,
            icon20: g.icons(20.0)?,
        })
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
