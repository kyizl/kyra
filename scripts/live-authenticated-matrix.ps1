param(
    [string]$Username = "KxrpyTyx",
    [string]$Password = $env:KYRA_TEST_PASSWORD,
    [switch]$SkipStatusMatrix,
    [switch]$SkipHeadlessPreflight,
    [switch]$KeepArtifacts,
    [switch]$StopAtBedwarsServer
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$password = if ($Password) { $Password } else { "KxrpyTyx_kyra_test_2026" }
$work = Join-Path $env:TEMP "kyra-pika-nbt-$PID"
$proxyLog = Join-Path $work "proxy.log"
$proxyStdout = Join-Path $work "proxy.stdout.log"
$proxyStderr = Join-Path $work "proxy.stderr.log"
$clientLog = Join-Path $work "client.log"
$proxyProcess = $null
$completed = $false
$statusMatrixRan = -not $SkipStatusMatrix

New-Item -ItemType Directory -Force -Path $work | Out-Null

try {
    if (-not $SkipStatusMatrix) {
        & (Join-Path $root "scripts\live-server-check.ps1") -Username $Username -AllSupportedVersions
        if ($LASTEXITCODE -ne 0) {
            throw "live protocol status matrix failed"
        }
    }

    $headlessRoot = Join-Path $work "client-headless"
    git clone --depth 1 https://github.com/oxidized-mc/client-headless.git $headlessRoot
    if ($LASTEXITCODE -ne 0) {
        throw "failed to clone oxidized client-headless"
    }
    if (-not $SkipHeadlessPreflight) {
        & cargo test --manifest-path (Join-Path $headlessRoot "Cargo.toml")
        if ($LASTEXITCODE -ne 0) {
            throw "client-headless preflight failed"
        }
        Write-Host "client-headless preflight passed; repository is currently a non-functional 26.1 scaffold."
    }

    $pikaRoot = Join-Path $work "pika-nbt"
    git clone --depth 1 https://github.com/kyizl/pika-nbt.git $pikaRoot
    if ($LASTEXITCODE -ne 0) {
        throw "failed to clone pika-nbt"
    }
    Push-Location $pikaRoot
    try {
        & bun install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "pika-nbt dependency install failed" }
        & bun run build
        if ($LASTEXITCODE -ne 0) { throw "pika-nbt build failed" }
        New-Item -ItemType Directory -Force -Path (Join-Path $pikaRoot "scripts") | Out-Null
        Copy-Item (Join-Path $root "scripts\pika-authenticated-client-test.ts") `
            (Join-Path $pikaRoot "scripts\pika-authenticated-client-test.ts") -Force
    } finally {
        Pop-Location
    }

    & cargo build --quiet -p kyra_proxy --bin kyra_proxy_live
    if ($LASTEXITCODE -ne 0) { throw "failed to build observed proxy" }
    $proxyProcess = Start-Process `
        -FilePath (Join-Path $root "target\debug\kyra_proxy_live.exe") `
        -ArgumentList @("127.0.0.1:25566", "172.65.169.236:25565", "pika.host", "--observe") `
        -WorkingDirectory $root `
        -RedirectStandardOutput $proxyStdout `
        -RedirectStandardError $proxyStderr `
        -PassThru `
        -WindowStyle Hidden

    $ready = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        Start-Sleep -Milliseconds 500
        $ready = Test-NetConnection -ComputerName 127.0.0.1 -Port 25566 -InformationLevel Quiet
        if ($ready) { break }
    }
    if (-not $ready) { throw "observed proxy did not listen on 127.0.0.1:25566" }

    Push-Location $pikaRoot
    try {
        $env:KYRA_TEST_USERNAME = $Username
        $env:KYRA_TEST_PASSWORD = $password
        $env:KYRA_TEST_HOST = "127.0.0.1"
        $env:KYRA_TEST_PORT = "25566"
        $env:KYRA_TEST_VERSION = "1.8.9"
        if ($StopAtBedwarsServer) {
            $env:KYRA_TEST_STOP_AT_BEDWARS_SERVER = "1"
        } else {
            Remove-Item Env:KYRA_TEST_STOP_AT_BEDWARS_SERVER -ErrorAction SilentlyContinue
        }
        $clientPassed = $false
        for ($attempt = 1; $attempt -le 3; $attempt++) {
            Write-Host "Authenticated client attempt $attempt/3"
            & bun run scripts/pika-authenticated-client-test.ts 2>&1 | Tee-Object -FilePath $clientLog -Append
            if ($LASTEXITCODE -eq 0) {
                $clientPassed = $true
                break
            }
            if ($attempt -lt 3) { Start-Sleep -Seconds 5 }
        }
        if (-not $clientPassed) { throw "authenticated Pika client test failed after 3 attempts" }
    } finally {
        Pop-Location
    }

    $proxyText = (Get-Content -Raw -Path $proxyStdout) + (Get-Content -Raw -Path $proxyStderr)
    Set-Content -Path $proxyLog -Value $proxyText
    if ($proxyText -match "observer rejected|decode-error") {
        throw "proxy observer reported a rejected or undecodable packet; see $proxyLog"
    }
    if ($proxyText -notmatch "player-join") {
        throw "proxy did not observe a player-join event; see $proxyLog"
    }
    Write-Host "Authenticated 1.8.9 proxy session passed for $Username."
    if ($statusMatrixRan) {
        Write-Host "All-version status matrix passed."
    } else {
        Write-Host "All-version status matrix was skipped."
    }
    if ($KeepArtifacts) {
        Write-Host "Logs: $proxyLog and $clientLog"
    }
    $completed = $true
} finally {
    if ($proxyProcess -and -not $proxyProcess.HasExited) {
        Stop-Process -Id $proxyProcess.Id -Force
    }
    if ($completed -and -not $KeepArtifacts) {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    } elseif (-not $completed) {
        Write-Host "Test failed; preserved diagnostics under $work"
    } else {
        Write-Host "Preserved diagnostics under $work"
    }
}
