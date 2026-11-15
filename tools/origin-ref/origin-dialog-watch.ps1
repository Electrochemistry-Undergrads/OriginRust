<#
.SYNOPSIS
  Helper of origin-dialog.ps1: waits for a dialog window of a given Origin process, saves a
  picture of it, lists its controls (class, text, position, tick state) and plays a list of
  steps (clicks and keys relative to the dialog), then closes the dialog.
#>
param(
  [Parameter(Mandatory = $true)][int]$OriginPid,
  [Parameter(Mandatory = $true)][string]$OutDir,
  [string]$StepsFile = '',
  [string]$DoubleClick = '',   # screen position "x,y" to double-click until a dialog appears (up to 4 tries)
  [string]$MenuClicks = '',    # screen positions "x,y;x,y;..." clicked once, in order (a menu path), to open the dialog
  [int]$WaitSeconds = 40,
  [int]$SettleMs = 800
)
$ErrorActionPreference = 'Continue'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class DlgWin {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, StringBuilder l);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, string l);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  public static List<IntPtr> Tops(uint pid) {
    var r = new List<IntPtr>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) r.Add(h); return true; }, IntPtr.Zero);
    return r;
  }
  public static List<IntPtr> Children(IntPtr h) {
    var r = new List<IntPtr>();
    EnumChildWindows(h, (c, l) => { r.Add(c); return true; }, IntPtr.Zero);
    return r;
  }
  public static string ComboItems(IntPtr h) {
    long n = SendMessage(h, 0x146, IntPtr.Zero, IntPtr.Zero).ToInt64();
    long cur = SendMessage(h, 0x147, IntPtr.Zero, IntPtr.Zero).ToInt64();
    var parts = new List<string>();
    for (int i = 0; i < n && i < 200; i++) {
      var sb = new StringBuilder(256);
      SendMessage(h, 0x148, (IntPtr)i, sb);
      parts.Add((i == cur ? "*" : "") + sb.ToString());
    }
    return "n=" + n + " [" + string.Join(" | ", parts) + "]";
  }
  // Picks item `item` of the n-th visible combo box of the dialog and tells the parent (CBN_SELCHANGE).
  public static string PickCombo(IntPtr dlg, int n, int item) {
    int k = 0;
    foreach (var c in Children(dlg)) {
      if (Cls(c) != "ComboBox" || !IsWindowVisible(c)) continue;
      if (k++ != n) continue;
      SendMessage(c, 0x14E, (IntPtr)item, IntPtr.Zero);
      int id = GetDlgCtrlID(c);
      IntPtr parent = GetParent(c);
      SendMessage(parent, 0x111, (IntPtr)((1 << 16) | (id & 0xFFFF)), c);
      return "picked combo " + n + " item " + item + " id " + id;
    }
    return "combo " + n + " not found";
  }
  // Sets the text of the n-th visible edit box (WM_SETTEXT, then EN_CHANGE to the parent).
  public static string SetEdit(IntPtr dlg, int n, string text) {
    int k = 0;
    foreach (var c in Children(dlg)) {
      if (Cls(c) != "Edit" || !IsWindowVisible(c)) continue;
      if (k++ != n) continue;
      SendMessage(c, 0x000C, IntPtr.Zero, text);
      SendMessage(GetParent(c), 0x111, (IntPtr)((0x300 << 16) | (GetDlgCtrlID(c) & 0xFFFF)), c);
      return "set edit " + n;
    }
    return "edit " + n + " not found";
  }
  // Clicks the visible button with this text (BM_CLICK).
  public static string ClickButton(IntPtr dlg, string text) {
    foreach (var c in Children(dlg)) {
      if (Cls(c) != "Button" || !IsWindowVisible(c) || Text(c) != text) continue;
      PostMessage(c, 0x00F5, IntPtr.Zero, IntPtr.Zero);
      return "clicked " + text;
    }
    return "button " + text + " not found";
  }
  // WM_GETTEXT, which (unlike GetWindowText) also reads the contents of another process's edit boxes.
  public static string Text(IntPtr h) { var s = new StringBuilder(512); SendMessage(h, 0x000D, (IntPtr)512, s); return s.ToString(); }
  public static string Cls(IntPtr h) { var s = new StringBuilder(128); GetClassName(h, s, 128); return s.ToString(); }
}
"@
[void][DlgWin]::SetProcessDPIAware()
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$out = (Resolve-Path $OutDir).Path

