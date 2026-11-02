# WiPS Udaljena Korisnicka Podrska: klijent

Ovaj repozitorij je **RustDesk** (https://github.com/rustdesk/rustdesk, AGPL-3.0) na fiksiranoj verziji iz `wips/UPSTREAM`, uz WiPS izmjene koje se primjenjuju u CI-ju.

| Što | Gdje |
|---|---|
| Upstream verzija | `wips/UPSTREAM` |
| Naše izmjene izvornog koda | `wips/patches/app/*.patch`, `wips/patches/hbb_common/*.patch` |
| Brend (ikone, logo) | `wips/branding/` |
| Primjena izmjena | `wips/apply-patches.sh` |
| Build | `.github/workflows/wips-build-windows.yml` (ručno ili git tag `v*`) |

**Licenca:** AGPL-3.0, kao upstream (`LICENCE`). Izvorni kod svake objavljene verzije javno je dostupan.

## Izmjene
| Patch | Što mijenja |
|---|---|
| `hbb_common/0001-wips-naziv-server-kljuc` | naziv aplikacije, ID server, javni ključ, api-server (zapis spajanja) |
| `hbb_common/0002-wips-ugradjene-opcije` | skriva "Powered by RustDesk" i mrežne postavke (server, proxy, WebSocket), da korisnika nitko ne može nagovoriti na tuđi server |
| `app/0001-wips-windows-metapodaci` | naziv i tvrtka u svojstvima .exe datoteka (program i exe koji se preuzima) |
| `app/0002-wips-o-programu-i-licenca` | "O aplikaciji": izvorni kod, wips.hr, napomena o RustDesku i licenci; instalacija: link na licencu umjesto RustDeskove izjave o privatnosti |
| `app/0003-wips-hr-prijevod-naziva` | hrvatski tekstovi u kojima se naziv sklanja ("RustDesku" bi postalo "WiPSPodrskau") |

Vrijednosti `@@WIPS_*@@` dolaze iz `wips/server.env`. Ikone: `python3 -P wips/branding/generiraj-ikone.py` iz `logo-izvor.png` (treba Pillow), rezultat se commita; CI ih samo kopira.

## Status
- [x] Faza 1: build nepromijenjenog upstreama na GitHub Actions
- [x] Faza 2: patchevi za naziv, server i ključ (testni server)
- [x] Faza 3: ikone, logo, "O aplikaciji", skrivene mrežne postavke
- [ ] Faza 4: pravi javni ključ s rd.wips.hr (ugrađen), test, pilot verzija
