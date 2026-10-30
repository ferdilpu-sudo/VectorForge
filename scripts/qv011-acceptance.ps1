param(
    [string[]]$ImagePaths = @(),
    [string]$ReportRoot = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$rustDir = Join-Path $repoRoot "src-tauri"
if (-not (Test-Path -LiteralPath (Join-Path $rustDir "Cargo.toml"))) {
    throw "VectorForge Rust workspace not found."
}
if ([string]::IsNullOrWhiteSpace($ReportRoot)) {
    $ReportRoot = Join-Path $env:TEMP "VectorForge-QA"
}

$runDir = Join-Path $ReportRoot ("QV011-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $runDir -Force | Out-Null
$results = [System.Collections.Generic.List[object]]::new()

function Invoke-Check {
    param(
        [string]$Name,
        [string]$WorkingDirectory,
        [string]$Executable,
        [string[]]$CommandArgs
    )
    $logFile = Join-Path $runDir ($Name + ".log")
    $start = Get-Date
    $exitCode = 1
    Push-Location $WorkingDirectory
    try {
        $previousPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $global:LASTEXITCODE = 1
        try {
            & $Executable @CommandArgs *> $logFile
            $exitCode = [int]$LASTEXITCODE
        }
        catch {
            $_ | Out-File -FilePath $logFile -Append -Encoding UTF8
        }
        finally {
            $ErrorActionPreference = $previousPreference
        }
    }
    finally {
        Pop-Location
    }
    $status = if ($exitCode -eq 0) { "PASS" } else { "FAIL" }
    $seconds = [math]::Round(((Get-Date) - $start).TotalSeconds, 2)
    $results.Add([pscustomobject]@{
        Gate = $Name
        Status = $status
        ExitCode = $exitCode
        Seconds = $seconds
        LogFile = $logFile
    })
    Write-Host ("[{0}] {1} ({2}s)" -f $status, $Name, $seconds)
    if ($exitCode -ne 0 -and (Test-Path -LiteralPath $logFile)) {
        Get-Content -LiteralPath $logFile -Tail 12 | ForEach-Object { Write-Host $_ }
    }
}

Invoke-Check "01-npm-ci" $repoRoot "npm" @("ci")
Invoke-Check "02-npm-lint" $repoRoot "npm" @("run", "lint")
Invoke-Check "03-npm-test" $repoRoot "npm" @("test")
Invoke-Check "04-npm-build" $repoRoot "npm" @("run", "build")
Invoke-Check "05-cargo-check" $rustDir "cargo" @("check", "--locked")
Invoke-Check "06-cargo-test" $rustDir "cargo" @("test", "--locked")
Invoke-Check "07-cargo-clippy" $rustDir "cargo" @("clippy", "--all-targets", "--locked", "--", "-D", "warnings")
Invoke-Check "08-cargo-fmt" $rustDir "cargo" @("fmt", "--", "--check")

$index = 0
foreach ($image in $ImagePaths) {
    $index++
    $resolved = (Resolve-Path -LiteralPath $image -ErrorAction Stop).Path
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
        throw "Image fixture is not a file: $image"
    }
    $name = "09-image-" + $index.ToString("00")
    $qaScript = Join-Path $PSScriptRoot "qv010-quality-qa.ps1"
    Invoke-Check $name $repoRoot "powershell.exe" @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $qaScript, "-ImagePath", $resolved
    )
}

$csv = Join-Path $runDir "qa-summary.csv"
$results | Export-Csv -LiteralPath $csv -NoTypeInformation -Encoding UTF8
Write-Host ""
Write-Host ("Report directory: {0}" -f $runDir)
Write-Host ("Summary: {0} PASS / {1} FAIL" -f
    @($results | Where-Object Status -eq "PASS").Count,
    @($results | Where-Object Status -eq "FAIL").Count)
if (@($results | Where-Object Status -eq "FAIL").Count -gt 0) {
    exit 1
}
exit 0
