# WiPS Udaljena Korisnicka Podrska: klijent

Ovaj repozitorij je **RustDesk** (https://github.com/rustdesk/rustdesk, AGPL-3.0) na fiksiranoj verziji iz `wips/UPSTREAM`, uz WiPS izmjene koje se primjenjuju u CI-ju.

| Što | Gdje |
|---|---|
| Upstream verzija | `wips/UPSTREAM` |
| Naše izmjene izvornog koda | `wips/patches/app/*.patch`, `wips/patches/hbb_common/*.patch` |
| Brend (ikone, logo) | `wips/branding/` |
| Primjena izmjena | `wips/apply-patches.sh <varijanta>` |
| Build korisničke verzije | `.github/workflows/wips-build-lagani.yml` (ručno ili git tag `v*`): lagani klijent bez Fluttera, jedan exe; vidi [`LAGANI.md`](LAGANI.md) |
| Build tehničarske verzije | `.github/workflows/wips-build-windows.yml` (ručno ili git tag `v*`), Flutter |
| Potpis | Azure Artifact Signing u oba workflowa (postavljanje je opisano u zaglavlju `wips-build-windows.yml`); bez varijable `ARTIFACT_SIGNING_ACCOUNT` build je nepotpisan. Zbog environmenta `release` build se može pokrenuti samo s `main` ili taga `v*` |

**Licenca:** AGPL-3.0, kao upstream (`LICENCE`). Izvorni kod svake objavljene verzije javno je dostupan.

## Verzije
| | Korisnička (`korisnik`) | Tehničarska (`tehnicar`) |
|---|---|---|
| Naziv | `WiPSPodrska`, "WiPS Udaljena Korisnicka Podrska" | `WiPSPodrskaTehnicar`, "WiPS Podrska Tehnicar" |
| Veze | samo prima (`conn-type=incoming`; jezgra odbija odlazne veze) | prima i pokreće |
| Za koga | klijenti, stranica za preuzimanje | samo WiPS tehničari |
| Sučelje | lagani klijent (`wips/lagani/`, Win32 + Direct2D, bez Fluttera) | Flutter |

Vrijednosti su u `wips/varijante/<varijanta>.env`; zajedničke (server, ključ, linkovi) u `wips/server.env`.
Nazivi moraju biti različiti: o njima ovise mapa instalacije, Windows servis, postavke i ID.

Korisnička verzija nije sigurnosna granica prema serveru: javni ključ je u svakom klijentu, pa se službeni RustDesk može spojiti preko našeg servera.
Korisničku verziju štiti pristanak korisnika (svako spajanje treba klik, lozinka se ne koristi) i zapis spajanja. Zato ne može raditi bez nadzora (unattended); za takve uređaje treba tehničarska ili posebna varijanta.

## Izmjene
| Patch | Što mijenja |
|---|---|
| `hbb_common/0001-wips-naziv-server-kljuc` | naziv aplikacije, ID server, javni ključ, api-server (zapis spajanja) |
| `hbb_common/0002-wips-ugradjene-opcije` | skriva "Powered by RustDesk" i mrežne postavke (server, proxy, WebSocket), da korisnika nitko ne može nagovoriti na tuđi server; način rada verzije (`conn-type`); u korisničkoj verziji skriva kartice Mreža, Račun i Pisač u Postavkama (`WIPS_SKRIJ_KARTICE`) i prisiljava spajanje samo uz klik korisnika, bez lozinke (`approve-mode=click`, `WIPS_SAMO_KLIK_OPCIJA`) |
| `hbb_common/0003-wips-rezervni-server` | uz glavni server (`WIPS_HOST`) vraća i rezervni (`WIPS_HOST2`) kao popis servera; opcija `wips-server` bira servere (prazno = automatski oba, `glavni`, `rezervni`), a `wips_aktivni_server()` javlja na koji je klijent prijavljen; prazan `WIPS_HOST2` ne mijenja ništa ([`REZERVNI-SERVER.md`](REZERVNI-SERVER.md)) |
| `app/0001-wips-windows-metapodaci` | naziv i tvrtka u svojstvima .exe datoteka (program i exe koji se preuzima), izvorni naziv = naziv verzije |
| `app/0002-wips-o-programu-i-licenca` | "O aplikaciji": izvorni kod, wips.hr, napomena o RustDesku i licenci; instalacija: link na licencu umjesto RustDeskove izjave o privatnosti |
| `app/0003-wips-hr-prijevod-naziva` | hrvatski tekstovi u kojima se naziv sklanja ("RustDesku" bi postalo "WiPSPodrskau") |
| `app/0004-wips-izgled-korisnicke-verzije` | novi početni prozor korisničke verzije (`wips_incoming_home.dart`, smjer C "Svijetli Fluent", font Manrope iz `wips/branding/fonts/`); tehničarska verzija ga ne koristi |
| `app/0005-wips-izgled-prozora-za-odobravanje` | prozor za odobravanje i tijek veze u korisničkoj verziji (`wips_cm_card.dart`, dio `server_page.dart`): zaglavlje s tehničarem i stanjem, napomena prije odobravanja, dozvole kao popis s prekidačima, gumbi u stilu C; logika gumba ostaje upstream (`_CmControlPanel`) |
| `app/0006-wips-lagano-sucelje` | lagano sučelje (`wips/lagani/`, [`LAGANI.md`](LAGANI.md)): feature `lagani` u `Cargo.toml`, modul preko `#[path]` u `src/lib.rs`, poziv iz `src/main.rs`, manifest i ikona iz `build.rs`, glasovni poziv u `src/ui_cm_interface.rs` dostupan i uz `lagani`; bez feature-a `lagani` ne mijenja ništa (Flutter buildovi ga ne koriste) |
| `app/0007-wips-rezervni-server` | s više servera svaki se u `src/rendezvous_mediator.rs` spaja i ponovno pokušava sam (`start_neovisno`), da pad jednoga ne prekida prijavu na drugi; s jednim serverom stari put ostaje netaknut |
| `app/0008-wips-izdavac` | izdavač u unosu za deinstalaciju (`Publisher`) je `WIPS_PUBLISHER` (kao u potpisu), ne naziv aplikacije |
| `app/0009-wips-aktivni-server` | IPC upit `wips-aktivni-server` u `src/ipc.rs`: sučelje od servisa doznaje na koji je server klijent prijavljen i prikazuje „glavni” ili „rezervni” na početnom prozoru |
| `app/0010-wips-lagani-instalacija-samo-exe` | uz `lagani` instalacija i nadogradnja (`src/platform/windows.rs`) u mapu programa kopiraju samo pokrenuti exe, pod imenom aplikacije, a ne cijelu mapu iz koje je pokrenut (upstream XCOPY kopirao bi npr. cijele Preuzimanja); veličina u popisu programa je veličina exe-a; bez `lagani` ne mijenja ništa |

Vrijednosti `@@WIPS_*@@` dolaze iz `wips/server.env` i `wips/varijante/<varijanta>.env`. Ikone (iste za obje verzije): `python3 -P wips/branding/generiraj-ikone.py` iz `logo-izvor.png` (treba Pillow), rezultat se commita; CI ih samo kopira.

## Status
- [x] Faza 1: build nepromijenjenog upstreama na GitHub Actions
- [x] Faza 2: patchevi za naziv, server i ključ (testni server)
- [x] Faza 3: ikone, logo, "O aplikaciji", skrivene mrežne postavke
- [x] Faza 4: pravi javni ključ s rd.wips.hr, test izvana (3. 11. 2026.)
- [ ] Faza 5: korisnička i tehničarska verzija, test, pilot
- [x] Lagani korisnički klijent ([`LAGANI.md`](LAGANI.md)); od 13. 11. 2026. jedina korisnička verzija, Flutter samo za tehničare
