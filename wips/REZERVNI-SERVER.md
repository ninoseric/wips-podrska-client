# Rezervni RustDesk server

Odluka 12. 11. 2026.: uz glavni server `rd.wips.hr` (wips-vps1) stalno radi i rezervni server na
Hetzneru. Ako glavni padne, podrška radi dalje bez ičije reakcije.

## Kako radi

- **Korisnički program** se prijavljuje na **oba** servera istodobno (`Config::get_rendezvous_servers()` vraća
  `[WIPS_HOST, WIPS_HOST2]`, `RendezvousMediator::start_all` otvara vezu sa svakim).
  - Zakrpa `app/0007` pazi da pad jednog servera ne prekida prijavu na drugi.
  - Status u prozoru pokazuje najbolji od dva (`get_online_state` = maksimum), pa „Spremno za podršku” stoji i kad
    radi samo rezervni.
- **Tehničarski program** spaja se preko glavnog servera (`custom-rendezvous-server` ostaje `WIPS_HOST`). Ako TCP veza
  s glavnim ne uspije, sam pokušava rezervni (`Client::_start_inner`, „try the other servers”). To je kod iz RustDeska,
  nije naša zakrpa.
- Oba servera imaju **isti par ključeva** (`id_ed25519`), pa javni ključ u exe-u vrijedi za oba.
- Relay: hbbs nije pokrenut s `-r`, pa klijent koristi relay na istom računalu kao i server preko kojeg je spojen.

## Postavke

`wips/server.env`: `WIPS_HOST2` je adresa rezervnog servera. Prazno znači samo glavni server, kao prije.

**Izbor servera u korisničkom programu** (od 13. 11. 2026.), u Postavkama, kartica Općenito, red „Server”:
- **Automatski (glavni, po potrebi rezervni)**: zadano, prijava na oba servera.
- **Samo glavni** ili **Samo rezervni**: prijava samo na taj server.

Vrijednost je opcija `wips-server` (prazno, `glavni` ili `rezervni`) u konfiguraciji servisa. Promjena odmah
ponovno pokreće prijavu. Instaliranoj verziji može se postaviti i naredbom (kao administrator):
`WiPSPodrska.exe --option wips-server rezervni`. Početni prozor ispod stanja piše „Povezano sa serverom WiPS
(glavni)” ili „(rezervni)”, prema serveru na koji je klijent stvarno prijavljen. Taj podatak daje IPC upit
`wips-aktivni-server` i osvježava se svake 3 s. Kad rade oba servera, piše „glavni”. Bez `WIPS_HOST2` red „Server”
se ne prikazuje.

## Poznata ograničenja

- **Zapis spajanja (audit)** ide samo na `api-server` = glavni server. Dok glavni ne radi, spajanja preko rezervnog se
  ne bilježe.
- Ako glavni **radi**, ali korisnik do njega ne dolazi (problem samo na korisnikovoj mreži), tehničar s glavnog dobiva
  „offline” i ne prelazi sam na rezervni. Prelazi samo kad glavni ne odgovara tehničaru.
- Klijenti izgrađeni prije ove promjene znaju samo za glavni server.
- Ako glavni server ne odgovara tako da paketi samo nestaju (bez odbijanja veze), tehničar čeka istek veze
  (`CONNECT_TIMEOUT`) prije prelaska na rezervni.
- **„Samo rezervni” i tehničar:** tehničar se spaja preko glavnog servera. Ako klijent ima „Samo rezervni”, a glavni
  radi, tehničar dobiva „offline” i ne prelazi na rezervni (`client.rs`: prelazi samo kad TCP veza s glavnim ne
  uspije). Isto tako, uz „Samo glavni” klijent nije dostupan dok glavni ne radi.

## Provjera

1. Oba servera rade: korisnik je spreman, tehničar se spaja.
2. Glavni nedostupan s obje strane, npr. firewall pravilo na testnoj virtualki prema 212.92.213.190: korisnik ostaje
   „Spreman”, tehničar se spaja preko rezervnog.
3. Glavni se vrati: nove veze opet idu preko glavnog.

### Rezultat (12. 11. 2026.)

Lagani klijent (run 43291727961) u virtualki „Windows 11 Pro”, nepotpisana tehničarska Flutter verzija
(run 43293582704) na domaćinu. Glavni server blokiran pravilom vatrozida: u virtualki za cijeli sustav, na domaćinu
samo za testni tehničarski exe.

| Slučaj | Rezultat |
|---|---|
| Oba servera rade | Klijent prijavljen na oba (hbbs na wips-rd2 zapisao ID 121179531); tehničar spojen preko `rd.wips.hr` |
| Glavni blokiran | Klijent je odmah označio glavni nedostupnim, prešao na `78.47.36.250` i ostao „Spremno za podršku”, bez prekida prijave na rezervni. Tehničar: „try the other servers” za 2 ms, veza i relay preko `78.47.36.250:21117`, odobreno |
| Glavni vraćen | Nova veza opet ide preko `rd.wips.hr` |

Dok glavni ne radi, klijent svake 2 minute zapiše upozorenje da `https://rd.wips.hr/api/heartbeat` i `/api/sysinfo`
ne odgovaraju (audit je samo na glavnom). Veze to ne ometa.

### Izbor servera (18. 11. 2026.)

Virtualka „Windows 11 Pro”, glavni server blokiran pravilom vatrozida u virtualki. Testiran je lagani klijent iz
runa 43518052517 (f93daa0), prenosiv i instaliran, a zatim instalirana verzija s popravkom visine postavki iz runa
44405140884 (2e07d53). Snimke i zapisi: `Share Virtualka\WiPS-test\lagani\server-izbor\rezultat\`.

| Slučaj | Rezultat |
|---|---|
| Automatski, oba servera rade | „Spremno za podršku”, „(glavni)”, prijava na oba |
| Samo rezervni, izbor u postavkama | Za manje od 1 s prijava samo na `78.47.36.250`, „(rezervni)” |
| Samo glavni | Prijava samo na `rd.wips.hr`, „(glavni)” |
| Samo glavni, glavni blokiran | „Podrška trenutno nije dostupna”, nema prijave na rezervni |
| Automatski, glavni blokiran | „Spremno za podršku”, „(rezervni)” |
| Glavni odblokiran | Prijava na glavni ponovno za 8 s, „(glavni)” |
| Instalirana verzija, naredba `--option wips-server …` | Servis mijenja prijavu i prozor pokazuje ispravan server. Čitanje vraća vrijednost, a `""` vraća Automatski |
| Instalirana verzija, red „Server” u postavkama | U f93daa0 je bio ispod ruba prozora (prozor 500 točaka, sadržaj sa Servisom 600) i nedostupan. Od 2e07d53 prozor instalirane verzije ima 620 točaka i red je cijeli. Izbor „Samo rezervni” pa „Automatski” radi i u servisu (`wips-server` u njegovoj konfiguraciji) |

Spajanje tehničara u ovom testu nije provjeravano.
