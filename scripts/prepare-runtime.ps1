param(
    [string]$Destination = (Join-Path $PSScriptRoot "..\src-tauri\resources\runtime")
)

$ErrorActionPreference = "Stop"
$destination = [System.IO.Path]::GetFullPath($Destination)
$marker = Join-Path $destination ".deepx-runtime-ready"
$existingNode = Join-Path $destination "node\node.exe"
$existingDsh = Join-Path $destination "node_modules\@deepseek-ai\dsh\lib\bin.js"
if ((Test-Path $marker) -and (Test-Path $existingNode) -and (Test-Path $existingDsh)) {
    return
}

$nodeVersion = "v22.23.2"

$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ("deepx-runtime-" + [guid]::NewGuid())
$buildDestination = Join-Path $temporary "runtime"
$archive = Join-Path $temporary "node.zip"
$nodeRoot = Join-Path $temporary ("node-" + $nodeVersion + "-win-x64")
$peerNames = @(
    "@deepseek-ai/dsh-anonymous-user-id",
    "@deepseek-ai/dsh-atomic-write",
    "@deepseek-ai/dsh-authorization",
    "@deepseek-ai/dsh-bash-local",
    "@deepseek-ai/dsh-code-runtime",
    "@deepseek-ai/dsh-compaction",
    "@deepseek-ai/dsh-fs",
    "@deepseek-ai/dsh-invariants",
    "@deepseek-ai/dsh-output-retention",
    "@deepseek-ai/dsh-sandbox",
    "@deepseek-ai/dsh-scope",
    "@deepseek-ai/dsh-session-telemetry",
    "@deepseek-ai/dsh-session-title-llm",
    "@deepseek-ai/dsh-shell",
    "@deepseek-ai/dsh-spill",
    "@deepseek-ai/dsh-subagent-in-process-driver",
    "@deepseek-ai/dsh-timeout",
    "@deepseek-ai/dsh-workflow"
)

New-Item -ItemType Directory -Force -Path $temporary, $buildDestination | Out-Null
Invoke-WebRequest "https://nodejs.org/dist/$nodeVersion/node-$nodeVersion-win-x64.zip" -OutFile $archive -TimeoutSec 120
tar.exe -xf $archive -C $temporary
if ($LASTEXITCODE -ne 0) {
    throw "Failed to extract the Node.js runtime archive"
}
Copy-Item -Path $nodeRoot -Destination (Join-Path $buildDestination "node") -Recurse -Force

$node = Join-Path $buildDestination "node\node.exe"
$npm = Join-Path $buildDestination "node\node_modules\npm\bin\npm-cli.js"
$npmOptions = @("install", "--no-audit", "--no-fund", "--no-package-lock", "--legacy-peer-deps", "--fetch-timeout", "30000", "--fetch-retries", "1", "--maxsockets", "8", "--prefix", $buildDestination)
& $node $npm $npmOptions "@deepseek-ai/dsh@latest" "@deepseek-ai/cordis-plugin-group"
if ($LASTEXITCODE -ne 0) {
    throw "Failed to install the bundled DeepSeek Harness"
}

$dsh = Join-Path $buildDestination "node_modules\@deepseek-ai\dsh\lib\bin.js"
$dshManifest = Join-Path $buildDestination "node_modules\@deepseek-ai\dsh\package.json"
if (-not (Test-Path $dsh) -or -not (Test-Path $dshManifest)) {
    throw "Bundled DeepSeek Harness entry point is missing"
}
$dshVersion = (Get-Content -LiteralPath $dshManifest -Raw | ConvertFrom-Json).version
if ([string]::IsNullOrWhiteSpace($dshVersion)) {
    throw "Bundled DeepSeek Harness version is missing"
}
$versionedPeers = $peerNames | ForEach-Object { "$_@$dshVersion" }
& $node $npm $npmOptions $versionedPeers
if ($LASTEXITCODE -ne 0) {
    throw "Failed to align bundled DeepSeek Harness dependencies"
}

& $node $dsh --help | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Bundled DeepSeek Harness CLI smoke test failed"
}

$smokeHome = Join-Path $temporary "dsh-home"
$smokePort = Get-Random -Minimum 31000 -Maximum 32000
$smokeOut = Join-Path $temporary "harness.stdout.log"
$smokeErr = Join-Path $temporary "harness.stderr.log"
$previousDshHome = $env:DSH_HOME
$process = $null
try {
    $env:DSH_HOME = $smokeHome
    $process = Start-Process -FilePath $node -ArgumentList @($dsh, "--profile", "web", "--no-open", "--port", "$smokePort") -WindowStyle Hidden -RedirectStandardOutput $smokeOut -RedirectStandardError $smokeErr -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(60)
    $ready = $false
    while ([DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
        try {
            $response = Invoke-WebRequest "http://127.0.0.1:$smokePort/" -TimeoutSec 2 -UseBasicParsing
            if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 500) {
                $ready = $true
                break
            }
        } catch {}
        Start-Sleep -Milliseconds 500
    }
    if (-not $ready) {
        $errors = if (Test-Path $smokeErr) { Get-Content -LiteralPath $smokeErr -Raw } else { "" }
        throw "Bundled DeepSeek Harness web smoke test failed: $errors"
    }
} finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
    $env:DSH_HOME = $previousDshHome
}

Set-Content -LiteralPath (Join-Path $buildDestination ".deepx-runtime-ready") -Value $dshVersion -NoNewline -Encoding utf8

if (Test-Path $destination) {
    Remove-Item -LiteralPath $destination -Recurse -Force
}
Copy-Item -LiteralPath $buildDestination -Destination $destination -Recurse -Force
