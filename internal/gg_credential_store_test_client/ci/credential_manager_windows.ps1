# Checks the user token store against the Credential Manager and Python keyring,
# including the compound targets that keyring uses for a second instance.
# Usage: credential_manager_windows.ps1 <test client>
param([Parameter(Mandatory)] [string] $Client)
$ErrorActionPreference = 'Stop'

$Client = (Resolve-Path -Path $Client).Path
$first = 'https://dashboard.gitguardian.com'
$second = 'https://b.test-client.invalid'
$third = 'https://c.test-client.invalid'
$env:GG_CONFIG_DIR = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid())
$env:PYTHON_KEYRING_BACKEND = 'keyring.backends.Windows.WinVaultKeyring'
New-Item -ItemType Directory -Path $env:GG_CONFIG_DIR | Out-Null

function Fail([string] $message) {
    throw "FAIL: $message"
}

function Invoke-Client([string] $token, [string[]] $arguments) {
    $token | & $Client @arguments | ConvertFrom-Json
}

function Set-KeyringToken([string] $instance, [string] $token) {
    python -c 'import sys, keyring; keyring.set_password(*sys.argv[1:])' ggshield $instance $token
}

function Assert-Read([string] $label, [string] $instance, [string] $token) {
    $output = Invoke-Client '' @('read', '--instance', $instance)
    if ($output.result.token -ne $token -or $output.result.source -ne 'user_os_store') {
        Fail "$label`: $($output | ConvertTo-Json -Compress)"
    }
}

function Assert-KeyringToken([string] $label, [string] $instance, [string] $token) {
    $read = python -c 'import sys, keyring; print(keyring.get_password(*sys.argv[1:]))' ggshield $instance
    if ($read -ne $token) { Fail "$label`: $read" }
}

function Invoke-Store([string] $label, [string] $instance, [string] $token) {
    $output = Invoke-Client $token @('store', '--instance', $instance)
    if ($output.result.location -ne 'user_os_store') { Fail "$label`: $($output | ConvertTo-Json -Compress)" }
}

try {
    Invoke-Store 'store' $first 'gg_first_crate'
    Assert-KeyringToken 'Python keyring read' $first 'gg_first_crate'

    Set-KeyringToken $first 'gg_first_keyring'
    Assert-Read 'read after Python keyring' $first 'gg_first_keyring'

    Invoke-Store 'store of the second instance' $second 'gg_second'
    Assert-KeyringToken 'Python keyring read of the moved first instance' $first 'gg_first_keyring'
    Assert-KeyringToken 'Python keyring read of the second instance' $second 'gg_second'

    Invoke-Store 'store of the third instance' $third 'gg_third'
    Set-KeyringToken $first 'gg_first_last'
    Assert-Read 'read of the first instance' $first 'gg_first_last'
    Assert-Read 'read of the second instance' $second 'gg_second'
    Assert-Read 'read of the moved third instance' $third 'gg_third'

    foreach ($instance in $first, $second, $third) {
        $output = Invoke-Client '' @('delete', '--instance', $instance)
        if (-not $output.result.deleted) { Fail "delete of $instance`: $($output | ConvertTo-Json -Compress)" }
        Assert-KeyringToken "Python keyring read after the delete of $instance" $instance 'None'
    }

    Write-Host 'credential manager: passed'
}
finally {
    Remove-Item -Path $env:GG_CONFIG_DIR -Recurse -Force -ErrorAction SilentlyContinue
}
