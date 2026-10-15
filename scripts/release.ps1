# FeatherWall Automated Release Packaging Script
# Targets: x86_64-pc-windows-msvc (Windows 10/11)
$ErrorActionPreference = "Stop"

$projRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$parentRoot = Resolve-Path (Join-Path $projRoot "..")
$target = "x86_64-pc-windows-msvc"

Write-Host "================================================================================"
Write-Host "FEATHERWALL REPRODUCIBLE RELEASE BUILD"
Write-Host "================================================================================"

Write-Host "`n1. Running cargo fmt check..."
cargo fmt --all --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt failed" }

Write-Host "`n2. Running cargo check..."
cargo check --workspace --target $target
if ($LASTEXITCODE -ne 0) { throw "cargo check failed" }

Write-Host "`n3. Running cargo test..."
cargo test --workspace --target $target
if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }

Write-Host "`n4. Running cargo clippy..."
cargo clippy --workspace --all-targets --target $target
if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed" }

Write-Host "`n5. Building release binaries..."
cargo build --release --target $target
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# Locate built binaries
$binDir = Join-Path $projRoot "target\$target\release"
if (-not (Test-Path (Join-Path $binDir "featherwall.exe"))) {
    $binDir = Join-Path $projRoot "target\release"
}

# Assembly directory
$releaseDir = Join-Path $projRoot "FeatherWall-Release"
if (Test-Path $releaseDir) { Remove-Item $releaseDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $releaseDir "benchmark") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $releaseDir "diagnostics") | Out-Null

Write-Host "`n6. Assembling FeatherWall-Release artifacts..."
Copy-Item (Join-Path $binDir "featherwall.exe") (Join-Path $releaseDir "FeatherWall.exe")
Copy-Item (Join-Path $binDir "fwctl.exe") (Join-Path $releaseDir "fwctl.exe")
Copy-Item (Join-Path $projRoot "README.md") $releaseDir
Copy-Item (Join-Path $projRoot "LICENSE") $releaseDir
Copy-Item (Join-Path $projRoot "THIRD_PARTY_NOTICES.md") $releaseDir

# Copy benchmark data
Copy-Item (Join-Path $projRoot "benchmark\results.csv") (Join-Path $releaseDir "benchmark")
Copy-Item (Join-Path $projRoot "benchmark\report.md") (Join-Path $releaseDir "benchmark")
Copy-Item (Join-Path $projRoot "benchmark\system-info.txt") (Join-Path $releaseDir "benchmark")

# Copy diagnostic data
Copy-Item (Join-Path $projRoot "diagnostics\build-log.txt") (Join-Path $releaseDir "diagnostics")
Copy-Item (Join-Path $projRoot "diagnostics\test-results.txt") (Join-Path $releaseDir "diagnostics")

# Mirror to parent workspace
$parentReleaseDir = Join-Path $parentRoot "FeatherWall-Release"
if (Test-Path $parentReleaseDir) { Remove-Item $parentReleaseDir -Recurse -Force }
Copy-Item $releaseDir $parentRoot -Recurse -Force

Write-Host "`n7. Creating FeatherWall-Windows-x64-Release.zip..."
$winReleaseZip = Join-Path $parentRoot "FeatherWall-Windows-x64-Release.zip"
if (Test-Path $winReleaseZip) { Remove-Item $winReleaseZip -Force }
Compress-Archive -Path "$releaseDir\*" -DestinationPath $winReleaseZip

# Also place in FeatherWall/dist
$distDir = Join-Path $projRoot "dist"
New-Item -ItemType Directory -Force -Path $distDir | Out-Null
Copy-Item $winReleaseZip (Join-Path $distDir "FeatherWall-Windows-x64-Release.zip") -Force

Write-Host "`n8. Packaging FeatherWall-Fixed-Source.zip..."
$sourceZip = Join-Path $parentRoot "FeatherWall-Fixed-Source.zip"
if (Test-Path $sourceZip) { Remove-Item $sourceZip -Force }

# Archive source excluding target/
$sourceTemp = Join-Path $projRoot "dist\source_staging"
if (Test-Path $sourceTemp) { Remove-Item $sourceTemp -Recurse -Force }
New-Item -ItemType Directory -Force -Path $sourceTemp | Out-Null

$itemsToCopy = @(
    "src", "docs", "scripts", "benchmark", "diagnostics",
    "Cargo.toml", "Cargo.lock", "README.md", "LICENSE", "THIRD_PARTY_NOTICES.md", ".gitignore"
)
foreach ($item in $itemsToCopy) {
    $srcPath = Join-Path $projRoot $item
    if (Test-Path $srcPath) {
        Copy-Item $srcPath (Join-Path $sourceTemp $item) -Recurse -Force
    }
}
Compress-Archive -Path "$sourceTemp\*" -DestinationPath $sourceZip
Copy-Item $sourceZip (Join-Path $distDir "FeatherWall-Fixed-Source.zip") -Force
Remove-Item $sourceTemp -Recurse -Force

Write-Host "`n================================================================================"
Write-Host "RELEASE PACKAGING COMPLETE!"
Write-Host "  - Release Directory: $releaseDir"
Write-Host "  - Windows x64 Package: $winReleaseZip"
Write-Host "  - Fixed Source Package: $sourceZip"
Write-Host "================================================================================"
