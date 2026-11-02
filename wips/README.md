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

## Status
- [ ] Faza 1: build nepromijenjenog upstreama na GitHub Actions
- [ ] Faza 2: patchevi za naziv, server i ključ (testni server)
- [ ] Faza 3: ikone i logo
- [ ] Faza 4: pravi javni ključ s rd.wips.hr, pilot verzija
