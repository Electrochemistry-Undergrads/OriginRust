<#
.SYNOPSIS
  Runs a LabTalk script in a hidden, temporary Origin instance through COM
  automation and prints the values of the variables you ask for.

.DESCRIPTION
  This is how reference numbers for the numeric parity tests are produced:
  feed Origin an input we generated ourselves, let it run one of its analysis
  tools, and read the results back. Origin is started, used and closed again;
  nothing in the Origin installation or the user's Origin settings is changed.

  In the script text, %WORK% is replaced by the absolute path of -WorkDir
  (with a trailing backslash), so a script can read inputs from and write
  outputs to that folder.

.EXAMPLE
  .\run-labtalk.ps1 -Script integ.labtalk.txt -WorkDir C:\tmp\ref -Vars area,x0

.NOTES
  Works only on a computer with OriginLab Origin installed and registered
  for COM automation (class "Origin.Application").
#>
param(
  [Parameter(Mandatory = $true)][string]$Script,
  [string]$WorkDir = (Get-Location).Path,
  [string[]]$Vars = @(),
  [string[]]$StrVars = @()
)
$ErrorActionPreference = 'Stop'
$work = (Resolve-Path $WorkDir).Path.TrimEnd('\') + '\'
$text = (Get-Content -Raw -LiteralPath $Script).Replace('%WORK%', $work)
# Only one Origin instance may run at a time. A named lock makes a second copy of this
# script wait (up to 30 minutes) until the first one has closed Origin.
$lock = New-Object System.Threading.Mutex($false, 'Global\SheafOriginReference')
try { $held = $lock.WaitOne([TimeSpan]::FromMinutes(30)) } catch [System.Threading.AbandonedMutexException] { $held = $true }
if (-not $held) { throw 'Another Origin reference run held the lock for more than 30 minutes.' }
$origin = $null
try {
  $origin = New-Object -ComObject Origin.Application
  $ok = $origin.Execute($text)
  "execute=$ok"
  foreach ($v in $Vars) {
    $value = [double]$origin.LTVar($v)
    "$v=" + $value.ToString('R', [Globalization.CultureInfo]::InvariantCulture)
  }
  foreach ($v in $StrVars) {
    "$v=" + $origin.LTStr($v)
  }
}
finally {
  if ($null -ne $origin) {
    try { [void]$origin.Exit() } catch { "warning: Origin did not exit cleanly: $_" }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)
    # Give the Origin process a moment to end before the next run starts one.
    Start-Sleep -Seconds 2
  }
  $lock.ReleaseMutex()
  $lock.Dispose()
}
