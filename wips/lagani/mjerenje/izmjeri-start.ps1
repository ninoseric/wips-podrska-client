# Mjeri veličinu exe-a i vrijeme od pokretanja procesa do pojave glavnog prozora laganog klijenta
# (klasa prozora WiPSLaganiGlavni, wips/lagani/glavni.rs). Prvo pokretanje je „hladno” (Defender skenira
# novi exe), sljedeća su „topla”. Radi na bilo kojem 64-bitnom Windowsu (domaćin ili virtualka).
#
#   .\izmjeri-start.ps1 -Exe C:\put\WiPSPodrska.exe [-Ponavljanja 3] [-Izlaz rezultat.json]
#
# Program se nakon svakog mjerenja gasi (zajedno s pomoćnim procesima istog imena).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [int]$Ponavljanja = 3,
    [string]$Izlaz = ''
)
$ErrorActionPreference = 'Stop'
$klasa = 'WiPSLaganiGlavni'

Add-Type -Namespace WipsMjerenje -Name Win32 -MemberDefinition @"
[System.Runtime.InteropServices.DllImport("user32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
public static extern System.IntPtr FindWindowW(string lpClassName, string lpWindowName);
"@

$exe = Get-Item -LiteralPath $Exe
$naziv = [IO.Path]::GetFileNameWithoutExtension($exe.Name)
Get-Process -Name $naziv -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

$rezultati = @()
for ($i = 1; $i -le $Ponavljanja; $i++) {
    $sat = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $exe.FullName -PassThru
    $hwnd = [IntPtr]::Zero
    while ($sat.ElapsedMilliseconds -lt 60000) {
        $hwnd = [WipsMjerenje.Win32]::FindWindowW($klasa, $null)
        if ($hwnd -ne [IntPtr]::Zero) { break }
        if ($p.HasExited) { break }
        Start-Sleep -Milliseconds 10
    }
    $ms = $sat.ElapsedMilliseconds
    $ok = $hwnd -ne [IntPtr]::Zero
    $rezultati += [pscustomobject]@{ pokretanje = $i; hladno = ($i -eq 1); prozor_ms = $ms; uspjeh = $ok }
    "{0}. pokretanje{1}: {2}" -f $i, $(if ($i -eq 1) { ' (hladno)' } else { '' }), $(if ($ok) { "$ms ms do prozora" } else { "prozor se nije pojavio ($ms ms)" })
    Start-Sleep -Seconds 2
    Get-Process -Name $naziv -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
}

$velicina = [math]::Round($exe.Length / 1MB, 1)
"Veličina: $velicina MB ($($exe.Length) B), $($exe.Name)"
$os = Get-CimInstance Win32_OperatingSystem
$sazetak = [pscustomobject]@{
    exe = $exe.Name; velicina_mb = $velicina; velicina_b = $exe.Length
    os = "$($os.Caption) $($os.Version) ($($os.OSArchitecture))"; racunalo = $env:COMPUTERNAME
    vrijeme = (Get-Date -Format 's'); pokretanja = $rezultati
}
if ($Izlaz) { $sazetak | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Izlaz -Encoding UTF8; "Zapisano: $Izlaz" }
