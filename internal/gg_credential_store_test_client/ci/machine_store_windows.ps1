# Stores a machine token as an administrator and checks it as a standard user.
# Usage: machine_store_windows.ps1 <test client>
param([Parameter(Mandatory)] [string] $Client)
$ErrorActionPreference = 'Stop'

$instance = 'https://dashboard.gitguardian.com'
$dir = Join-Path $env:ProgramData 'GitGuardian\ggshield'
$file = Join-Path $dir 'machine_credential.yaml'
$work = Join-Path $env:SystemDrive 'gg-machine-store'
$userClient = Join-Path $work 'gg_credential_store_test_client.exe'
$userName = 'ggmachinetest'

function Fail([string] $message) {
    throw "FAIL: $message"
}

function Invoke-AsAdministrator([string] $token, [string[]] $arguments) {
    $token | & $Client @arguments | ConvertFrom-Json
}

function Invoke-AsUser([string] $token, [string[]] $arguments) {
    $in = Join-Path $work 'in.txt'
    $out = Join-Path $work 'out.json'
    Set-Content -Path $in -Value $token -NoNewline
    Start-Process -FilePath $userClient -ArgumentList $arguments -Credential $credential `
        -LoadUserProfile -Wait -RedirectStandardInput $in -RedirectStandardOutput $out
    Get-Content -Path $out -Raw | ConvertFrom-Json
}

$store = @('store', '--instance', $instance, '--machine-id', 'machine-1')
$read = @('read', '--instance', $instance)

Remove-Item -Path $dir, $work -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $work | Out-Null
Copy-Item -Path $Client -Destination $userClient
$password = ConvertTo-SecureString "$([guid]::NewGuid())Aa1!" -AsPlainText -Force
New-LocalUser -Name $userName -Password $password -PasswordNeverExpires | Out-Null
$credential = [pscredential]::new($userName, $password)

try {
    $output = Invoke-AsAdministrator 'gg_mat_admin' $store
    if ($output.result.location -ne 'machine_file') { Fail "administrator store: $($output | ConvertTo-Json -Compress)" }
    $acl = Get-Acl -Path $file
    Write-Host "owner of $file`: $($acl.Owner)"
    if (-not $acl.AreAccessRulesProtected) { Fail "$file inherits its ACL" }

    $output = Invoke-AsUser '' $read
    $result = $output.result
    if ($result.token -ne 'gg_mat_admin' -or $result.source -ne 'machine_file' -or $result.machine_id -ne 'machine-1') {
        Fail "user read: $($output | ConvertTo-Json -Compress)"
    }

    $output = Invoke-AsUser '' @('managed-instance')
    if ($output.result.instance -ne $instance) { Fail "managed instance: $($output | ConvertTo-Json -Compress)" }

    $output = Invoke-AsUser 'gg_mat_user' $store
    if ($output.error.kind -ne 'machine_store_denied') { Fail "user overwrite: $($output | ConvertTo-Json -Compress)" }
    $output = Invoke-AsAdministrator '' $read
    if ($output.result.token -ne 'gg_mat_admin') { Fail "read after overwrite: $($output | ConvertTo-Json -Compress)" }

    Write-Host 'machine store: passed'
}
finally {
    Remove-Item -Path $dir, $work -Recurse -Force -ErrorAction SilentlyContinue
    Remove-LocalUser -Name $userName -ErrorAction SilentlyContinue
}
