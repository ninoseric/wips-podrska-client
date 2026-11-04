# Pokreće se u virtualki (Win+R): mjeri klijent i rezultate piše u rezultat\<Oznaka>\ u dijeljenoj mapi.
#   powershell -ExecutionPolicy Bypass -File Z:\WiPS-test\lagani\posao.ps1 -Exe WiPSPodrska.exe -Oznaka lagani
param([string]$Exe = 'WiPSPodrska.exe', [string]$Oznaka = 'lagani', [int]$Prikaz = 25)
$ErrorActionPreference = 'Continue'
$share = $PSScriptRoot
$rez = Join-Path $share "rezultat\$Oznaka"
New-Item -ItemType Directory -Force $rez | Out-Null
Start-Transcript -Path (Join-Path $rez 'transcript.txt') -Force | Out-Null
try {
    $cilj = Join-Path $env:TEMP "wips-$Oznaka"
    if (Test-Path $cilj) { Remove-Item -Recurse -Force $cilj -ErrorAction SilentlyContinue }
    New-Item -ItemType Directory -Force $cilj | Out-Null
    Copy-Item (Join-Path $share $Exe) $cilj -Force
    $exePut = Join-Path $cilj $Exe
    "Kopirano: $exePut"
    # Flutter verzija raspakira u %LOCALAPPDATA%\wipspodrska; obrisati da prvo pokretanje bude stvarno hladno.
    $rasp = Join-Path $env:LOCALAPPDATA 'wipspodrska'
    if ($Oznaka -ne 'lagani' -and (Test-Path $rasp)) { Remove-Item -Recurse -Force $rasp -ErrorAction SilentlyContinue; "Obrisano: $rasp" }
    $tocno = ($Oznaka -ne 'lagani')
    & (Join-Path $share 'izmjeri-start.ps1') -Exe $exePut -Ponavljanja 4 -Izlaz (Join-Path $rez 'mjerenje.json') -Prikaz $Prikaz -Naziv WiPSPodrska -Tocno:$tocno
    $zap = @()
    foreach ($k in @("$env:APPDATA\WiPSPodrska", "$env:LOCALAPPDATA\WiPSPodrska")) {
        if (Test-Path $k) { $zap += Get-ChildItem $k -Recurse -Filter *.log -ErrorAction SilentlyContinue }
    }
    $zap | ForEach-Object { Select-String -Path $_.FullName -Pattern 'lagani|panic|error' -ErrorAction SilentlyContinue } |
        ForEach-Object { $_.Line } | Set-Content (Join-Path $rez 'zapisnik.txt')
    if (Test-Path $rasp) { "Raspakirano u $rasp : {0:N1} MB" -f ((Get-ChildItem $rasp -Recurse -File | Measure-Object Length -Sum).Sum / 1MB) }
} finally {
    Stop-Transcript | Out-Null
    Set-Content (Join-Path $rez 'GOTOVO') (Get-Date -Format 's')
}
