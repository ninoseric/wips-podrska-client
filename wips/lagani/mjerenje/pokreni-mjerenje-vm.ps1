# Na domaćinu: pošalje exe laganog klijenta u testnu virtualku (WiPS test agent), pričeka mjerenje,
# snimi zaslon virtualke dok je prozor otvoren i ispiše rezultate.
#   .\pokreni-mjerenje-vm.ps1 -Exe C:\put\WiPSPodrska.exe [-Vm 'Windows 10 Dev'] [-Snimka snimka.png]
# Virtualka mora biti iste arhitekture kao exe („Windows 10 Dev” je 32-bitna, pa traži i686 build).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [string]$Vm = 'Windows 10 Dev',
    [string]$Snimka = (Join-Path $PSScriptRoot 'snimka-vm.png')
)
$ErrorActionPreference = 'Stop'
. (Join-Path $env:USERPROFILE '.claude\skills\wips-redizajn\alati\vm\vm.ps1')
$VM_IME = $Vm

$agent = Start-WipsVm
"Agent: $($agent.racunalo), verzija $($agent.verzija)"
$id = Submit-WipsPosao 'lagani_mjerenje' (Join-Path $PSScriptRoot 'posao_mjerenje.ps1') @{
    (Split-Path $Exe -Leaf) = $Exe
    'izmjeri-start.ps1'     = (Join-Path $PSScriptRoot 'izmjeri-start.ps1')
} 15
"Posao: $id"

# Mjerenja su gotova kad se pojavi mjerenje.json; nakon toga izmjeri-start.ps1 drži prozor otvoren 25 s.
$r = "$RAZMJENA\rezultati\$id"
$t0 = Get-Date
while (-not (Test-Path "$r\mjerenje.json") -and -not (Test-Path "$r\GOTOVO")) {
    if (((Get-Date) - $t0).TotalMinutes -gt 12) { throw "Mjerenje nije završilo u 12 min." }
    Start-Sleep -Seconds 2
}
Start-Sleep -Seconds 8
& $VBM controlvm $VM_IME screenshotpng $Snimka
"Snimka: $Snimka"

$res = Wait-WipsPosao $id 15
""
Get-Content "$($res.Mapa)\mjerenje.txt"
""
"--- zapisnik programa ---"
Get-Content "$($res.Mapa)\zapisnik-lagani.txt" -ErrorAction SilentlyContinue
"--- OS ---"
Get-Content "$($res.Mapa)\os.txt" -ErrorAction SilentlyContinue
