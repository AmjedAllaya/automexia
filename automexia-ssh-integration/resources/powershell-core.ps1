if(Get-Variable __amxS -Scope Global -ea Ignore){return}
$env:COLORTERM='truecolor';$env:TERM_PROGRAM='Automexia'
$p=Get-Command prompt -Type Function -ea Ignore
$r=Get-Command PSConsoleHostReadLine -Type Function -ea Ignore
$m=Get-Module PSReadLine;if(-not $m){$m=Import-Module PSReadLine -PassThru -ea Ignore}
if(-not $r){$r=Get-Command PSConsoleHostReadLine -Type Function -ea Ignore}
$global:__amxS=@{P=$(if($p){$p.ScriptBlock});R=$(if($r){$r.ScriptBlock});B=$false;I=0;E=[char]27;A=[char]7;C='';F='';X='';U=[Text.Encoding]::UTF8}
$s=$global:__amxS
$u=[Environment]::UserName;if($s.U.GetByteCount($u) -gt 256 -or $u -match '[\x00-\x1f\x7f-\x9f]'){$u=''}
$u=[Convert]::ToBase64String($s.U.GetBytes('AMXSSHUSER1|@@PANE@@|@@GENERATION@@|'+$u))
$s.N="$($s.E)]1337;SetUserVar=automexia_ssh_user=$u$($s.A)"
$s.L=[bool]$m -and [bool]$s.R
if($s.L){
function global:PSConsoleHostReadLine {
 $line=& $global:__amxS.R
 if(-not [string]::IsNullOrWhiteSpace($line)){$global:__amxS.B=$true;[Console]::Write("$($global:__amxS.E)]133;C$($global:__amxS.A)")}
 $line
}}
function global:prompt {
 param($ok=$?)
 $native=if($global:__amxS.P){& $global:__amxS.P}else{'PS> '}
 $s=$global:__amxS;$code=if($ok){0}else{1}
 $done='';if($s.B){$done="$($s.E)]133;D;$code$($s.A)"}
 $path=$PWD.ProviderPath
 if($PWD.Provider.Name -ne 'FileSystem' -or $s.U.GetByteCount($path) -gt [int]'@@PATH_LIMIT@@' -or $path -match '[\x00-\x1f\x7f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]'){$path=''}
 if($s.C -ne $path -or -not $s.F){$s.C=$path;$b=[Convert]::ToBase64String($s.U.GetBytes('AMXSSHCWD1|@@PANE@@|@@GENERATION@@|'+$path));$s.F="$($s.E)]1337;SetUserVar=automexia_ssh_cwd=$b$($s.A)"}
 [Console]::Write($done+$s.F+$s.N)
 if($s.ContainsKey('Helper')){& $s.Helper}else{
 $x="AMXSSHCTX1|@@PANE@@|@@GENERATION@@|`n"
 foreach($q in 'git_branch:GIT_BRANCH','kubernetes_context:KUBECONTEXT,KUBE_CONTEXT','kubernetes_namespace:KUBE_NAMESPACE','docker_context:DOCKER_CONTEXT','terraform_workspace:TF_WORKSPACE','environment:AUTOMEXIA_ENV,ENVIRONMENT,APP_ENV,NODE_ENV','aws_profile:AWS_PROFILE,AWS_DEFAULT_PROFILE','azure_cloud:AZURE_CLOUD_NAME','gcp_project:CLOUDSDK_CORE_PROJECT'){
 $k,$n=$q.Split(':');$v='';foreach($n in $n.Split(',')){$v=[Environment]::GetEnvironmentVariable($n);if($null -ne $v){break}};if($s.U.GetByteCount([string]$v) -gt 256 -or $v -match '[\p{C}\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]'){$v=''};$x+="$k=$v`n"}
 if($x -ne $s.X){$s.X=$x;$x=[Convert]::ToBase64String($s.U.GetBytes($x));$s.H="$($s.E)]1337;SetUserVar=automexia_ssh_context=$x$($s.A)"}
 [Console]::Write($s.H)}
 if($s.B -or $s.I -eq 0){$s.I++;[Console]::Write("$($s.E)]133;A;aid=$($s.I)$($s.A) `r`n")}
 $s.B=$false
 "$($s.E)]133;P;k=c;aid=$($s.I)$($s.A)$native$($s.E)]133;B$($s.A)"
}
$r=if($s.L){'@@RECEIPT@@'}else{'@@PROMPT_CWD_RECEIPT@@'}
[Console]::Write("$($s.E)]1337;SetUserVar=automexia_ssh_ready=$r$($s.A)")
