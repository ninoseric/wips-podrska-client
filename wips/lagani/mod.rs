// WiPS: lagano sučelje korisničke verzije (prototip) umjesto Sciter/Flutter sučelja.
// Gradi se s `--features lagani` (bez `flutter`); u stablo ga uključuje patch app/0006 preko
// `#[path]` u src/lib.rs, a poziva ga src/main.rs umjesto `ui::start`.
// Jezgra (server, IPC, odobravanje veze, snimanje ekrana) ostaje RustDeskova: ovdje su samo prozori.
mod cm;
mod glavni;
mod win32;

use hbb_common::log;

pub fn start(args: &mut [String]) {
    log::info!("lagani: start, args {:?}", args);
    if args.is_empty() {
        glavni::run();
    } else if args[0] == "--cm" {
        cm::run();
    } else {
        // --install, --connect i ostalo korisnička verzija nema; pokaži početni prozor.
        log::warn!("lagani: nepodržani argumenti {:?}, otvaram početni prozor", args);
        glavni::run();
    }
}
