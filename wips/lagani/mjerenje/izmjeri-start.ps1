# Mjeri veličinu exe-a i vrijeme od pokretanja do pojave prvog vidljivog glavnog prozora programa
# (bilo kojeg procesa čije ime počinje imenom exe-a, pa radi i za Flutter verziju koja se raspakira i pokrene
# drugi exe). Prvo pokretanje je „hladno” (Defender skenira novi exe), sljedeća su „topla”.
#
#   .\izmjeri-start.ps1 -Exe C:\put\WiPSPodrska.exe [-Ponavljanja 3] [-Izlaz rezultat.json] [-Prikaz 20] [-Naziv WiPSPodrska]
#
# Program se nakon svakog mjerenja gasi (svi procesi s tim prefiksom imena). -Prikaz N ga na kraju još jednom
# pokrene i ostavi N sekundi otvorenog (za snimku zaslona), pa ugasi.
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [int]$Ponavljanja = 3,
    [string]$Izlaz = '',
    [int]$Prikaz = 0,
    [string]$Naziv = '',
    # Prozor traži samo u procesu koji se zove točno $Naziv (Flutter: raspakirani WiPSPodrska.exe, a ne
    # packer WiPSPodrska-1.5.1-x86_64.exe s prozorom napretka raspakiravanja).
    [switch]$Tocno
)
$ErrorActionPreference = 'Stop'

$datoteka = Get-Item -LiteralPath $Exe
if (-not $Naziv) { $Naziv = ([IO.Path]::GetFileNameWithoutExtension($datoteka.Name) -split '-')[0] }
$uzorak = "$Naziv*"
$uzorakProzor = if ($Tocno) { $Naziv } else { $uzorak }

function Ugasi { Get-Process -Name $uzorak -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue }
function ProzorHandle {
    foreach ($pr in (Get-Process -Name $uzorakProzor -ErrorAction SilentlyContinue)) {
        if ($pr.MainWindowHandle -ne [IntPtr]::Zero) { return $pr.MainWindowHandle }
    }
    return [IntPtr]::Zero
}

Ugasi
Start-Sleep -Milliseconds 500

$rezultati = @()
for ($i = 1; $i -le $Ponavljanja; $i++) {
    $sat = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $datoteka.FullName -PassThru
    $hwnd = [IntPtr]::Zero
    while ($sat.ElapsedMilliseconds -lt 90000) {
        $hwnd = ProzorHandle
        if ($hwnd -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 20
    }
    $ms = $sat.ElapsedMilliseconds
    $ok = $hwnd -ne [IntPtr]::Zero
    $rezultati += [pscustomobject]@{ pokretanje = $i; hladno = ($i -eq 1); prozor_ms = $ms; uspjeh = $ok }
    "{0}. pokretanje{1}: {2}" -f $i, $(if ($i -eq 1) { ' (hladno)' } else { '' }), $(if ($ok) { "$ms ms do prozora" } else { "prozor se nije pojavio ($ms ms)" })
    Start-Sleep -Seconds 3
    Ugasi
    Start-Sleep -Seconds 2
}

$velicina = [math]::Round($datoteka.Length / 1MB, 1)
"Veličina: $velicina MB ($($datoteka.Length) B), $($datoteka.Name)"
$os = Get-CimInstance Win32_OperatingSystem
$sazetak = [pscustomobject]@{
    exe = $datoteka.Name; velicina_mb = $velicina; velicina_b = $datoteka.Length
    os = "$($os.Caption) $($os.Version) ($($os.OSArchitecture))"; racunalo = $env:COMPUTERNAME
    vrijeme = (Get-Date -Format 's'); pokretanja = $rezultati
}
if ($Izlaz) { $sazetak | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Izlaz -Encoding UTF8; "Zapisano: $Izlaz" }

if ($Prikaz -gt 0) {
    "Prikaz prozora $Prikaz s (za snimku zaslona)..."
    $p = Start-Process -FilePath $datoteka.FullName -PassThru
    Start-Sleep -Seconds $Prikaz
    Ugasi
}
