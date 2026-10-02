param(
    [switch]$SkipSourceGates
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$requiredTauriCli = "2.12.0"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$bundleRoot = Join-Path $root "src-tauri\target\release\bundle"

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Label,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Command
    )

    Write-Host ""
    Write-Host "== $Label ==" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Label gagal dengan exit code $LASTEXITCODE."
    }
}

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw "H02 installer build harus dijalankan pada Windows."
}

Push-Location $root
try {
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        throw "npm tidak ditemukan."
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Cargo tidak ditemukan."
    }

    $tauriVersion = (& cargo tauri --version 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or -not $tauriVersion) {
        throw "Tauri CLI belum tersedia. Instal dengan: cargo install tauri-cli --version $requiredTauriCli --locked"
    }
    if ($tauriVersion -notmatch [regex]::Escape($requiredTauriCli)) {
        throw "Tauri CLI harus $requiredTauriCli untuk H02. Versi saat ini: $tauriVersion"
    }

    Write-Host "Tauri CLI: $tauriVersion"

    $lockedBuildProcesses = @(
        Get-Process -Name "esbuild", "node" -ErrorAction SilentlyContinue
    )
    if ($lockedBuildProcesses.Count -gt 0) {
        Write-Host ""
        Write-Warning "Proses Node/esbuild masih aktif dan dapat mengunci node_modules selama npm ci."
        $lockedBuildProcesses |
            Select-Object ProcessName, Id, Path |
            Format-Table -AutoSize
        throw "Tutup Vite/dev server atau hentikan proses Node/esbuild yang terkait VectorForge, lalu jalankan ulang H02."
    }

    if (-not $SkipSourceGates) {
        Invoke-Checked "npm ci" { npm ci }
        Invoke-Checked "Frontend lint/typecheck" { npm run lint }
        Invoke-Checked "Frontend tests" { npm test }
        Invoke-Checked "Frontend build" { npm run build }

        Push-Location (Join-Path $root "src-tauri")
        try {
            Invoke-Checked "cargo check" { cargo check }
            Invoke-Checked "cargo test" { cargo test }
            Invoke-Checked "cargo clippy" { cargo clippy --all-targets -- -D warnings }
            Invoke-Checked "cargo fmt check" { cargo fmt -- --check }
        }
        finally {
            Pop-Location
        }
    }

    if (Test-Path $bundleRoot) {
        Remove-Item $bundleRoot -Recurse -Force
    }

    Invoke-Checked "Build NSIS installer" { cargo tauri build --bundles nsis }

    $msiSucceeded = $true
    Write-Host ""
    Write-Host "== Build MSI installer ==" -ForegroundColor Cyan
    & cargo tauri build --bundles msi
    if ($LASTEXITCODE -ne 0) {
        $msiSucceeded = $false
        Write-Warning "MSI build gagal. Pada Windows, pastikan optional feature VBSCRIPT aktif jika error berasal dari WiX/light.exe."
    }

    $installers = @(
        Get-ChildItem $bundleRoot -Recurse -File -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Extension -ieq ".msi" -or $_.Name -like "*-setup.exe"
            }
    )

    if ($installers.Count -eq 0) {
        throw "Tidak ada artefak installer ditemukan di $bundleRoot."
    }

    Write-Host ""
    Write-Host "== Installer artifacts ==" -ForegroundColor Cyan

    $report = foreach ($file in $installers) {
        $sizeMiB = [Math]::Round($file.Length / 1MB, 2)
        $signature = Get-AuthenticodeSignature -FilePath $file.FullName
        [pscustomobject]@{
            Type = if ($file.Extension -ieq ".msi") { "MSI" } else { "NSIS" }
            File = $file.Name
            SizeMiB = $sizeMiB
            Under25MiB = $sizeMiB -lt 25
            SigningStatus = $signature.Status
            SHA256 = (Get-FileHash -Algorithm SHA256 -Path $file.FullName).Hash
            Path = $file.FullName
        }
    }

    $report | Sort-Object Type | Format-Table -AutoSize
    $report | Sort-Object Type | Format-List

    Write-Host ""
    Write-Host "WebView2 mode: downloadBootstrapper (runtime tidak dihitung di ukuran installer)." -ForegroundColor DarkGray
    Write-Host "NSIS install mode: currentUser." -ForegroundColor DarkGray

    if (-not $msiSucceeded) {
        exit 2
    }
}
finally {
    Pop-Location
}
