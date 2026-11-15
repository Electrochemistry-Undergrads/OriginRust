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
  [string]$AfterFile = '',  # the same, one line each, from a file (bash passes -After as one string)
  [switch]$NoDialog,        # no dialog is expected: place the window, then run the After lines at once (mouse and keyboard work on the graph window)
  [int]$WinW = 1600, [int]$WinH = 1000   # size the Origin window is given (it is clipped by a smaller screen)
)
if ($AfterFile -ne '') { $After = @(Get-Content -LiteralPath $AfterFile | Where-Object { $_.Trim() -ne '' }) }
$ErrorActionPreference = 'Stop'
$work = (Resolve-Path $WorkDir).Path.TrimEnd('\') + '\'
$watch = Join-Path $PSScriptRoot 'origin-dialog-watch.ps1'
$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 30 minutes.' }
$origin = $null; $proc = $null
$inputCode = @"
using System;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Text;
namespace Sheaf {
public static class AfterInput {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] public struct CURSORINFO { public int cbSize, flags; public IntPtr hCursor; public POINT pt; }
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool GetCursorInfo(ref CURSORINFO ci);
  [DllImport("user32.dll")] public static extern IntPtr LoadCursor(IntPtr inst, IntPtr name);
  [DllImport("user32.dll")] public static extern bool DrawIconEx(IntPtr hdc, int x, int y, IntPtr hIcon, int w, int h, int step, IntPtr bg, int flags);
  public static void Move(int x, int y) { SetCursorPos(x, y); }
  public static void Down(int button) { mouse_event(button == 1 ? 8u : 2u, 0, 0, 0, UIntPtr.Zero); }
  public static void Up(int button) { mouse_event(button == 1 ? 0x10u : 4u, 0, 0, 0, UIntPtr.Zero); }
  public static void Key(byte vk, bool down) { keybd_event(vk, 0, down ? 0u : 2u, UIntPtr.Zero); }
  public static string CursorName() {
    CURSORINFO ci = new CURSORINFO(); ci.cbSize = Marshal.SizeOf(typeof(CURSORINFO));
    if (!GetCursorInfo(ref ci)) return "GetCursorInfo failed";
    int[] ids = { 32512, 32513, 32514, 32515, 32516, 32642, 32643, 32644, 32645, 32646, 32648, 32649, 32650, 32651 };
    string[] names = { "ARROW", "IBEAM", "WAIT", "CROSS", "UPARROW", "SIZENWSE", "SIZENESW", "SIZEWE", "SIZENS", "SIZEALL", "NO", "HAND", "APPSTARTING", "HELP" };
    for (int i = 0; i < ids.Length; i++) if (LoadCursor(IntPtr.Zero, (IntPtr)ids[i]) == ci.hCursor) return "standard " + names[i];
    return "custom 0x" + ci.hCursor.ToInt64().ToString("X") + " at " + ci.pt.X + "," + ci.pt.Y;
  }
  public static void CursorShot(string path) {
    CURSORINFO ci = new CURSORINFO(); ci.cbSize = Marshal.SizeOf(typeof(CURSORINFO));
    if (!GetCursorInfo(ref ci)) return;
    using (Bitmap b = new Bitmap(64, 64)) {
      using (Graphics g = Graphics.FromImage(b)) { g.Clear(Color.White); IntPtr dc = g.GetHdc(); DrawIconEx(dc, 8, 8, ci.hCursor, 0, 0, 0, IntPtr.Zero, 3); g.ReleaseHdc(dc); }
      b.Save(path, System.Drawing.Imaging.ImageFormat.Png);
    }
  }
}
}
"@
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
  if ($NoDialog) { $helper = $null } else { $helper = Start-Process -FilePath 'powershell.exe' -ArgumentList $helperArgs -WindowStyle Hidden -PassThru }
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
    [void][DlgOpen]::MoveWindow($proc.MainWindowHandle, 0, 0, $WinW, $WinH, $true)
    [void][DlgOpen]::SetWindowPos($proc.MainWindowHandle, [IntPtr]::new(-1), 0, 0, $WinW, $WinH, 0x0040)  # topmost
    [void][DlgOpen]::SetForegroundWindow($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 1500
  }
  if ($null -ne $helper) { [void]$helper.WaitForExit(60000) }
  Add-Type -AssemblyName System.Windows.Forms
  Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition $inputCode -ErrorAction SilentlyContinue
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
    if ($l.StartsWith('@CLICK:')) {
      # "@CLICK:x,y" clicks a screen position (for example the maximize button of the report window); "@WHEEL:x,y,n" scrolls n notches (positive = down); "@WAIT:ms" waits.
      Add-Type -AssemblyName System.Windows.Forms
      $xy = $l.Substring(7) -split ','
      Add-Type -Name AfterMouse -Namespace Sheaf -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);' -ErrorAction SilentlyContinue
      [void][Sheaf.AfterMouse]::SetCursorPos([int]$xy[0], [int]$xy[1]); Start-Sleep -Milliseconds 300
      [Sheaf.AfterMouse]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [Sheaf.AfterMouse]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 800
      continue
    }
    if ($l.StartsWith('@WHEEL:')) {
      $xy = $l.Substring(7) -split ','
      Add-Type -Name AfterMouse -Namespace Sheaf -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);' -ErrorAction SilentlyContinue
      [void][Sheaf.AfterMouse]::SetCursorPos([int]$xy[0], [int]$xy[1]); Start-Sleep -Milliseconds 300
      for ($k = 0; $k -lt [Math]::Abs([int]$xy[2]); $k++) { [Sheaf.AfterMouse]::mouse_event(0x800, 0, 0, [BitConverter]::ToUInt32([BitConverter]::GetBytes([int](-120 * [Math]::Sign([int]$xy[2]))), 0), [UIntPtr]::Zero); Start-Sleep -Milliseconds 150 }
      Start-Sleep -Milliseconds 600; continue
    }
    if ($l.StartsWith('@MOVE:')) { $xy = $l.Substring(6) -split ','; [Sheaf.AfterInput]::Move([int]$xy[0], [int]$xy[1]); Start-Sleep -Milliseconds 500; continue }
    if ($l.StartsWith('@RCLICK:') -or $l.StartsWith('@DCLICK:')) {
      # "@RCLICK:x,y" right click, "@DCLICK:x,y" double click.
      $xy = $l.Substring(8) -split ','; [Sheaf.AfterInput]::Move([int]$xy[0], [int]$xy[1]); Start-Sleep -Milliseconds 300
      $n = 1; $b = 0; if ($l.StartsWith('@RCLICK:')) { $b = 1 } else { $n = 2 }
      for ($k = 0; $k -lt $n; $k++) { [Sheaf.AfterInput]::Down($b); [Sheaf.AfterInput]::Up($b); Start-Sleep -Milliseconds 80 }
      Start-Sleep -Milliseconds 800; continue
    }
    if ($l.StartsWith('@DRAG:')) {
      # "@DRAG:x1,y1,x2,y2[,steps[,hold]]" presses the left button at x1,y1, moves to x2,y2 in steps (default 12), lets go; hold=1 keeps the button
      # down (a later "@UP" lets go, so a screenshot or "@KEY" can be taken in the middle of a drag); "@DOWN:x,y" and "@UP:x,y" are the halves.
      $a = @($l.Substring(6) -split ',' | ForEach-Object { [int]$_ }); $steps = 12; if ($a.Count -gt 4) { $steps = $a[4] }
      [Sheaf.AfterInput]::Move([int]$a[0], [int]$a[1]); Start-Sleep -Milliseconds 400
      [Sheaf.AfterInput]::Down(0); Start-Sleep -Milliseconds 200
      for ($k = 1; $k -le $steps; $k++) { [Sheaf.AfterInput]::Move([int]($a[0] + ($a[2] - $a[0]) * $k / $steps), [int]($a[1] + ($a[3] - $a[1]) * $k / $steps)); Start-Sleep -Milliseconds 60 }
      Start-Sleep -Milliseconds 300
      if (-not ($a.Count -gt 5 -and $a[5] -eq 1)) { [Sheaf.AfterInput]::Up(0); Start-Sleep -Milliseconds 800 }
      continue
    }
    if ($l.StartsWith('@DOWN:')) { $xy = $l.Substring(6) -split ','; [Sheaf.AfterInput]::Move([int]$xy[0], [int]$xy[1]); Start-Sleep -Milliseconds 400; [Sheaf.AfterInput]::Down(0); Start-Sleep -Milliseconds 300; continue }
    if ($l.StartsWith('@UP:')) { $xy = $l.Substring(4) -split ','; [Sheaf.AfterInput]::Move([int]$xy[0], [int]$xy[1]); Start-Sleep -Milliseconds 300; [Sheaf.AfterInput]::Up(0); Start-Sleep -Milliseconds 800; continue }
    if ($l.StartsWith('@KEYDOWN:') -or $l.StartsWith('@KEYUP:')) {
      # "@KEYDOWN:ctrl|shift|alt" holds a modifier (until "@KEYUP:"), for clicks and drags with a modifier.
      $name = $l.Substring($l.IndexOf(':') + 1).Trim().ToLower(); $vk = @{ ctrl = 0x11; shift = 0x10; alt = 0x12 }[$name]
      [Sheaf.AfterInput]::Key([byte]$vk, $l.StartsWith('@KEYDOWN:')); Start-Sleep -Milliseconds 200; continue
    }
    if ($l.StartsWith('@KEY:')) { [System.Windows.Forms.SendKeys]::SendWait($l.Substring(5)); Start-Sleep -Milliseconds 800; continue }  # SendKeys notation
    if ($l.StartsWith('@ECHO:')) { '== ' + $l.Substring(6); continue }  # a label in the output
    if ($l -eq '@CURSOR') { "cursor = " + [Sheaf.AfterInput]::CursorName(); continue }
    if ($l.StartsWith('@CURSORSHOT:')) { [Sheaf.AfterInput]::CursorShot($l.Substring(12)); "saved cursor " + $l.Substring(12); continue }
    if ($l.StartsWith('@SHOTRECT:')) {
      # "@SHOTRECT:path.png,x,y,w,h" saves a picture of this part of the screen.
      Add-Type -AssemblyName System.Drawing
      $a = $l.Substring(10) -split ','; $r = [int[]]$a[1..4]
      $bmp = New-Object System.Drawing.Bitmap($r[2], $r[3]); $g = [System.Drawing.Graphics]::FromImage($bmp)
      $g.CopyFromScreen($r[0], $r[1], 0, 0, (New-Object System.Drawing.Size($r[2], $r[3]))); $g.Dispose()
      $bmp.Save($a[0], [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose(); "saved $($a[0])"; continue
    }
    if ($l.StartsWith('@HOVER:')) {
      # "@HOVER:x,y" moves the pointer to x,y in small steps (so the program sees mouse moves and shows tips) and waits 2 s.
      $xy = [int[]]($l.Substring(7) -split ','); [Sheaf.AfterInput]::Move($xy[0] - 12, $xy[1] - 8); Start-Sleep -Milliseconds 100
      for ($k = 1; $k -le 6; $k++) { [Sheaf.AfterInput]::Move($xy[0] - 12 + 2 * $k, $xy[1] - 8 + [int](1.33 * $k)); Start-Sleep -Milliseconds 80 }
      Start-Sleep -Milliseconds 2000; continue
    }
    if ($l.StartsWith('@PATH:')) {
      # "@PATH:x1,y1;x2,y2;x3,y3..." presses the left button at the first point, moves through the others (8 small steps per leg), lets go (a freehand drag).
      $pts = @($l.Substring(6) -split ';' | ForEach-Object { , ([int[]]($_ -split ',')) })
      [Sheaf.AfterInput]::Move($pts[0][0], $pts[0][1]); Start-Sleep -Milliseconds 400; [Sheaf.AfterInput]::Down(0); Start-Sleep -Milliseconds 200
      for ($i = 1; $i -lt $pts.Count; $i++) { for ($k = 1; $k -le 8; $k++) { [Sheaf.AfterInput]::Move([int]($pts[$i-1][0] + ($pts[$i][0] - $pts[$i-1][0]) * $k / 8), [int]($pts[$i-1][1] + ($pts[$i][1] - $pts[$i-1][1]) * $k / 8)); Start-Sleep -Milliseconds 40 } }
      Start-Sleep -Milliseconds 300; [Sheaf.AfterInput]::Up(0); Start-Sleep -Milliseconds 800; continue
    }
    if ($l.StartsWith('@WAIT:')) { Start-Sleep -Milliseconds ([int]$l.Substring(6)); continue }
    if ($l.StartsWith('@SHOT:')) {
      # "@SHOT:path.png" saves a picture of the whole primary screen (the report sheet that is on top).
      Add-Type -AssemblyName System.Drawing; Add-Type -AssemblyName System.Windows.Forms
      $sb = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
      $bmp = New-Object System.Drawing.Bitmap($sb.Width, $sb.Height)
      $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($sb.Location, [System.Drawing.Point]::Empty, $sb.Size); $g.Dispose()
      $bmp.Save($l.Substring(6), [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose(); "saved $($l.Substring(6))"
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
