<#
.SYNOPSIS
  Opens one of Origin's own dialogs from a LabTalk command (for example "impasc -d;") in a
  temporary visible Origin instance, saves a picture of the dialog and a list of its controls
  (and, with -StepsFile, pictures after clicks), and closes everything without saving.

.DESCRIPTION
  The pictures show OriginLab's interface and stay in the local git-ignored scratch folder; only
  our own written description goes into docs/spec. The command blocks while the dialog is open,
  so a helper process (origin-dialog-watch.ps1) looks at the dialog and closes it with Escape.
  Takes the same system-wide lock as the other scripts. Steps use the notation of
  origin-dialog-watch.ps1: name=actions, actions separated by "|": SendKeys text, @L:x,y left
  click, @D:x,y double click, @S:x,y,n mouse wheel (n notches, positive = down), @C:i,j pick item j of the i-th visible combo box, @T:i,text set the i-th visible edit box, @B:text click the button, @W:ms wait; x,y are pixels from the top-left corner of the dialog.
#>
param(
  [string]$Setup = '',
  [string]$Command = '',
  [string]$MenuClicks = '',    # instead of -Command: click these screen positions "x,y;x,y" in order (a menu path) in the Origin window placed at 0,0
  [string]$DoubleClick = '',   # instead of -Command: double-click this screen position "x,y" in the Origin window placed at 0,0 (for example a column header)
  [Parameter(Mandatory = $true)][string]$OutDir,
  [string]$StepsFile = '',
  [string]$WorkDir = '.',
  [string[]]$After = @(),  # LabTalk lines run after the dialog closed; "$name$" lines print a string, "?name" a number
  [string]$AfterFile = ''  # the same, one line each, from a file (bash passes -After as one string)
)
if ($AfterFile -ne '') { $After = @(Get-Content -LiteralPath $AfterFile | Where-Object { $_.Trim() -ne '' }) }
$ErrorActionPreference = 'Stop'
$work = (Resolve-Path $WorkDir).Path.TrimEnd('\') + '\'
$watch = Join-Path $PSScriptRoot 'origin-dialog-watch.ps1'
$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 30 minutes.' }
$origin = $null; $proc = $null
try {
  $before = @(Get-Process | ForEach-Object { $_.Id })
  $origin = New-Object -ComObject Origin.Application
  $origin.Visible = 1
  for ($i = 0; $i -lt 40 -and $null -eq $proc; $i++) {
    Start-Sleep -Milliseconds 500
    $proc = Get-Process | Where-Object { $before -notcontains $_.Id -and $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -like 'Origin*' } | Select-Object -First 1
  }
  if ($null -eq $proc) { throw 'The Origin window was not found.' }
  if ($Setup -ne '') { "setup=" + $origin.Execute((Get-Content -Raw -LiteralPath $Setup).Replace('%WORK%', $work)) }
  $helperArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $watch, '-OriginPid', $proc.Id, '-OutDir', $OutDir)
  if ($DoubleClick -ne '') { $helperArgs += @('-DoubleClick', $DoubleClick) }
  if ($MenuClicks -ne '') { $helperArgs += @('-MenuClicks', $MenuClicks) }
  if ($StepsFile -ne '') { $helperArgs += @('-StepsFile', (Resolve-Path $StepsFile).Path) }
  $helper = Start-Process -FilePath 'powershell.exe' -ArgumentList $helperArgs -WindowStyle Hidden -PassThru
  if ($Command -ne '') {
    "command=" + $origin.Execute($Command.Replace('%WORK%', $work))
  } else {
    Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class DlgOpen {
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hgt, bool repaint);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
}
"@
    [void][DlgOpen]::SetProcessDPIAware()
    [void][DlgOpen]::ShowWindow($proc.MainWindowHandle, 1)
    [void][DlgOpen]::MoveWindow($proc.MainWindowHandle, 0, 0, 1600, 1000, $true)
    [void][DlgOpen]::SetWindowPos($proc.MainWindowHandle, [IntPtr]::new(-1), 0, 0, 1600, 1000, 0x0040)  # topmost
    [void][DlgOpen]::SetForegroundWindow($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 1500
  }
  [void]$helper.WaitForExit(60000)
  foreach ($l in $After) {
    if ($l.StartsWith('#')) {
      # "#[Book]Sheet" prints the sheet's cells, as run-labtalk-lines.ps1 does.
      $sh = $l.Substring(1).Trim(); $inv = [Globalization.CultureInfo]::InvariantCulture; $t = [System.__ComObject]
      try { $r = $t.InvokeMember('GetWorksheet', [Reflection.BindingFlags]::InvokeMethod, $null, $origin, @($sh)) } catch { "#ERR $sh $_"; continue }
      if ($r -is [Array]) {
        $rows = $r.GetLength(0); $cols = $r.GetLength(1); "#SHEET $sh $rows $cols"; $l0 = $r.GetLowerBound(0); $l1 = $r.GetLowerBound(1)
        for ($i = $l0; $i -lt $l0 + $rows; $i++) { $line = for ($j = $l1; $j -lt $l1 + $cols; $j++) { $c = $r[$i, $j]; if ($c -is [DBNull] -or $null -eq $c) { '' } elseif ($c -is [double]) { $c.ToString('R', $inv) } else { [string]$c } }; $line -join "`t" }
      } else { "#NOARRAY $sh" }
      continue
    }
    if ($l.StartsWith('?')) { $v = $l.Substring(1); "$v = " + $origin.LTVar($v) }
    elseif ($l.StartsWith('$')) { $v = $l.Substring(1); "$v = " + $origin.LTStr($v) }
    else { "[" + $origin.Execute($l) + "] $l" }
  }
}
finally {
  if ($null -ne $origin) {
    try { [void]$origin.Execute('doc -s; doc -n;') } catch { }
    try { [void]$origin.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)
    Start-Sleep -Seconds 2
    # Only the instance this script started (a process that was not there before) is stopped.
    Get-Process | Where-Object { $before -notcontains $_.Id -and $_.ProcessName -like 'ORIGIN*' } | Stop-Process -Force -ErrorAction SilentlyContinue
  }
  $lock.ReleaseMutex(); $lock.Dispose()
}
