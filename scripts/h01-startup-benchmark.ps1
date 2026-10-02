param(
    [ValidateRange(3, 100)]
    [int]$Iterations = 10,
    [string]$Executable = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if ([string]::IsNullOrWhiteSpace($Executable)) {
    $Executable = Join-Path $repoRoot "src-tauri\target\release\vectorforge.exe"
}

if (-not (Test-Path $Executable)) {
    throw "Release executable not found: $Executable. Run npm run build, then cargo build --release in src-tauri."
}

function Get-Percentile {
    param(
        [double[]]$Values,
        [ValidateRange(0, 100)]
        [double]$Percentile
    )

    $sorted = @($Values | Sort-Object)
    if ($sorted.Count -eq 0) {
        return 0
    }

    $index = [Math]::Ceiling(($Percentile / 100) * $sorted.Count) - 1
    $index = [Math]::Max(0, [Math]::Min($sorted.Count - 1, $index))
    return [double]$sorted[$index]
}

$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$computer = Get-CimInstance Win32_ComputerSystem
$exeInfo = Get-Item $Executable

$rows = @()

for ($i = 1; $i -le $Iterations; $i++) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $Executable -PassThru

    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    $windowVisible = $false

    while ([DateTime]::UtcNow -lt $deadline) {
        if ($process.HasExited) {
            throw "VectorForge exited before showing a window on iteration $i."
        }

        $process.Refresh()
        if ($process.MainWindowHandle -ne 0) {
            $windowVisible = $true
            break
        }

        Start-Sleep -Milliseconds 20
    }

    $timer.Stop()

    if (-not $windowVisible) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        throw "VectorForge did not expose a main window within 15 seconds on iteration $i."
    }

    Start-Sleep -Milliseconds 500
    $process.Refresh()

    $rows += [pscustomobject]@{
        Run = $i
        StartupMs = [Math]::Round($timer.Elapsed.TotalMilliseconds, 1)
        IdleWorkingSetMiB = [Math]::Round($process.WorkingSet64 / 1MB, 1)
    }

    if (-not $process.CloseMainWindow()) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    } elseif (-not $process.WaitForExit(3000)) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }

    Start-Sleep -Milliseconds 250
}

$startup = [double[]]$rows.StartupMs
$memory = [double[]]$rows.IdleWorkingSetMiB

$summary = [pscustomobject]@{
    Timestamp = (Get-Date).ToString("o")
    OS = $os.Caption
    OSVersion = $os.Version
    CPU = $cpu.Name.Trim()
    LogicalProcessors = $computer.NumberOfLogicalProcessors
    RAMGiB = [Math]::Round($computer.TotalPhysicalMemory / 1GB, 1)
    ExecutableMiB = [Math]::Round($exeInfo.Length / 1MB, 2)
    Iterations = $Iterations
    StartupMedianMs = [Math]::Round((Get-Percentile -Values $startup -Percentile 50), 1)
    StartupP95Ms = [Math]::Round((Get-Percentile -Values $startup -Percentile 95), 1)
    IdleWorkingSetMedianMiB = [Math]::Round((Get-Percentile -Values $memory -Percentile 50), 1)
    IdleWorkingSetP95MiB = [Math]::Round((Get-Percentile -Values $memory -Percentile 95), 1)
}

Write-Host ""
Write-Host "Per-run results:"
$rows | Format-Table -AutoSize

Write-Host ""
Write-Host "H01 startup summary:"
$summary | Format-List
