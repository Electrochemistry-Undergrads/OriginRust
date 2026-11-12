param([Parameter(Mandatory=$true)][string]$Script,[string]$WorkDir='.',[int]$TimeoutSec=240)
# Like run-labtalk-lines.ps1 (same line syntax: "?x" prints a number, "$x" a string, "#[Book]Sheet" dumps a
# worksheet), but with a watchdog: if a command has not returned after $TimeoutSec seconds (a dialog opened
# and is waiting for a click), the Origin process that this script started is closed, and the script ends.
# It takes the same system-wide lock as the other reference scripts and never touches an Origin it did not start.
$inv=[Globalization.CultureInfo]::InvariantCulture
$work=(Resolve-Path $WorkDir).Path.TrimEnd('\')+'\'
$lines=Get-Content -LiteralPath $Script
$lock=New-Object System.Threading.Mutex($false,'Global\SheafOriginReference')
try{$held=$lock.WaitOne([TimeSpan]::FromMinutes(30))}catch [System.Threading.AbandonedMutexException]{$held=$true}
$origin=$null
$before=@(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'Origin*' } | ForEach-Object { $_.Id })
$ps=$null
try{
 $origin=New-Object -ComObject Origin.Application
 $mine=@(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'Origin*' -and ($before -notcontains $_.Id) } | ForEach-Object { $_.Id })
 # Watchdog thread: closes only the processes that appeared when this script started Origin.
 $state=[hashtable]::Synchronized(@{deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSec);stop=$false;killed=$false})
 $ps=[powershell]::Create()
 [void]$ps.AddScript({ param($state,$mine)
   while(-not $state.stop){ Start-Sleep -Milliseconds 500
     if([DateTime]::UtcNow -gt $state.deadline){ $state.killed=$true; foreach($i in $mine){ try{Stop-Process -Id $i -Force}catch{} }; break } } }).AddArgument($state).AddArgument($mine)
 [void]$ps.BeginInvoke()
 foreach($l in $lines){
  if($state.killed){ "#TIMEOUT the previous command did not return; Origin was closed"; break }
  if($l.Trim() -eq ''){continue}
  $state.deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSec)
  if($l.StartsWith('?')){ $v=$l.Substring(1).Trim(); try{ $x=$origin.LTVar($v); "$v = "+([double]$x).ToString('R',$inv)}catch{"$v = ERR $_"}; continue}
  if($l.StartsWith('$')){ $v=$l.Substring(1).Trim(); try{ "$v = "+$origin.LTStr($v) }catch{"$v = ERR $_"}; continue}
  if($l.StartsWith('#')){ $sh=$l.Substring(1).Trim(); $t=[System.__ComObject]
    try{ $r=$t.InvokeMember('GetWorksheet',[Reflection.BindingFlags]::InvokeMethod,$null,$origin,@($sh)) }catch{ "#ERR $sh $_"; continue }
    if($r -is [Array]){ $rows=$r.GetLength(0);$cols=$r.GetLength(1);"#SHEET $sh $rows $cols";$l0=$r.GetLowerBound(0);$l1=$r.GetLowerBound(1)
      for($i=$l0;$i -lt $l0+$rows;$i++){ $line=for($j=$l1;$j -lt $l1+$cols;$j++){$c=$r[$i,$j]; if($c -is [DBNull] -or $null -eq $c){''}elseif($c -is [double]){$c.ToString('R',$inv)}else{[string]$c}}; $line -join "`t" } } else {"#NOARRAY $sh"}
    continue}
  try{ $ok=$origin.Execute($l.Replace('%WORK%',$work)); "[$ok] $l" }catch{ "[ERR] $l  $_" }
 }
}catch{"#EXC $_"}
finally{
 if($null -ne $state){$state.stop=$true}
 if($null -ne $origin -and -not ($state -and $state.killed)){try{[void]$origin.Exit()}catch{}}
 if($null -ne $origin){try{[void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)}catch{}}
 Start-Sleep -Seconds 2
 $lock.ReleaseMutex();$lock.Dispose()
}
