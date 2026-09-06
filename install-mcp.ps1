[CmdletBinding()]
param(
    [string]$InstallDir = $env:KEPT_MCP_INSTALL_DIR
)

$ErrorActionPreference = 'Stop'
$Repo = 'billlzzz18/bl1nk-kept'
$BinaryName = 'bl1nk-kept-mcp'
if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'kept\bin' }

$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'x86_64-pc-windows-msvc'; break }
    'ARM64' { 'aarch64-pc-windows-msvc'; break }
    default { throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

$headers = @{ 'User-Agent' = 'bl1nk-kept-installer' }
$releases = Invoke-RestMethod -Headers $headers "https://api.github.com/repos/$Repo/releases"
$assetName = "$BinaryName-$arch.exe"
$release = $releases | Where-Object { -not $_.draft -and ($_.assets.name -contains $assetName) } | Select-Object -First 1
if (-not $release) { throw "No release contains $assetName" }

$asset = $release.assets | Where-Object name -eq $assetName | Select-Object -First 1
$checksumAsset = $release.assets | Where-Object name -eq "$assetName.sha256" | Select-Object -First 1
$temp = Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Force $temp | Out-Null
try {
    $download = Join-Path $temp $assetName
    Write-Host "Downloading $assetName from release $($release.tag_name)..."
    Invoke-WebRequest -Headers $headers -Uri $asset.browser_download_url -OutFile $download

    if ($checksumAsset) {
        $checksumPath = Join-Path $temp "$assetName.sha256"
        Invoke-WebRequest -Headers $headers -Uri $checksumAsset.browser_download_url -OutFile $checksumPath
        $expected = ((Get-Content $checksumPath -Raw) -split '\s+')[0].ToLowerInvariant()
        $actual = (Get-FileHash $download -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($expected -ne $actual) { throw 'Checksum verification failed' }
    }

    New-Item -ItemType Directory -Force $InstallDir | Out-Null
    $destination = Join-Path $InstallDir "$BinaryName.exe"
    Move-Item -Force $download $destination
    Write-Host "Installed $BinaryName $($release.tag_name) to $destination" -ForegroundColor Green
    Write-Host "Claude Code: claude mcp add -s user kept -- `"$destination`""
} finally {
    Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
}
