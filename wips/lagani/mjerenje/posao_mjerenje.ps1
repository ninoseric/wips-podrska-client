# Posao za WiPS test agent u virtualki (alati/vm/README.md): izmjeri lagani klijent i vrati rezultate.
# U mapi posla ($env:WIPS_POSAO) očekuje exe klijenta i izmjeri-start.ps1; rezultate piše u $env:WIPS_REZULTAT.
$ErrorActionPreference = 'Continue'
$posao = $env:WIPS_POSAO
$rez = $env:WIPS_REZULTAT

$exe = Get-ChildItem -Path $posao -Filter *.exe | Select-Object -First 1
if (-not $exe) { throw "U poslu nema exe datoteke." }

# Svježa kopija u TEMP: prvo pokretanje je hladno (Defender skenira novu datoteku).
$cilj = Join-Path $env:TEMP 'wips-lagani'
if (Test-Path $cilj) { Remove-Item -Recurse -Force $cilj -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force $cilj | Out-Null
Copy-Item -LiteralPath $exe.FullName -Destination $cilj -Force
$exeKopija = Join-Path $cilj $exe.Name

& (Join-Path $posao 'izmjeri-start.ps1') -Exe $exeKopija -Ponavljanja 3 -Izlaz (Join-Path $rez 'mjerenje.json') -Prikaz 25 *>&1 |
    Tee-Object -FilePath (Join-Path $rez 'mjerenje.txt')

# Redci zapisnika programa koje piše lagano sučelje ("prikazan N ms nakon pokretanja procesa").
$zapisnici = @()
foreach ($korijen in @("$env:APPDATA\WiPSPodrska", "$env:LOCALAPPDATA\WiPSPodrska")) {
    if (Test-Path $korijen) { $zapisnici += Get-ChildItem $korijen -Recurse -Filter *.log -ErrorAction SilentlyContinue }
}
$zapisnici | ForEach-Object { Select-String -Path $_.FullName -Pattern 'lagani' -ErrorAction SilentlyContinue } |
    ForEach-Object { $_.Line } | Set-Content (Join-Path $rez 'zapisnik-lagani.txt')

Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, OSArchitecture, TotalVisibleMemorySize |
    Format-List | Out-String | Set-Content (Join-Path $rez 'os.txt')
