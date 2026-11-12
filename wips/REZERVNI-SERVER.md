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

## Poznata ograničenja

- **Zapis spajanja (audit)** ide samo na `api-server` = glavni server. Dok glavni ne radi, spajanja preko rezervnog se
  ne bilježe.
- Ako glavni **radi**, ali korisnik do njega ne dolazi (problem samo na korisnikovoj mreži), tehničar s glavnog dobiva
  „offline” i ne prelazi sam na rezervni. Prelazi samo kad glavni ne odgovara tehničaru.
- Klijenti izgrađeni prije ove promjene znaju samo za glavni server.
- Ako glavni server ne odgovara tako da paketi samo nestaju (bez odbijanja veze), tehničar čeka istek veze
  (`CONNECT_TIMEOUT`) prije prelaska na rezervni.

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
