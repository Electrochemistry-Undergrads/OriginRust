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
#>
param(
  [string]$Setup = '',
  [Parameter(Mandatory = $true)][string]$OutDir,
  [string[]]$Steps = @('plain='),
  [int]$SettleMs = 1500,
  [int]$Width = 1600,
  [int]$Height = 1000
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
}
"@
[void][SheafWin]::SetProcessDPIAware()
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$out = (Resolve-Path $OutDir).Path

function Save-Window([IntPtr]$hwnd, [string]$file) {
  $r = New-Object SheafWin+RECT
  [void][SheafWin]::GetWindowRect($hwnd, [ref]$r)
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
try {
  $before = @(Get-Process -Name 'Origin93_64' -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
  $origin = New-Object -ComObject Origin.Application
  $origin.Visible = 1
  Start-Sleep -Milliseconds 3000
  if ($Setup -ne '') {
    $text = Get-Content -Raw -LiteralPath $Setup
    "execute=" + $origin.Execute($text)
  }
  $proc = Get-Process -Name 'Origin93_64' | Where-Object { $before -notcontains $_.Id -and $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  if ($null -eq $proc) { throw 'The Origin window was not found.' }
  $hwnd = $proc.MainWindowHandle
  [void][SheafWin]::ShowWindow($hwnd, 1)
  [void][SheafWin]::MoveWindow($hwnd, 0, 0, $Width, $Height, $true)
  [void][SheafWin]::SetForegroundWindow($hwnd)
  Start-Sleep -Milliseconds $SettleMs
  foreach ($step in $Steps) {
    $name, $keys = $step -split '=', 2
    [void][SheafWin]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 300
    if ($keys) {
      [System.Windows.Forms.SendKeys]::SendWait($keys)
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
  }
  $lock.ReleaseMutex()
  $lock.Dispose()
}
