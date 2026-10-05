# Runs inside resize-stress-windows.ps1's isolated native fixture. No user store.
function Wait-ActionFrame {
    param([scriptblock]$Accept, [string]$Stage)
    $script:testStage = $Stage
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        $frame = Read-AutomexiaSnapshot -AfterSequence ([int64]$script:actionFrame.sequence)
        $script:actionFrame = $frame
        if (& $Accept $frame) { return $frame }
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($ModalCaptureDirectory) {
        $null = [IO.Directory]::CreateDirectory($ModalCaptureDirectory)
        $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $ModalCaptureDirectory 'failure.png'))
    }
    throw "Quick Actions did not reach $Stage; $($script:actionFrame.palette_accessibility_summary)"
}
function Action-Key {
    param([uint32]$Key, [bool]$Control = $false, [bool]$Shift = $false)
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, $false, $Control, $Shift)) {
        throw 'Quick Actions native keyboard delivery failed'
    }
    Start-Sleep -Milliseconds 90
}
function Action-Text {
    param([string]$Text)
    if (-not [AutomexiaResizeDriver]::SendActionText($window, $Text, $false)) { throw 'Quick Actions text input failed' }
    Start-Sleep -Milliseconds 120
}
function Action-Choose {
    param([string]$Label)
    # Search the real palette, then activate its selected native row.
    Action-Text $Label
    $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like "*; $Label*" } "choose $Label"
    Action-Key 0x0D
}
function Action-Field {
    param([string]$Label, [string]$Value)
    Action-Choose $Label
    # Field summaries deliberately omit the value and row details. Match the
    # specific editor scope instead of requiring those private details.
    $fieldScope = switch ($Label) {
        'Name' { '^Action name; 1 results;' }
        'Command' { '^Command .+no passwords or tokens; 1 results;' }
        default { throw "Unregistered fixture field: $Label" }
    }
    $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -match $fieldScope } "edit $Label"
    Action-Key 0x41 $true
    Action-Text $Value
    Action-Key 0x0D
    $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -match '^(Edit |Workflow step)' } "accept $Label"
}
function Open-ActionCenter {
    Send-AutomexiaTestControl ('open-quick-actions:actions-' + [guid]::NewGuid().ToString('N'))
    $null = Wait-ActionFrame { param($f) $f.palette_enabled -and [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'open actions'
}
$script:actionFrame = $initial
$originalPrompt = $initial.latest_prompt_id
Open-ActionCenter
Action-Key 0x4E $true
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit Quick Action;*' } 'create action'
Action-Field 'Name' 'Fixture Insert'
Action-Field 'Command' 'Write-Output ACTION_INSERT_ONLY'
Action-Key 0x53 $true
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'save action'
Action-Choose 'Fixture Insert'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Review Quick Action;*' } 'review insertion'
if ($script:actionFrame.latest_prompt_id -ne $originalPrompt) { throw 'Review executed a command' }
Action-Key 0x0D
$inserted = Wait-ActionFrame { param($f)
    -not $f.palette_enabled -and [string](Get-ActiveAutomexiaPanel $f).raw_cursor_line_text -like '*ACTION_INSERT_ONLY*'
} 'insert without execution'
if ($inserted.latest_prompt_id -ne $originalPrompt) { throw 'Ordinary Quick Action synthesized Enter' }
Action-Key 0x43 $true
$null = Wait-ActionFrame { param($f) $f.latest_prompt_id -ne $originalPrompt } 'clear inserted line'
Open-ActionCenter
Action-Key 0x4E $true $true
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'create workflow'
Action-Field 'Name' 'Fixture Workflow'
Action-Choose 'Step 1'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Workflow step 1;*' } 'first step'
Action-Field 'Command' 'Set-Content -LiteralPath ($env:AUTOMEXIA_CONFIG_HOME + "/action-order.txt") -Value one'
Action-Key 0x1B
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'back to workflow'
Action-Choose 'Add step'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Workflow step 2;*' } 'second step'
Action-Field 'Command' 'Add-Content -LiteralPath ($env:AUTOMEXIA_CONFIG_HOME + "/action-order.txt") -Value two'
Action-Key 0x1B
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'back to workflow'
Action-Key 0x53 $true
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'save workflow'
Action-Choose 'Fixture Workflow'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Review: Fixture Workflow;*' } 'review workflow'
$output = Join-Path $configRoot 'action-order.txt'
if (Test-Path -LiteralPath $output) { throw 'Saving or reviewing workflow executed it' }
if ($ModalCaptureDirectory) {
    $null = [IO.Directory]::CreateDirectory($ModalCaptureDirectory)
    $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $ModalCaptureDirectory 'workflow-review.png'))
}
Action-Choose 'Run workflow'
$null = Wait-ActionFrame { param($f) -not $f.palette_enabled } 'explicit workflow run'
$deadline = [DateTime]::UtcNow.AddSeconds(20)
do {
    Start-Sleep -Milliseconds 100
    if ((Test-Path -LiteralPath $output) -and ([IO.File]::ReadAllText($output).Trim() -eq "one`r`ntwo")) { break }
} while ([DateTime]::UtcNow -lt $deadline)
if (-not (Test-Path -LiteralPath $output) -or [IO.File]::ReadAllText($output).Trim() -ne "one`r`ntwo") {
    Open-ActionCenter
    Action-Choose 'Workflow progress'
    $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Workflow:*' } 'failed workflow status'
    throw "Workflow did not execute exactly once in order: $($script:actionFrame.palette_accessibility_summary)"
}
Open-ActionCenter
Action-Choose 'Workflow progress'
$complete = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like '*Workflow complete.*' } 'workflow complete'
if ($ModalCaptureDirectory) {
    $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $ModalCaptureDirectory 'workflow-complete.png'))
}
Action-Key 0x1B
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'back to actions'
Action-Choose 'Manage saved actions'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Manage Quick Actions;*' } 'manage actions'
Action-Choose 'Fixture Workflow'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'edit saved workflow'
Action-Choose 'Duplicate'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'duplicate workflow'
Action-Field 'Name' 'Fixture Failure'
Action-Choose 'Step 1'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Workflow step 1;*' } 'failure step'
Action-Field 'Command' 'Write-Error "Fixture expected failure" -ErrorAction Stop'
Action-Key 0x1B
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'back after failure edit'
Action-Key 0x53 $true
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'save failure workflow'
Action-Choose 'Fixture Failure'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Review: Fixture Failure;*' } 'review failure workflow'
Action-Choose 'Run workflow'
$null = Wait-ActionFrame { param($f) -not $f.palette_enabled } 'run failure workflow'
Start-Sleep -Milliseconds 1500
Open-ActionCenter
Action-Choose 'Workflow progress'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like '*Command failed.*' } 'stop on failed command'
if ([IO.File]::ReadAllText($output).Trim() -ne "one`r`ntwo") { throw 'Failure allowed the next step to execute' }
Action-Choose 'Cancel remaining steps'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like '*Cancelled.*' } 'cancel failed run'
Action-Key 0x1B
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'return to list'
Action-Choose 'Manage saved actions'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Manage Quick Actions;*' } 'manage failure action'
Action-Choose 'Fixture Failure'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'edit for pause'
Action-Field 'Name' 'Fixture Pause'
Action-Choose 'Step 1'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Workflow step 1;*' } 'pause step'
Action-Field 'Command' 'Write-Output fixture-pause'
Action-Choose 'After this command'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Workflow step 1;*' } 'new shell option'
Action-Choose 'After this command'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Workflow step 1;*' } 'pause option'
Action-Key 0x1B
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'back after pause edit'
Action-Key 0x53 $true
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'save pause workflow'
Action-Choose 'Fixture Pause'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Review: Fixture Pause;*' } 'review pause workflow'
Action-Choose 'Run workflow'
$null = Wait-ActionFrame { param($f) -not $f.palette_enabled } 'run pause workflow'
Start-Sleep -Milliseconds 1000
Open-ActionCenter
Action-Choose 'Workflow progress'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like '*Step completed. Resume to continue.*' } 'explicit pause'
if ([IO.File]::ReadAllText($output).Trim() -ne "one`r`ntwo") { throw 'Pause allowed next step to execute' }
Action-Choose 'Cancel remaining steps'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like '*Cancelled.*' } 'cancel paused run'
Start-Sleep -Milliseconds 500
if ([IO.File]::ReadAllText($output).Trim() -ne "one`r`ntwo") { throw 'Cancelled workflow resumed' }
Action-Key 0x1B
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'return to actions for delete'
Action-Choose 'Manage saved actions'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Manage Quick Actions;*' } 'manage deletion'
Action-Choose 'Fixture Pause'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'edit before deletion'
Action-Choose 'Delete'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Delete this saved action?*' } 'delete confirmation'
Action-Key 0x1B
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit workflow;*' } 'cancel deletion'
Action-Choose 'Delete'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Delete this saved action?*' } 'repeat deletion'
Action-Choose 'Delete action'
$null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'delete confirmed'
# Returning from Save or Discard must retain the invoking command-menu query,
# and consume that return location once. Repeated Back cannot edit or execute.
foreach ($saveDraft in @($false, $true)) {
    Send-AutomexiaTestControl ('open-palette:action-back-' + $saveDraft)
    $null = Wait-ActionFrame { param($f) $f.palette_enabled -and [string]$f.palette_accessibility_summary -like 'Command categories;*' } 'open invoking menu'
    Action-Choose 'Quick Actions'
    $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'open actions from menu'
    $promptBeforeBack = $script:actionFrame.latest_prompt_id
    Action-Key 0x4E $true
    $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit Quick Action;*' } 'create navigation draft'
    Action-Field 'Name' 'Back fixture'
    Action-Field 'Command' 'Write-Output back-fixture'
    if ($saveDraft) {
        Action-Key 0x53 $true
    } else {
        if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $true, $true)) { throw 'Draft Back delivery failed' }
        $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Discard this draft?*' } 'draft Back confirmation'
        if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $false, $true)) { throw 'Confirmation Back delivery failed' }
        $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Edit Quick Action;*' } 'cancel discard without skipping editor'
        Action-Key 0x1B
        $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Discard this draft?*' } 'request discard again'
        Action-Choose 'Discard changes'
    }
    $null = Wait-ActionFrame { param($f) [string]$f.palette_accessibility_summary -like 'Quick Actions;*' } 'return from save or discard'
    if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $true, $true)) { throw 'Action list Back delivery failed' }
    $null = Wait-ActionFrame { param($f) $f.palette_enabled -and [string]$f.palette_accessibility_summary -like 'All commands;*Quick Actions*' } 'restore invoking search once'
    if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $true, $true)) { throw 'Invoking menu Back delivery failed' }
    $null = Wait-ActionFrame { param($f) -not $f.palette_enabled } 'leave invoking menu without reopening actions'
    if ($script:actionFrame.latest_prompt_id -ne $promptBeforeBack) { throw 'Menu navigation executed a command' }
}
Write-Host 'Quick Actions native fixture passed: keyboard create/edit/duplicate/save/delete confirmation; insert without Enter; ordered Run; failure stop; explicit pause and cancellation; held Back and menu origin after Save/Discard.'
