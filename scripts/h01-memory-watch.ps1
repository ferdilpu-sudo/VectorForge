param(
    [ValidateRange(5, 600)]
    [int]$Seconds = 60,
    [ValidateRange(100, 5000)]
    [int]$IntervalMilliseconds = 500
)

$ErrorActionPreference = "Stop"
$root = Get-Process -Name "vectorforge" -ErrorAction SilentlyContinue |
    Sort-Object StartTime -Descending |
    Select-Object -First 1

if (-not $root) {
    throw "VectorForge is not running. Start the release executable first."
}

function Get-ProcessTreeIds {
    param([int]$RootId)

    $snapshot = Get-CimInstance Win32_Process |
        Select-Object ProcessId, ParentProcessId
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    [void]$ids.Add($RootId)

    $changed = $true
    while ($changed) {
        $changed = $false
        foreach ($entry in $snapshot) {
            $parent = [int]$entry.ParentProcessId
            $child = [int]$entry.ProcessId
            if ($ids.Contains($parent) -and -not $ids.Contains($child)) {
                [void]$ids.Add($child)
                $changed = $true
            }
        }
    }

    return @($ids)
}

$samples = @()
$deadline = [DateTime]::UtcNow.AddSeconds($Seconds)

Write-Host "Watching VectorForge process tree for $Seconds seconds. Run the H01 scenario now."

while ([DateTime]::UtcNow -lt $deadline) {
    if ($root.HasExited) {
        break
    }

    $ids = Get-ProcessTreeIds -RootId $root.Id
    [int64]$workingSet = 0
    foreach ($id in $ids) {
        $process = Get-Process -Id $id -ErrorAction SilentlyContinue
        if ($process) {
            $workingSet += $process.WorkingSet64
        }
    }

    $samples += [Math]::Round($workingSet / 1MB, 1)
    Start-Sleep -Milliseconds $IntervalMilliseconds
}

if ($samples.Count -eq 0) {
    throw "No memory samples were collected."
}

$sorted = @($samples | Sort-Object)
$p95Index = [Math]::Ceiling(0.95 * $sorted.Count) - 1
$p95Index = [Math]::Max(0, [Math]::Min($sorted.Count - 1, $p95Index))
$medianIndex = [Math]::Ceiling(0.50 * $sorted.Count) - 1
$medianIndex = [Math]::Max(0, [Math]::Min($sorted.Count - 1, $medianIndex))

[pscustomobject]@{
    RootPid = $root.Id
    Samples = $samples.Count
    IntervalMs = $IntervalMilliseconds
    MedianProcessTreeMiB = $sorted[$medianIndex]
    P95ProcessTreeMiB = $sorted[$p95Index]
    PeakProcessTreeMiB = ($sorted | Measure-Object -Maximum).Maximum
} | Format-List
