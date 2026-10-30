param(
    [Parameter(Mandatory = $true)]
    [string]$ImagePath
)

$ErrorActionPreference = "Stop"
$resolved = (Resolve-Path -LiteralPath $ImagePath -ErrorAction Stop).Path
if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
    throw "File image tidak ditemukan: $ImagePath"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$rustDir = Join-Path $repoRoot "src-tauri"
if (-not (Test-Path -LiteralPath (Join-Path $rustDir "Cargo.toml"))) {
    throw "Folder Rust VectorForge tidak ditemukan."
}

$previous = [Environment]::GetEnvironmentVariable("VECTORFORGE_QUALITY_IMAGE", "Process")
Push-Location $rustDir
try {
    $env:VECTORFORGE_QUALITY_IMAGE = $resolved
    Write-Host "QV-010 local image quality report (source file remains local)."
    & cargo test --locked qv010_real_image_raster_feedback_quality_report -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "Tes QV-010 gagal (exit code $LASTEXITCODE)."
    }
}
finally {
    if ($null -eq $previous) {
        Remove-Item Env:VECTORFORGE_QUALITY_IMAGE -ErrorAction SilentlyContinue
    } else {
        $env:VECTORFORGE_QUALITY_IMAGE = $previous
    }
    Pop-Location
}
