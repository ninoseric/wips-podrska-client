# WiPS Udaljena Korisnicka Podrska: klijent

Ovaj repozitorij je **RustDesk** (https://github.com/rustdesk/rustdesk, AGPL-3.0) na fiksiranoj verziji iz `wips/UPSTREAM`, uz WiPS izmjene koje se primjenjuju u CI-ju.

| Što | Gdje |
|---|---|
| Upstream verzija | `wips/UPSTREAM` |
| Naše izmjene izvornog koda | `wips/patches/app/*.patch`, `wips/patches/hbb_common/*.patch` |
| Brend (ikone, logo) | `wips/branding/` |
| Primjena izmjena | `wips/apply-patches.sh <varijanta>` |
| Build | `.github/workflows/wips-build-windows.yml` (ručno ili git tag `v*`), obje verzije paralelno |

**Licenca:** AGPL-3.0, kao upstream (`LICENCE`). Izvorni kod svake objavljene verzije javno je dostupan.

## Verzije
| | Korisnička (`korisnik`) | Tehničarska (`tehnicar`) |
|---|---|---|
| Naziv | `WiPSPodrska`, "WiPS Udaljena Korisnicka Podrska" | `WiPSPodrskaTehnicar`, "WiPS Podrska Tehnicar" |
| Veze | samo prima (`conn-type=incoming`; jezgra odbija odlazne veze) | prima i pokreće |
| Za koga | klijenti, stranica za preuzimanje | samo WiPS tehničari |

Vrijednosti su u `wips/varijante/<varijanta>.env`; zajedničke (server, ključ, linkovi) u `wips/server.env`.
Nazivi moraju biti različiti: o njima ovise mapa instalacije, Windows servis, postavke i ID.

Korisnička verzija nije sigurnosna granica prema serveru: javni ključ je u svakom klijentu, pa se službeni RustDesk može spojiti preko našeg servera.
Štite jednokratna lozinka i pristanak korisnika, a na unattended uređajima trajna lozinka, popis pouzdanih ID-ova i zapis spajanja.

## Izmjene
| Patch | Što mijenja |
|---|---|
| `hbb_common/0001-wips-naziv-server-kljuc` | naziv aplikacije, ID server, javni ključ, api-server (zapis spajanja) |
| `hbb_common/0002-wips-ugradjene-opcije` | skriva "Powered by RustDesk" i mrežne postavke (server, proxy, WebSocket), da korisnika nitko ne može nagovoriti na tuđi server; način rada verzije (`conn-type`); u korisničkoj verziji skriva kartice Mreža, Račun i Pisač u Postavkama (`WIPS_SKRIJ_KARTICE`) |
| `app/0001-wips-windows-metapodaci` | naziv i tvrtka u svojstvima .exe datoteka (program i exe koji se preuzima), izvorni naziv = naziv verzije |
| `app/0002-wips-o-programu-i-licenca` | "O aplikaciji": izvorni kod, wips.hr, napomena o RustDesku i licenci; instalacija: link na licencu umjesto RustDeskove izjave o privatnosti |
| `app/0003-wips-hr-prijevod-naziva` | hrvatski tekstovi u kojima se naziv sklanja ("RustDesku" bi postalo "WiPSPodrskau") |
| `app/0004-wips-izgled-korisnicke-verzije` | novi početni prozor korisničke verzije (`wips_incoming_home.dart`, smjer C "Svijetli Fluent", font Manrope iz `wips/branding/fonts/`); tehničarska verzija ga ne koristi |

Vrijednosti `@@WIPS_*@@` dolaze iz `wips/server.env` i `wips/varijante/<varijanta>.env`. Ikone (iste za obje verzije): `python3 -P wips/branding/generiraj-ikone.py` iz `logo-izvor.png` (treba Pillow), rezultat se commita; CI ih samo kopira.

## Status
- [x] Faza 1: build nepromijenjenog upstreama na GitHub Actions
- [x] Faza 2: patchevi za naziv, server i ključ (testni server)
- [x] Faza 3: ikone, logo, "O aplikaciji", skrivene mrežne postavke
- [x] Faza 4: pravi javni ključ s rd.wips.hr, test izvana (3. 11. 2026.)
- [ ] Faza 5: korisnička i tehničarska verzija, test, pilot
