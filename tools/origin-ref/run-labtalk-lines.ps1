param([Parameter(Mandatory=$true)][string]$Script,[string]$WorkDir='.')
$inv=[Globalization.CultureInfo]::InvariantCulture
$work=(Resolve-Path $WorkDir).Path.TrimEnd('\')+'\'
$lines=Get-Content -LiteralPath $Script
$lock=New-Object System.Threading.Mutex($false,'Global\SheafOriginReference')
try{$held=$lock.WaitOne([TimeSpan]::FromMinutes(30))}catch [System.Threading.AbandonedMutexException]{$held=$true}
$origin=$null
try{
 $origin=New-Object -ComObject Origin.Application
 foreach($l in $lines){
  if($l.Trim() -eq ''){continue}
  if($l.StartsWith('?')){ $v=$l.Substring(1).Trim(); try{ $x=$origin.LTVar($v); "$v = "+([double]$x).ToString('R',$inv)}catch{"$v = ERR $_"}; continue}
  if($l.StartsWith('$')){ $v=$l.Substring(1).Trim(); "$v = "+$origin.LTStr($v); continue}
  if($l.StartsWith('#')){ $sh=$l.Substring(1).Trim(); $t=[System.__ComObject]
    try{ $r=$t.InvokeMember('GetWorksheet',[Reflection.BindingFlags]::InvokeMethod,$null,$origin,@($sh)) }catch{ "#ERR $sh $_"; continue }
    if($r -is [Array]){ $rows=$r.GetLength(0);$cols=$r.GetLength(1);"#SHEET $sh $rows $cols";$l0=$r.GetLowerBound(0);$l1=$r.GetLowerBound(1)
      for($i=$l0;$i -lt $l0+$rows;$i++){ $line=for($j=$l1;$j -lt $l1+$cols;$j++){$c=$r[$i,$j]; if($c -is [DBNull] -or $null -eq $c){''}elseif($c -is [double]){$c.ToString('R',$inv)}else{[string]$c}}; $line -join "`t" } } else {"#NOARRAY $sh"}
    continue}
  $ok=$origin.Execute($l.Replace('%WORK%',$work))
  "[$ok] $l"
 }
}catch{"#EXC $_"}
finally{
 if($null -ne $origin){try{[void]$origin.Exit()}catch{};try{[void][Runtime.InteropServices.Marshal]::ReleaseComObject($origin)}catch{};Start-Sleep -Seconds 2}
 $lock.ReleaseMutex();$lock.Dispose()
}
