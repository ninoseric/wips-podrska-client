# U virtualki: pričeka prozor zadane klase, pa klikne na (X, Y) u logičkim pikselima (DIP) njegovog klijentskog dijela.
# Više klikova: -Tocke "90,550;200,100". Zapisnik u rezultat\klik\klik.txt, GOTOVO kad završi.
param([string]$Klasa, [string]$Tocke, [int]$Cekaj = 120, [int]$Pauza = 1, [int]$Razmak = 1500)

$izlaz = Join-Path $PSScriptRoot "rezultat\klik"
New-Item -ItemType Directory -Force $izlaz | Out-Null
$log = Join-Path $izlaz "klik.txt"
Remove-Item -Force (Join-Path $izlaz "GOTOVO") -ErrorAction SilentlyContinue
function Zapisi($t) { "$(Get-Date -Format 'HH:mm:ss.fff') $t" | Add-Content $log }

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class K {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
}
"@

Zapisi "čekam prozor $Klasa"
$h = [IntPtr]::Zero
$kraj = (Get-Date).AddSeconds($Cekaj)
while ((Get-Date) -lt $kraj) {
  $h = [K]::FindWindowW($Klasa, [NullString]::Value)
  if ($h -ne [IntPtr]::Zero) { break }
  Start-Sleep -Milliseconds 200
}
if ($h -eq [IntPtr]::Zero) { Zapisi "prozor $Klasa nije stigao u $Cekaj s"; "NEMA" | Set-Content (Join-Path $izlaz "GOTOVO"); exit 1 }
Zapisi "prozor $Klasa (hwnd $h)"
if ([K]::IsIconic($h)) { [K]::ShowWindow($h, 9) | Out-Null; Zapisi "vraćen iz smanjenog"; Start-Sleep -Milliseconds 500 }
Start-Sleep -Seconds $Pauza
$dpi = [K]::GetDpiForWindow($h); if ($dpi -eq 0) { $dpi = 96 }
$s = $dpi / 96.0
foreach ($t in $Tocke.Split(';')) {
  $xy = $t.Split(',')
  $p = New-Object K+POINT; $p.X = [int]([double]$xy[0] * $s); $p.Y = [int]([double]$xy[1] * $s)
  [K]::ClientToScreen($h, [ref]$p) | Out-Null
  [K]::SetForegroundWindow($h) | Out-Null
  [K]::SetCursorPos($p.X, $p.Y) | Out-Null
  Start-Sleep -Milliseconds 300
  [K]::mouse_event(2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80; [K]::mouse_event(4, 0, 0, 0, [IntPtr]::Zero)
  Zapisi "klik $t -> ($($p.X),$($p.Y)), dpi $dpi"
  Start-Sleep -Milliseconds $Razmak
}
"OK" | Set-Content (Join-Path $izlaz "GOTOVO")
