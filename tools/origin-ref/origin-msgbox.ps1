<#
.SYNOPSIS
  Lists the visible windows (message boxes) of the newest Origin process with their texts, and with
  -Click presses the button of that text in them. A hidden Origin started by run-labtalk-lines.ps1 can
  stop on a question (for example "Would you like your data to be normalized?"); this shows it and
  answers it so the run can go on.
#>
param([string]$Click = '')
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class OMsg {
  public delegate bool P(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(P p, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, P p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  public static string Text(IntPtr h) { var s = new StringBuilder(1024); GetWindowText(h, s, 1024); return s.ToString(); }
  public static List<IntPtr> Tops(uint pid) { var r = new List<IntPtr>(); EnumWindows((h, l) => { uint q; GetWindowThreadProcessId(h, out q); if (q == pid && IsWindowVisible(h)) r.Add(h); return true; }, IntPtr.Zero); return r; }
  public static List<IntPtr> Kids(IntPtr d) { var r = new List<IntPtr>(); EnumChildWindows(d, (h, l) => { r.Add(h); return true; }, IntPtr.Zero); return r; }
}
"@
$p = Get-Process | Where-Object { $_.ProcessName -like 'ORIGIN*' } | Sort-Object StartTime | Select-Object -Last 1
if ($null -eq $p) { 'no Origin running'; exit }
"Origin $($p.Id) started $($p.StartTime)"
foreach ($w in [OMsg]::Tops([uint32]$p.Id)) {
  "window: " + [OMsg]::Text($w)
  foreach ($k in [OMsg]::Kids($w)) {
    $t = [OMsg]::Text($k)
    if ($t -ne '') { "  " + $t }
    if ($Click -ne '' -and $t -eq $Click) { [void][OMsg]::PostMessage($k, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero); "  -> clicked $Click" }
  }
}
