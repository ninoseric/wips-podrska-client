# Lagani korisnički klijent (prototip)

Ovaj repozitorij je odvojen od `ninoseric/wips-podrska-client` 4. 11. 2026. (na commitu `610d1f6`), da se nova
tehnologija sučelja razvija bez rizika za postojeći klijent. Cilj je korisnička verzija koja je po veličini i brzini
pokretanja bliža AnyDesk QuickSupportu (3,5 MB, brz start), a koju smo dosad distribuirali korisnicima.

Repo je privatan dok traje prototip. Prije prve isporuke korisnicima mora postati javan, jer AGPL-3.0 traži javan
izvorni kod svega što distribuiramo (kao `wips-podrska-client`).

## Polazno stanje: zašto je sadašnja korisnička verzija velika i spora

Izmjereno na CI buildu `41803996932` (korisnička verzija, RustDesk 1.5.1 + WiPS zakrpe):

| Datoteka | Veličina | Što je |
|---|---|---|
| `librustdesk.dll` | 39,5 MB | Rust jezgra: mreža, kodeci (VP8/VP9/AV1, uz `--hwcodec`/`--vram` i hardverski), prijenos datoteka, ispis, virtualni zaslon, i kod za upravljanje tuđim računalom koji korisnička verzija ne koristi |
| `flutter_windows.dll` | 17,2 MB | Flutter engine |
| `data/app.so` | 13,2 MB | Prevedeni Dart kod cijelog sučelja (i sakrivenih ekrana) |
| ~15 `*_plugin.dll` | ~5 MB | Flutter dodaci |
| fontovi, ikone, ICU | ~3 MB | |

Raspakirano je oko 80 MB. `libs/portable` to pakira brotlijem u jedan exe od oko 25 MB.

Zašto je pokretanje sporo:
1. Exe se sam raspakirava: pri prvom pokretanju izdanja raspakira ~80 MB u `%LOCALAPPDATA%\wipspodrska`
   (kasnija pokretanja istog izdanja to preskaču, provjerom vremenske oznake).
2. Zatim se učitavaju Flutter engine, Dart sučelje i Rust DLL od 40 MB.
3. Windows Defender pri prvom pokretanju skenira raspakirane DLL-ove.

Udio pojedinog koraka u vremenu pokretanja još nije izmjeren.

Odluka: sadašnji build ne režemo (izbacivanje kodeka i pluginova dalo bi nekoliko MB, ne red veličine).
Za korisničku verziju mijenjamo tehnologiju sučelja.

## Smjer

- **Zadržava se Rust jezgra RustDeska**: protokol, rd.wips.hr server, snimanje ekrana, kodeci, upravljanje unosom,
  odobravanje veze. Sučelje s jezgrom razgovara preko `src/ui_interface.rs` i `src/ui_cm_interface.rs`
  (oko 3.700 linija, ne ovise o Flutteru; na njih je bio spojen i stari Sciter UI).
- **Samo korisnička verzija** dobiva novo, lagano sučelje i gradi se bez feature-a `flutter`.
  Tehničarska verzija ostaje na Flutteru (u `wips-podrska-client`).
- **Preporučena tehnologija: WebView2** (HTML u nativnom prozoru; WebView2 je već u Windows 10/11).
  Dizajn smjera C („Svijetli Fluent”) već postoji kao HTML, ekrana je malo, a razlika u brzini prema nativnom
  Win32 je zanemariva. Alternativa je nativni Win32 iz Rusta: najmanji i najbrži, ali izgled se ručno crta.

| | Nativni Win32 | WebView2 |
|---|---|---|
| Veličina dodatka | ~nula | malo (runtime je u sustavu) |
| Pokretanje | trenutno | brzo |
| Izgled smjera C | ručno crtanje | skice se gotovo izravno prenose |
| Održavanje izgleda | teže | lako |

Očekivano: jedan exe bez raspakiravanja pri pokretanju, start za oko sekundu. Jezgra bez Fluttera procijenjena je
na 8–15 MB (komprimirano možda 5–8 MB); 3,5 MB je teško bez izbacivanja kodeka. To su procjene, ne mjerenja.

## Plan

1. **Prototip (2–3 dana)**: jezgra bez Fluttera + prazan prozor koji prikazuje ID. Mjerenje veličine i vremena
   pokretanja (prvo i drugo pokretanje) u virtualki „Windows 10 Dev”. Ovdje se odlučuje idemo li dalje.
2. **Sučelje (~1,5–2 tjedna)**: početni prozor (ID, status, telefon 01/61 76 166), prozor za odobravanje veze,
   indikator aktivne veze s prekidom, po želji chat.
