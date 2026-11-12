<#
.SYNOPSIS
  Starts a temporary, visible Origin instance, runs a LabTalk setup script of ours, and saves
  pictures of its main window: one of the plain window, and one per key sequence (for example
  an opened menu).

.DESCRIPTION
  Used to learn the layout of Origin's interface (which menus, toolbars and panels exist and
  where they sit) so that our own interface can follow the same workflow. The pictures show
  OriginLab's interface and therefore stay in the local, git-ignored scratch folder; only our
  own written description of what they show goes into docs/spec/.

  Takes the same system-wide lock as run-labtalk.ps1, so it never runs beside another Origin
  instance. Nothing is saved in Origin; the instance is closed at the end.

  -Steps is a list of "name=keys" items. "keys" uses the SendKeys notation (% is Alt,
  ^ is Ctrl, + is Shift, {DOWN}, {RIGHT}, {ENTER}, {ESC}, ...). After the picture of a step
  is taken, Escape is sent four times to close any open menu before the next step.
  An item "name=" (no keys) just takes a picture.

.EXAMPLE
  .\origin-screenshot.ps1 -Setup gui-setup.labtalk.txt -OutDir C:\tmp\shots -Steps 'plain=','file=%f','analysis=%a'

  A step list passed through "powershell -File" with commas arrives as ONE string, so only the
  first step would run. Use -StepsFile (one "name=keys" step per line) instead; it also lets a
  step hold commas in mouse positions. The actions of a step are separated by "|": SendKeys
  text, @L:x,y left click, @R:x,y right click, @D:x,y double click, @M:x,y move, @W:ms wait and
  @X:text, which runs LabTalk text in Origin (for example @X:win -a Graph1; picks a window).
#>
param(
  [string]$Setup = '',
  [Parameter(Mandatory = $true)][string]$OutDir,
  [string[]]$Steps = @('plain='),
  [int]$SettleMs = 1500,
  [int]$Width = 1600,
  [int]$Height = 1000,
  [string]$StepsFile = '',  # one "name=keys" step per line (blank lines and lines starting with # are skipped)
  [switch]$FullScreen   # save the whole primary screen, so dialogs outside the main window show
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class SheafWin {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hgt, bool repaint);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
}
"@
[void][SheafWin]::SetProcessDPIAware()
# Passing -Steps through "powershell -File" with commas delivers ONE string, so only the first
# step would be run. -StepsFile avoids that (and lets a step hold commas in mouse positions).
if ($StepsFile -ne '') {
  $Steps = @(Get-Content -LiteralPath $StepsFile | Where-Object { $_.Trim() -ne '' -and -not $_.TrimStart().StartsWith('#') })
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$out = (Resolve-Path $OutDir).Path

# A step's keys may hold several actions separated by "|". An action starting with "@" is a mouse
# action at a screen position x,y (the window is placed at 0,0): @L:x,y left click, @R:x,y right
# click, @D:x,y double click, @M:x,y only move, @W:ms wait, @X:text runs LabTalk text in Origin. Anything else is sent as SendKeys.
function Invoke-Action([string]$a) {
  if ($a.StartsWith('@')) {
    $kind = $a.Substring(1, 1)
    $arg = $a.Substring(3)
    if ($kind -eq 'X') { [void]$origin.Execute($arg); Start-Sleep -Milliseconds 500; return }  # run LabTalk
    if ($kind -eq 'W') { Start-Sleep -Milliseconds ([int]$arg); return }
    $xy = $arg -split ','
    [void][SheafWin]::SetCursorPos([int]$xy[0], [int]$xy[1])
    Start-Sleep -Milliseconds 150
    $down = 0x0002; $up = 0x0004
    if ($kind -eq 'R') { $down = 0x0008; $up = 0x0010 }
    $n = 1
    if ($kind -eq 'D') { $n = 2 }
    if ($kind -ne 'M') {
      for ($i = 0; $i -lt $n; $i++) {
        [SheafWin]::mouse_event($down, 0, 0, 0, [UIntPtr]::Zero)
        [SheafWin]::mouse_event($up, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 80
      }
    }
    Start-Sleep -Milliseconds 500
  } else {
    [System.Windows.Forms.SendKeys]::SendWait($a)
    Start-Sleep -Milliseconds 400
  }
}

function Save-Window([IntPtr]$hwnd, [string]$file) {
  $r = New-Object SheafWin+RECT
  [void][SheafWin]::GetWindowRect($hwnd, [ref]$r)
  if ($FullScreen) {
    $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $r.Left = $b.Left; $r.Top = $b.Top; $r.Right = $b.Right; $r.Bottom = $b.Bottom
  }
  $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $g.Dispose()
  $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "saved $file ($w x $h)"
}

$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 30 minutes.' }
$origin = $null
$proc = $null
try {
  $before = @(Get-Process | ForEach-Object { $_.Id })
  $origin = New-Object -ComObject Origin.Application
  $origin.Visible = 1
  Start-Sleep -Milliseconds 3000
  if ($Setup -ne '') {
    $text = Get-Content -Raw -LiteralPath $Setup
    "execute=" + $origin.Execute($text)
  }
  $proc = Get-Process | Where-Object { $before -notcontains $_.Id -and $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -like 'Origin*' } | Select-Object -First 1
  if ($null -eq $proc) { throw 'The Origin window was not found.' }
  $hwnd = $proc.MainWindowHandle
  [void][SheafWin]::ShowWindow($hwnd, 1)
  [void][SheafWin]::MoveWindow($hwnd, 0, 0, $Width, $Height, $true)
  [void][SheafWin]::SetWindowPos($hwnd, [IntPtr]::new(-1), 0, 0, $Width, $Height, 0x0040)  # topmost, so other windows do not cover it
  [void][SheafWin]::SetForegroundWindow($hwnd)
  Start-Sleep -Milliseconds $SettleMs
  foreach ($step in $Steps) {
    $name, $keys = $step -split '=', 2
    [void][SheafWin]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 300
    if ($keys) {
      foreach ($a in ($keys -split '\|')) { Invoke-Action $a }
      Start-Sleep -Milliseconds $SettleMs
    }
    Save-Window $hwnd (Join-Path $out "$name.png")
    if ($keys) {
      [System.Windows.Forms.SendKeys]::SendWait('{ESC}{ESC}{ESC}{ESC}')
      Start-Sleep -Milliseconds 400
    }
  }
}
finally {
  if ($null -ne $origin) {
    try { [void]$origin.Execute('doc -s; doc -n;') } catch { }
    try { [void]$origin.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)
    Start-Sleep -Seconds 2
    # Only the instance this script started (by its process id) is stopped, if it is still there.
    if ($null -ne $proc) { Get-Process -Id $proc.Id -ErrorAction SilentlyContinue | Stop-Process -Force }
  }
  $lock.ReleaseMutex()
  $lock.Dispose()
}
