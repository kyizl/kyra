param(
    [string]$Username = "whateva",
    [int[]]$ProtocolVersions = @(47, 759, 765, 769, 775),
    [switch]$AllSupportedVersions,
    [switch]$SkipProxy
)

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$targets = @(
    @{ Name = "pika-edge"; Host = "172.65.169.236"; ServerAddress = "pika.host"; Versions = @(47, 759, 765, 769, 774) },
    @{ Name = "jartex"; Host = "play.jartex.fun"; Versions = @(47, 759, 765, 769, 775) }
)

$failed = 0
$supportedVersions = @(47, 76, 107, 109, 110, 201, 210, 304, 315, 316, 321, 327, 335, 338, 340, 351, 393, 401, 404, 477, 480, 490, 498, 573, 575, 578, 709, 710, 735, 736, 751, 753, 754, 755, 756, 757, 758, 759, 760, 761, 762, 763, 764, 765, 766, 767, 768, 769, 770, 771, 772, 773, 774, 775)
function Invoke-StatusProbe([string]$HostName, [int]$Port, [int]$Version, [string[]]$ExtraArgs) {
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        cargo run --quiet -p kyra_proto_spike -- `
            --host $HostName --port $Port --protocol-version $Version `
            --username $Username --status @ExtraArgs
        if ($LASTEXITCODE -eq 0) {
            return $true
        }
        Start-Sleep -Seconds 2
    }
    return $false
}

foreach ($target in $targets) {
    $versions = if ($AllSupportedVersions) { $supportedVersions } else { $ProtocolVersions }
    if (-not $AllSupportedVersions -and -not $PSBoundParameters.ContainsKey("ProtocolVersions")) {
        $versions = $target.Versions
    }
    foreach ($version in $versions) {
        Write-Host "[$($target.Name)] status protocol=$version host=$($target.Host)"
        $serverAddressArgs = @()
        if ($target.ContainsKey("ServerAddress")) {
            $serverAddressArgs = @("--server-address", $target.ServerAddress)
        }
        if (-not (Invoke-StatusProbe $target.Host 25565 $version $serverAddressArgs)) {
            $failed++
            Write-Warning "status probe failed for $($target.Name) protocol $version"
        }
    }
}

if (-not $SkipProxy) {
    $proxyPort = 25570
    cargo build --quiet -p kyra_proxy --bin kyra_proxy_live
    if ($LASTEXITCODE -ne 0) {
        throw "failed to build live proxy"
    }
    $proxyProcess = Start-Process `
        -FilePath (Join-Path $root "target\debug\kyra_proxy_live.exe") `
        -ArgumentList @(
            "127.0.0.1:$proxyPort", "172.65.169.236:25565", "pika.host"
        ) `
        -WorkingDirectory $root `
        -PassThru `
        -WindowStyle Hidden
    try {
        $ready = $false
        for ($attempt = 0; $attempt -lt 120; $attempt++) {
            Start-Sleep -Milliseconds 500
            $ready = Test-NetConnection -ComputerName 127.0.0.1 -Port $proxyPort -InformationLevel Quiet
            if ($ready) {
                break
            }
        }
        if (-not $ready) {
            throw "proxy did not listen on 127.0.0.1:$proxyPort"
        }
        $proxyVersions = if ($AllSupportedVersions) { $supportedVersions } else { $ProtocolVersions }
        if (-not $AllSupportedVersions -and -not $PSBoundParameters.ContainsKey("ProtocolVersions")) {
            $proxyVersions = $targets[0].Versions
        }
        foreach ($version in $proxyVersions) {
            Write-Host "[proxy->pika] status protocol=$version"
            if (-not (Invoke-StatusProbe "127.0.0.1" $proxyPort $version @())) {
                $failed++
                Write-Warning "proxy status probe failed for protocol $version"
            }
        }
    } catch {
        $failed++
        Write-Warning $_
    } finally {
        if ($proxyProcess -and -not $proxyProcess.HasExited) {
            Stop-Process -Id $proxyProcess.Id -Force
        }
    }
}

if ($failed -gt 0) {
    Write-Error "$failed live status probes failed."
    exit 1
}

Write-Host "All live status probes passed."
