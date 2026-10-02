# Opted-in session helper only; connect once, then submit without prompt waits.
if (-not (Get-Variable __amxS -Scope Global -ErrorAction Ignore)) { return }
if (Get-Command __amx_ssh_helper_prompt -CommandType Function -ErrorAction Ignore) { return }
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { return }
if ($env:AMX_SSH_HELPER_PIPE -notmatch '^Automexia\.Ssh\.[0-9a-f]{64}$') { return }
$__amxHelperTransport = $null
try {
    $__amxHelperTransport = [IO.Pipes.NamedPipeClientStream]::new('.', $env:AMX_SSH_HELPER_PIPE, [IO.Pipes.PipeDirection]::Out, [IO.Pipes.PipeOptions]::Asynchronous)
    $__amxHelperTransport.Connect(200)
} catch {
    if ($null -ne $__amxHelperTransport) { $__amxHelperTransport.Dispose() }
    return
}
$global:__amx_ssh_helper_state = @{
    Transport = $__amxHelperTransport
    Pending = $null; Buffer = $null; Revision = [uint32]0; Disabled = $false; Body = ''; Encoded = ''
    Utf8 = [Text.UTF8Encoding]::new($false, $true)
}
Remove-Variable __amxHelperTransport -ErrorAction Ignore
[Environment]::SetEnvironmentVariable('AMX_SSH_HELPER_PIPE', $null)

function global:__amx_ssh_helper_retire {
    $global:__amx_ssh_helper_state.Disabled = $true
    [Console]::Write("$([char]27)]1337;SetUserVar=automexia_ssh_revision=$([char]7)")
}

function global:__amx_ssh_helper_prompt {
    $h = $global:__amx_ssh_helper_state
    if ($h.Disabled) { return }
    $names = 'cwd','HOME','KUBECONFIG','HOMEDRIVE','HOMEPATH','USERPROFILE','DOCKER_CONTEXT','DOCKER_HOST_PRESENT','AWS_PROFILE','AWS_DEFAULT_PROFILE','AWS_REGION','AWS_DEFAULT_REGION','AZURE_CLOUD_NAME','CLOUDSDK_ACTIVE_CONFIG_NAME','CLOUDSDK_CORE_PROJECT','CLOUDSDK_COMPUTE_REGION','TF_WORKSPACE','AUTOMEXIA_ENV','ENVIRONMENT','APP_ENV','NODE_ENV','GIT_BRANCH','KUBECONTEXT','KUBE_CONTEXT','KUBE_NAMESPACE'
    $body = [Text.StringBuilder]::new()
    $invalid = $false
    foreach ($name in $names) {
        $limit = 256
        if ($name -eq 'cwd') {
            $value = if ($PWD.Provider.Name -eq 'FileSystem') { $PWD.ProviderPath } else { '' }
            $limit = 4096
        } elseif ($name -eq 'DOCKER_HOST_PRESENT') {
            $value = if ([string]::IsNullOrEmpty([Environment]::GetEnvironmentVariable('DOCKER_HOST'))) { '0' } else { '1' }
        } else {
            $value = [Environment]::GetEnvironmentVariable($name)
            if ($name -in 'HOME','KUBECONFIG','HOMEDRIVE','HOMEPATH','USERPROFILE') { $limit = 4096 }
        }
        try {
            if ($h.Utf8.GetByteCount([string]$value) -gt $limit -or $value -match '[\x00-\x1f\x7f-\x9f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]') {
                $invalid = $true
                break
            }
        } catch {
            $invalid = $true
            break
        }
        [void]$body.Append($name).Append('=').Append($value).Append("`n")
    }
    $header = "AMXREQ1|@@PANE@@|@@GENERATION@@|4294967295`n"
    if ($invalid -or $h.Utf8.GetByteCount($header + $body.ToString()) + 2 -gt 16384) {
        [void]$body.Clear()
        foreach ($name in $names) { [void]$body.Append($name).Append("=`n") }
    }
    $snapshot = $body.ToString()
    if ($snapshot -cne $h.Body) {
        if ($h.Revision -eq [uint32]::MaxValue) {
            __amx_ssh_helper_retire
            return
        }
        $h.Body = $snapshot
        $h.Revision++
        $h.Encoded = [Convert]::ToBase64String($h.Utf8.GetBytes("AMXSSHREV1|@@PANE@@|@@GENERATION@@|$($h.Revision)"))
    }
    $header = "AMXREQ1|@@PANE@@|@@GENERATION@@|$($h.Revision)`n"
    [Console]::Write("$([char]27)]1337;SetUserVar=automexia_ssh_revision=$($h.Encoded)$([char]7)")
    try {
        if ($null -ne $h.Pending) {
            # Never wait or dispose an outstanding operation on the prompt path.
            if (-not $h.Pending.IsCompleted) { return }
            $h.Transport.EndWrite($h.Pending)
            $h.Pending = $null
            $h.Buffer = $null
        }
        $h.Buffer = $h.Utf8.GetBytes([char]0 + $header + $snapshot + [char]0)
        $h.Pending = $h.Transport.BeginWrite($h.Buffer, 0, $h.Buffer.Length, $null, $null)
    } catch {
        __amx_ssh_helper_retire
    }
}

$global:__amxS.Helper = ${function:__amx_ssh_helper_prompt}
__amx_ssh_helper_prompt
