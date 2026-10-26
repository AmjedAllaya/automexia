# Pure driver regression. It never loads UIA, injects input or launches Automexia.
param(
    [Parameter(Mandatory)][string]$RepositoryRoot,
    [ValidateSet('first', 'second')][string]$Position,
    [switch]$ImmediateUpdates,
    [switch]$SuppressAcknowledgement
)
$ErrorActionPreference = 'Stop'
$tokens = $null
$parseErrors = $null
$source = Join-Path $RepositoryRoot 'tests/integration/windows-package-review.ps1'
$ast = [Management.Automation.Language.Parser]::ParseFile($source, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) { throw 'Native driver source did not parse' }
$definition = $ast.Find({ param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Find-Setting'
}, $true)
if ($null -eq $definition) { throw 'Native navigation owner is missing' }
. ([scriptblock]::Create($definition.Extent.Text))

$script:opened = $false
$script:modelFocus = 'search'
$script:publishedFocus = 'search'
$script:query = ''
$script:arrows = 0
$script:toggled = $false
$script:activations = 0
$targetId = 'terminal.kubernetes_highlighting'
$otherId = 'terminal.output_highlighting'

function Read-Nodes {
    if (-not $script:opened) {
        return ,@([pscustomobject]@{ Name = 'Terminal output'; Id = ''; Focused = $true })
    }
    # A native accessibility client can see the last published frame while a
    # newer keyboard selection is already owned by the application's model.
    $observed = if ($ImmediateUpdates) { $script:modelFocus } else { $script:publishedFocus }
    if (-not $SuppressAcknowledgement) { $script:publishedFocus = $script:modelFocus }
    return ,@(
        [pscustomobject]@{ Name = 'Search settings'; Id = ''; Type = 'ControlType.Edit';
            Focused = $observed -eq 'search'; Value = $script:query },
        [pscustomobject]@{ Name = 'Highlight Kubernetes status'; Id = $targetId;
            Type = 'ControlType.CheckBox'; Focused = $observed -eq 'target'; Enabled = $true },
        [pscustomobject]@{ Name = 'Highlight recognized output'; Id = $otherId;
            Type = 'ControlType.CheckBox'; Focused = $observed -eq 'other'; Enabled = $true }
    )
}
function Wait-Nodes([scriptblock]$Predicate) {
    for ($i = 0; $i -lt 12; $i++) {
        $nodes = Read-Nodes
        if (& $Predicate $nodes) { return ,$nodes }
    }
    throw 'Model accessibility acknowledgement was not reached'
}
function Click-Node($Node) {
    if ($Node.Name -ne 'Search settings') { throw 'Unexpected model pointer target' }
    $script:modelFocus = 'search'
}
function Type-Text([string]$Text) { $script:query = $Text }
function Key([uint32]$Code, [bool]$Control = $false, [bool]$Shift = $false) {
    switch ($Code) {
        0x53 {
            if (-not $Control -or -not $Shift) { throw 'Unexpected model shortcut' }
            $script:opened = $true
        }
        0x41 { if (-not $Control) { throw 'Unexpected model selection' } }
        0x09 {
            if ($script:modelFocus -ne 'search') { throw 'Tab left an unacknowledged search field' }
            $script:modelFocus = if ($Position -eq 'first') { 'target' } else { 'other' }
        }
        0x28 {
            $script:arrows++
            $script:modelFocus = if ($script:modelFocus -eq 'target') { 'other' } else { 'target' }
        }
        0x20 { $script:activations++; $script:toggled = $script:modelFocus -eq 'target' }
        default { throw 'Unexpected model key' }
    }
}
try {
    $null = Find-Setting 'Highlight Kubernetes status' $targetId
} catch {
    if ($SuppressAcknowledgement -and
        $_.Exception.Message -eq 'Model accessibility acknowledgement was not reached' -and
        $script:arrows -eq 0 -and $script:activations -eq 0) {
        Write-Output 'PASS: missing focus prevented activation'
        exit 0
    }
    throw
}
if ($SuppressAcknowledgement) { throw 'Missing focus was accepted by the driver' }
Key 0x20
$expectedArrows = if ($Position -eq 'first') { 0 } else { 1 }
if (-not $script:toggled -or $script:arrows -ne $expectedArrows) {
    throw 'Driver outran the acknowledged focus and activated a different control'
}
Write-Output 'PASS: one acknowledged move at a time'