function Find-Dialog {
  foreach ($h in [DlgWin]::Tops([uint32]$OriginPid)) {
    $t = [DlgWin]::Text($h); $c = [DlgWin]::Cls($h)
    # Origin's tips ("Hint Message") open beside dialogs and are not the dialog wanted.
    if ($t -ne '' -and $t -notlike 'Origin*' -and $t -ne 'Hint Message' -and $c -ne 'ORIGIN_MAINFRAME') { return $h }
  }
  return [IntPtr]::Zero
}
function Save-Rect($rect, [string]$file) {
  $w = $rect.Right - $rect.Left; $h = $rect.Bottom - $rect.Top
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $g.Dispose(); $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
}
function Dump-Controls([IntPtr]$dlg, [string]$file) {
  $d = New-Object DlgWin+RECT; [void][DlgWin]::GetWindowRect($dlg, [ref]$d)
  $lines = @("dialog title=" + [DlgWin]::Text($dlg) + " class=" + [DlgWin]::Cls($dlg) + " rect=$($d.Left),$($d.Top),$($d.Right),$($d.Bottom)")
  foreach ($c in [DlgWin]::Children($dlg)) {
    $r = New-Object DlgWin+RECT; [void][DlgWin]::GetWindowRect($c, [ref]$r)
    $vis = [DlgWin]::IsWindowVisible($c)
    $cls = [DlgWin]::Cls($c)
    $chk = ''
    if ($cls -eq 'Button') { $chk = ' check=' + [DlgWin]::SendMessage($c, 0x00F0, [IntPtr]::Zero, [IntPtr]::Zero).ToInt64() }
    if ($cls -eq 'ComboBox') { $chk = ' items=' + [DlgWin]::ComboItems($c) }
    $lines += ("{0} vis={1} [{2},{3},{4},{5}]{6} text='{7}'" -f $cls, $vis, ($r.Left - $d.Left), ($r.Top - $d.Top), ($r.Right - $d.Left), ($r.Bottom - $d.Top), $chk, [DlgWin]::Text($c))
  }
  $lines | Set-Content -LiteralPath $file -Encoding UTF8
}
function Click-At([IntPtr]$dlg, [string]$kind, [int]$x, [int]$y) {
  $d = New-Object DlgWin+RECT; [void][DlgWin]::GetWindowRect($dlg, [ref]$d)
  [void][DlgWin]::SetCursorPos($d.Left + $x, $d.Top + $y); Start-Sleep -Milliseconds 150
  $down = 0x0002; $up = 0x0004
  if ($kind -eq 'R') { $down = 0x0008; $up = 0x0010 }
  $n = 1; if ($kind -eq 'D') { $n = 2 }
  if ($kind -eq 'S') {
    $notches = [int]$script:WheelNotches
    for ($i = 0; $i -lt [Math]::Abs($notches); $i++) {
      $delta = if ($notches -gt 0) { [uint32]([uint32]::MaxValue - 119) } else { [uint32]120 }  # wheel down is negative
      [DlgWin]::mouse_event(0x0800, 0, 0, $delta, [UIntPtr]::Zero); Start-Sleep -Milliseconds 120
    }
  }
  if ($kind -ne 'M' -and $kind -ne 'S') { for ($i = 0; $i -lt $n; $i++) { [DlgWin]::mouse_event($down, 0, 0, 0, [UIntPtr]::Zero); [DlgWin]::mouse_event($up, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 80 } }
  Start-Sleep -Milliseconds 400
}

$deadline = (Get-Date).AddSeconds($WaitSeconds)
$dlg = [IntPtr]::Zero
$tries = 0
$nextClick = (Get-Date).AddSeconds(4)
if ($MenuClicks -ne '') {
  Start-Sleep -Milliseconds 4000
  foreach ($p in ($MenuClicks -split ';')) {
    # "K:keys" sends keys (SendKeys notation, for example K:^3 for Ctrl+3) instead of a click.
    if ($p.StartsWith('K:')) { [System.Windows.Forms.SendKeys]::SendWait($p.Substring(2)); Start-Sleep -Milliseconds 700; continue }
    # "G:x1,y1,x2,y2" drags with the left button held; "DC:x,y" double-clicks.
    if ($p.StartsWith('G:')) {
      $g = $p.Substring(2) -split ','
      [void][DlgWin]::SetCursorPos([int]$g[0], [int]$g[1]); Start-Sleep -Milliseconds 300
      [DlgWin]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 200
      for ($k = 1; $k -le 10; $k++) {
        $mx = [int]([int]$g[0] + ([int]$g[2] - [int]$g[0]) * $k / 10); $my = [int]([int]$g[1] + ([int]$g[3] - [int]$g[1]) * $k / 10)
        [void][DlgWin]::SetCursorPos($mx, $my); Start-Sleep -Milliseconds 40
      }
      [DlgWin]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 700; continue
    }
    if ($p.StartsWith('DC:')) {
      $d2 = $p.Substring(3) -split ','
      [void][DlgWin]::SetCursorPos([int]$d2[0], [int]$d2[1]); Start-Sleep -Milliseconds 300
      for ($k = 0; $k -lt 2; $k++) { [DlgWin]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [DlgWin]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 80 }
      Start-Sleep -Milliseconds 700; continue
    }
    $xy = $p -split ','
    [void][DlgWin]::SetCursorPos([int]$xy[0], [int]$xy[1]); Start-Sleep -Milliseconds 300
    [DlgWin]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [DlgWin]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 700
  }
}
while ((Get-Date) -lt $deadline) {
  $dlg = Find-Dialog; if ($dlg -ne [IntPtr]::Zero) { break }
  if ($DoubleClick -ne '' -and $tries -lt 4 -and (Get-Date) -gt $nextClick) {
    $xy = $DoubleClick -split ','
    [void][DlgWin]::SetCursorPos([int]$xy[0], [int]$xy[1]); Start-Sleep -Milliseconds 200
    for ($i = 0; $i -lt 2; $i++) { [DlgWin]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [DlgWin]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 80 }
    $tries++; $nextClick = (Get-Date).AddSeconds(6)
  }
  Start-Sleep -Milliseconds 300
}
if ($dlg -eq [IntPtr]::Zero) { 'no dialog found' | Set-Content (Join-Path $out 'log.txt'); exit 1 }
Start-Sleep -Milliseconds 1500
[void][DlgWin]::SetWindowPos($dlg, [IntPtr]::new(-1), 0, 0, 0, 0, 0x0003)  # topmost, keep position and size
[void][DlgWin]::SetForegroundWindow($dlg)
Start-Sleep -Milliseconds 500
$d = New-Object DlgWin+RECT; [void][DlgWin]::GetWindowRect($dlg, [ref]$d)
Save-Rect $d (Join-Path $out 'first.png')
Dump-Controls $dlg (Join-Path $out 'first.controls.txt')
if ($StepsFile -ne '') {
  $steps = @(Get-Content -LiteralPath $StepsFile | Where-Object { $_.Trim() -ne '' -and -not $_.TrimStart().StartsWith('#') })
  foreach ($step in $steps) {
    $name, $acts = $step -split '=', 2
    # A button can close the dialog and open another in its place (Layer Contents > Plot Setup...):
    # the steps then go to the new one.
    if (-not [DlgWin]::IsWindow($dlg)) {
      for ($k = 0; $k -lt 20 -and -not [DlgWin]::IsWindow($dlg); $k++) { $n = Find-Dialog; if ($n -ne [IntPtr]::Zero) { $dlg = $n } else { Start-Sleep -Milliseconds 300 } }
      [void][DlgWin]::SetWindowPos($dlg, [IntPtr]::new(-1), 0, 0, 0, 0, 0x0003)
    }
    foreach ($a in ($acts -split '\|')) {
      if ($a.StartsWith('@')) {
        $kind = $a.Substring(1, 1); $arg = $a.Substring(3)
        if ($kind -eq 'W') { Start-Sleep -Milliseconds ([int]$arg); continue }
        if ($kind -eq 'T') { $ti = $arg -split ',', 2; $m = [DlgWin]::SetEdit($dlg, [int]$ti[0], $ti[1]); "$name $m" | Add-Content (Join-Path $out 'actions.txt'); Start-Sleep -Milliseconds 500; continue }
        if ($kind -eq 'B') { $m = [DlgWin]::ClickButton($dlg, $arg); "$name $m" | Add-Content (Join-Path $out 'actions.txt'); Start-Sleep -Milliseconds 800; continue }
        if ($kind -eq 'C') { $ci = $arg -split ','; $m = [DlgWin]::PickCombo($dlg, [int]$ci[0], [int]$ci[1]); "$name $m" | Add-Content (Join-Path $out 'actions.txt'); Start-Sleep -Milliseconds 500; continue }
        $xy = $arg -split ','
        if ($kind -eq 'S') { $script:WheelNotches = [int]$xy[2] }
        Click-At $dlg $kind ([int]$xy[0]) ([int]$xy[1])
      } elseif ($a -ne '') {
        [System.Windows.Forms.SendKeys]::SendWait($a); Start-Sleep -Milliseconds 400
      }
    }
    Start-Sleep -Milliseconds $SettleMs
    $cur = Find-Dialog
    if ($cur -eq [IntPtr]::Zero) { "dialog gone after step $name" | Add-Content (Join-Path $out 'log.txt'); break }
    $dlg = $cur
    $d = New-Object DlgWin+RECT; [void][DlgWin]::GetWindowRect($dlg, [ref]$d)
    Save-Rect $d (Join-Path $out "$name.png")
    Dump-Controls $dlg (Join-Path $out "$name.controls.txt")
  }
}
# close the dialog (Cancel through Escape, then the close message if it is still there)
[void][DlgWin]::SetForegroundWindow($dlg)
[System.Windows.Forms.SendKeys]::SendWait('{ESC}')
Start-Sleep -Milliseconds 700
$left = Find-Dialog
if ($left -ne [IntPtr]::Zero) { [void][DlgWin]::PostMessage($left, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero); Start-Sleep -Milliseconds 700 }
'done' | Add-Content (Join-Path $out 'log.txt')
