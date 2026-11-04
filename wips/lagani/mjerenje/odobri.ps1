# U virtualki: čeka prozor za odobravanje laganog klijenta (klasa WiPSLaganiCm), pričeka
# -Pauza sekundi (da se s domaćina snimi zahtjev), pa klikne „Dopusti”. Zapisnik u rezultat\odobri\.
param([int]$Cekaj = 180, [int]$Pauza = 6)

$izlaz = Join-Path $PSScriptRoot "rezultat\odobri"
New-Item -ItemType Directory -Force $izlaz | Out-Null
$log = Join-Path $izlaz "odobri.txt"
Remove-Item -Force (Join-Path $izlaz "GOTOVO") -ErrorAction SilentlyContinue
function Zapisi($t) { "$(Get-Date -Format 'HH:mm:ss.fff') $t" | Add-Content $log }

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class U {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
}
"@

Zapisi "čekam prozor WiPSLaganiCm"
$h = [IntPtr]::Zero
$kraj = (Get-Date).AddSeconds($Cekaj)
while ((Get-Date) -lt $kraj) {
  $h = [U]::FindWindowW("WiPSLaganiCm", [NullString]::Value)
  if ($h -ne [IntPtr]::Zero) { break }
  Start-Sleep -Milliseconds 200
}
if ($h -eq [IntPtr]::Zero) { Zapisi "prozor nije stigao u $Cekaj s"; "NEMA" | Set-Content (Join-Path $izlaz "GOTOVO"); exit 1 }
Zapisi "prozor za odobravanje prikazan (hwnd $h)"
Start-Sleep -Seconds $Pauza

$dpi = [U]::GetDpiForWindow($h); if ($dpi -eq 0) { $dpi = 96 }
$s = $dpi / 96.0
$p = New-Object U+POINT; $p.X = [int](102 * $s); $p.Y = [int](200 * $s)   # središte gumba „Dopusti” (DIP)
[U]::ClientToScreen($h, [ref]$p) | Out-Null
[U]::SetForegroundWindow($h) | Out-Null
[U]::SetCursorPos($p.X, $p.Y) | Out-Null
Start-Sleep -Milliseconds 300
[U]::mouse_event(2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80; [U]::mouse_event(4, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 150; [U]::SetCursorPos(20, 20) | Out-Null
Zapisi "kliknuto Dopusti na ($($p.X),$($p.Y)), dpi $dpi"
"OK" | Set-Content (Join-Path $izlaz "GOTOVO")
