# Lokalni testni okvir za lagano sučelje

Prevodi i pokreće `wips/lagani/*` (prave prozore) sa zamjenama za RustDesk jezgru (`src/main.rs`: lažni ID, stanje
„spojeno”, lažni zahtjev tehničara nakon 1,5 s). Treba samo rustup (toolchain 1.75, kao CI); vcpkg i Flutter ne trebaju.

    cd wips/lagani/proba
    cargo run                # početni prozor
    cargo run -- --cm        # prozor za odobravanje veze

Datoteke `d2d.rs`, `izgled.rs`, `win32.rs`, `glavni.rs`, `cm.rs` uključene su preko `#[path = "../..."]`, pa se mijenjaju
na jednom mjestu. Ovo nije dokaz da se cijeli program gradi: za to je CI workflow `wips-build-lagani.yml`.
