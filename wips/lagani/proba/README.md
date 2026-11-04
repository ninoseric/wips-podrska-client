# Lokalni testni okvir za lagano sučelje

Prevodi i pokreće pravi `wips/lagani/mod.rs` (sve prozore) sa zamjenama za RustDesk jezgru (`src/main.rs`: lažni ID,
stanje „spojeno”, lažni zahtjev tehničara, poruka nakon prihvaćanja). Treba samo rustup (toolchain 1.75, kao CI);
vcpkg i Flutter ne trebaju.

    cd wips/lagani/proba
    cargo run                    # početni prozor (nije instaliran: redak „Instaliraj”)
    cargo run -- --cm            # prozor za odobravanje veze
    cargo run -- --install       # instalacija

Varijable okoline (opis na vrhu `src/main.rs`): `LAGANI_TEMA=dark`, `LAGANI_JEZIK=en`, `LAGANI_DVA=1` (dvije veze),
`LAGANI_POZIV=1` (glasovni poziv), `LAGANI_STARIJA=1` (redak za nadogradnju), `LAGANI_KLIK="x,y;x,y"` (klikovi u DIP
koordinatama nakon 2 s), `LAGANI_TEKST=...` (poruka tehničaru preko polja za poruku).

Ovo nije dokaz da se cijeli program gradi: za to je CI workflow `wips-build-lagani.yml`.