3. **Procesi i integracija (~1 tjedan)**: glavni proces, `--server`, `--cm` (odobravanje), UAC, portabilni način.
4. **CI, potpis, pakiranje, testiranje (~1 tjedan)**: nova varijanta u workflowu, Azure potpis, testovi u virtualki.

Ukupno oko 4–6 tjedana.

## Rizici

- Put bez Fluttera (`#[cfg(not(feature = "flutter"))]`) dosad je služio Sciteru, koji je zastario; dio se možda ne
  gradi čisto. Popravci idu kao zakrpe u `wips/patches/` i nose se kroz svako ažuriranje upstreama.
- Uz svako ažuriranje RustDeska treba provjeriti promjene u `ui_interface` / `ui_cm_interface`.
- Dvije tehnologije sučelja: Flutter (tehničarska) i novi UI (korisnička).

## Napomene za ovaj repo

- CI workflow (`.github/workflows/wips-build-windows.yml`) je kopiran iz `wips-podrska-client`. Potpisivanje se ovdje
  preskače dok repo nema varijablu `ARTIFACT_SIGNING_ACCOUNT`; Azure federated credential vezan je uz ID-ove
  repozitorija, pa za ovaj repo treba poseban credential ako ovdje zatreba potpis.
- Privatni repo troši GitHub Actions minute (Windows runneri se naplaćuju dvostruko), za razliku od javnog.

## Prototip (korak 1 plana), stanje 4. 11. 2026.

Napravljeno, čeka prvi CI build i mjerenje:

| Što | Gdje |
|---|---|
| Novo sučelje (Win32 + GDI, bez dodatnih ovisnosti) | `wips/lagani/`: `glavni.rs` početni prozor (ID, stanje veze, uputa, „Kopiraj ID”), `cm.rs` prozor za odobravanje veze (`--cm`: Dopusti / Odbij / Prekini vezu), `win32.rs` omotač oko winapi-ja |
| Uključivanje u build | patch `wips/patches/app/0006-wips-lagano-sucelje.patch`: feature `lagani` u `Cargo.toml` (+ winapi feature-i `wingdi`, `libloaderapi`, `processthreadsapi`, `sysinfoapi`), `#[path = "../wips/lagani/mod.rs"] pub mod lagani` u `src/lib.rs`, poziv `lagani::start` umjesto `ui::start` u `src/main.rs`, ikona i manifest (DPI, vizualni stilovi) iz `build.rs` i za `lagani` |
| Build | `.github/workflows/wips-build-lagani.yml` (ručno pokretanje; opcija `hwcodec`): korisnička varijanta + svi patchevi, `cargo build --release --features lagani --bin rustdesk`, bez Fluttera, bridgea i TopMostWindow-a; artefakt `wips-podrska-lagani-windows-x64` s `WiPSPodrska.exe` i `VELICINA.txt` |
| Mjerenje | `wips/lagani/mjerenje/izmjeri-start.ps1 -Exe WiPSPodrska.exe`: veličina i vrijeme do pojave prozora (prvo, hladno pokretanje i sljedeća); program u zapisnik upisuje i vlastito vrijeme od stvaranja procesa do prozora („prikazan N ms nakon pokretanja procesa”) |

Kako radi: `core_main` ostaje isti (server u procesu kad program nije instaliran, `--server` iz servisa kad jest, `--cm` pokreće jezgra pri zahtjevu za vezu). Sciter modul `src/ui` se i dalje prevodi (na njega se oslanjaju `keyboard.rs` i `client/file_trait.rs`), ali se ne poziva; sciter.dll nije potreban jer se učitava tek na poziv. Tekstovi su zasad samo hrvatski, u kodu.

Još nema: instalacijskog tijeka (`--install`, gumb „Instaliraj”), chata, fonta Manrope i tamne teme, prikaza „odspojenog” tehničara, UAC/elevacije iz prozora, mjerenja s WebView2 (odluka Win32 ili WebView2 ovisi o rezultatu mjerenja). Potpis se u ovom workflowu preskače.

Zamka pri mjerenju: virtualka „Windows 10 Dev” je **32-bitna**, a build je x64, pa se u njoj ne može pokrenuti. Mogućnosti: virtualka „Windows 11 Pro” (64-bitna, treba dopuštenje i test agent), računalo domaćina, ili dodatni i686 build (vcpkg triplet `x86-windows-static`, kao upstream Sciter build).

Rezultati mjerenja: (upisati nakon prvog builda) veličina exe-a, hladno i toplo pokretanje, usporedba s Flutter verzijom (25 MB exe, oko 80 MB raspakirano).
