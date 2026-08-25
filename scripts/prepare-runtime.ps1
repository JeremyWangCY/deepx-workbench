param(
    [string]$Destination = (Join-Path $PSScriptRoot "..\src-tauri\resources\runtime")
)

$ErrorActionPreference = "Stop"
$destination = [System.IO.Path]::GetFullPath($Destination)
if (Test-Path $destination) {
    Remove-Item -LiteralPath $destination -Recurse -Force
}

$releases = Invoke-RestMethod "https://nodejs.org/dist/index.json"
$release = $releases |
    Where-Object { $_.lts -and $_.version -like "v22*" } |
    Select-Object -First 1
if (-not $release) {
    throw "No Node.js 22 LTS release was found"
}

$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ("deepx-runtime-" + [guid]::NewGuid())
$archive = Join-Path $temporary "node.zip"
$nodeRoot = Join-Path $temporary ("node-" + $release.version.ToString() + "-win-x64")
$packages = @(
    "@deepseek-ai/dsh@latest",
    "@deepseek-ai/cordis-plugin-group",
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

New-Item -ItemType Directory -Force -Path $temporary,$destination | Out-Null
Invoke-WebRequest "https://nodejs.org/dist/$($release.version.ToString())/node-$($release.version.ToString())-win-x64.zip" -OutFile $archive
tar.exe -xf $archive -C $temporary
if ($LASTEXITCODE -ne 0) {
    throw "Failed to extract the Node.js runtime archive"
}
Copy-Item -Path $nodeRoot -Destination (Join-Path $destination "node") -Recurse -Force

$node = Join-Path $destination "node\node.exe"
$npm = Join-Path $destination "node\node_modules\npm\bin\npm-cli.js"
& $node $npm install --no-audit --no-fund --no-package-lock --legacy-peer-deps --fetch-timeout 30000 --fetch-retries 1 --maxsockets 8 --prefix $destination $packages
if ($LASTEXITCODE -ne 0) {
    throw "Failed to prepare the bundled DeepSeek Harness runtime"
}

$dsh = Join-Path $destination "node_modules\@deepseek-ai\dsh\lib\bin.js"
if (-not (Test-Path $dsh)) {
    throw "Bundled DeepSeek Harness entry point is missing"
}
& $node $dsh --help | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Bundled DeepSeek Harness smoke test failed"
}