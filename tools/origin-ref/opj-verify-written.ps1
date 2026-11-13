<#
.SYNOPSIS
  Opens project files written by Sheaf in a hidden Origin and compares every workbook, sheet,
  column label and cell that Origin reports with what our own reader reports for the same file.

.DESCRIPTION
  For each file:
    1. our reader's listing:  opj_dump FILE --cells --all-rows N   (the example of crates/io)
    2. Origin's listing:      opj-dump-cells.ps1 (loads the file in a hidden Origin through COM, lists
                              the windows, sheets, short and long names, units, comments, roles and
                              the cells, never saves)
    3. opj-compare-cells.py compares the two.
  A file that Origin cannot load shows up as an empty or incomplete Origin listing and a failed
  comparison. The Origin run takes the system-wide lock Global\SheafOriginReference (inside
  opj-dump-cells.ps1), is killed by a watchdog after -TimeoutSec seconds (a modal dialog in a hidden
  Origin would otherwise hang it) and only ever closes the Origin instance it started.

  -DumpExe is the built example: `cargo build -p sheaf-io --example opj_dump`, then
  target\debug\examples\opj_dump.exe (under your CARGO_TARGET_DIR).

  The exit status is the number of files that did not compare equal.

.EXAMPLE
  .\opj-verify-written.ps1 -Files kinds.opj,wide.opj -DumpExe C:\target\debug\examples\opj_dump.exe -OutDir C:\scratch\verify
#>
param(
  [Parameter(Mandatory = $true)][string[]]$Files,
  [Parameter(Mandatory = $true)][string]$DumpExe,
  [Parameter(Mandatory = $true)][string]$OutDir,
  # Columns with at most this many used rows are compared cell by cell; longer ones are sampled.
  [int]$AllRows = 250,
  [int]$TimeoutSec = 900
)
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$failed = 0
foreach ($file in $Files) {
  $file = (Resolve-Path -LiteralPath $file).Path
  $name = [IO.Path]::GetFileNameWithoutExtension($file)
  $ours = Join-Path $OutDir "$name.ours.txt"
  $theirs = Join-Path $OutDir "$name.origin.txt"
  $details = Join-Path $OutDir "$name.diff.txt"
  # Start-Process with a redirect keeps the UTF-8 bytes of the listing as they are.
  Start-Process -FilePath $DumpExe -ArgumentList @("`"$file`"", '--cells', '--all-rows', $AllRows) `
    -RedirectStandardOutput $ours -Wait -NoNewWindow
  if (Test-Path -LiteralPath $theirs) { Remove-Item -LiteralPath $theirs }
  try {
    & (Join-Path $here 'opj-dump-cells.ps1') -Project $file -Out $theirs -AllRowsLimit $AllRows -TimeoutSec $TimeoutSec
  } catch {
    "$name : Origin run failed: $($_.Exception.Message)"
  }
  if (-not (Test-Path -LiteralPath $theirs) -or (Get-Item -LiteralPath $theirs).Length -eq 0) {
    "$name : FAILED - Origin wrote no listing (it did not open the file)"
    $failed++
    continue
  }
  $loaded = (Select-String -Path $theirs -Pattern '^# project: loaded=' | Select-Object -First 1).Line
  $books = (Select-String -Path $theirs -Pattern '^BOOK\t').Count
  $cells = (Select-String -Path $theirs -Pattern '^CELL\t').Count
  "$name : $loaded; Origin lists $books workbook(s) and $cells cell(s)"
  & python (Join-Path $here 'opj-compare-cells.py') $theirs $ours --details $details
  if ($LASTEXITCODE -ne 0) { "$name : DIFFERENCES - see $details"; $failed++ } else { "$name : every compared item matches" }
}
exit $failed
