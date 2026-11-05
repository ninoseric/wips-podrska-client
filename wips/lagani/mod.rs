// WiPS: lagano sučelje korisničke verzije umjesto Sciter/Flutter sučelja.
// Gradi se s `--features lagani` (bez `flutter`); u stablo ga uključuje patch app/0006 preko
// `#[path]` u src/lib.rs, a poziva ga src/main.rs umjesto `ui::start`.
// Jezgra (server, IPC, odobravanje veze, instalacija, snimanje ekrana) ostaje RustDeskova: ovdje su prozori.
mod cm;
mod d2d;
mod dijalog;
mod glavni;
mod instalacija;
mod izgled;
mod postavke;
mod sustav;
mod tema;
mod ui;
mod win32;

use hbb_common::log;

/// Prijevod istim ključevima i datotekama (src/lang) kao Flutter verzija; jezik prema opciji "lang" ili sustavu.
pub(crate) fn tr(key: &str) -> String {
    crate::lang::translate(key.to_owned())
}

pub fn start(args: &mut [String]) {
    log::info!("lagani: start, args {:?}", args);
    if args.is_empty() {
        glavni::run();
    } else if args[0] == "--cm" {
        cm::run();
    } else if args[0] == "--install" {
        instalacija::run();
    } else {
        // --connect i ostalo korisnička verzija nema; pokaži početni prozor.
        log::warn!("lagani: nepodržani argumenti {:?}, otvaram početni prozor", args);
        glavni::run();
    }
}
