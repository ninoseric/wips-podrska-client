# Ugasi lagani klijent, kopiraj novi exe u Preuzimanja (kao kod korisnika) i pokreni ga odande.
Get-Process WiPSPodrska* -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
$cilj = Join-Path $env:USERPROFILE "Downloads\WiPSPodrska.exe"
Copy-Item (Join-Path $PSScriptRoot "WiPSPodrska.exe") $cilj -Force
Start-Process $cilj
