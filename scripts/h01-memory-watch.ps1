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

function Get-PercentileValue {
    param(
        [object[]]$Values,
        [double]$Percentile
    )

    $sorted = @($Values | Sort-Object)
    $index = [Math]::Ceiling($Percentile * $sorted.Count) - 1
    $index = [Math]::Max(0, [Math]::Min($sorted.Count - 1, $index))
    return $sorted[$index]
}

$samples = @()
$rootSamples = @()
$childSamples = @()
$processPeaks = @{}
$deadline = [DateTime]::UtcNow.AddSeconds($Seconds)

Write-Host "Watching VectorForge process tree for $Seconds seconds. Run the H01 scenario now."

while ([DateTime]::UtcNow -lt $deadline) {
    if ($root.HasExited) {
        break
    }

    $ids = Get-ProcessTreeIds -RootId $root.Id
    [int64]$workingSet = 0
    [int64]$rootWorkingSet = 0
    [int64]$childWorkingSet = 0

    foreach ($id in $ids) {
        $process = Get-Process -Id $id -ErrorAction SilentlyContinue
        if (-not $process) {
            continue
        }

        $bytes = [int64]$process.WorkingSet64
        $workingSet += $bytes

        if ($id -eq $root.Id) {
            $rootWorkingSet += $bytes
        } else {
            $childWorkingSet += $bytes
        }

        $peakKey = "$($process.ProcessName)#$id"
        $mib = [Math]::Round($bytes / 1MB, 1)
        if (-not $processPeaks.ContainsKey($peakKey) -or $mib -gt $processPeaks[$peakKey]) {
            $processPeaks[$peakKey] = $mib
        }
    }

    $samples += [Math]::Round($workingSet / 1MB, 1)
    $rootSamples += [Math]::Round($rootWorkingSet / 1MB, 1)
    $childSamples += [Math]::Round($childWorkingSet / 1MB, 1)
    Start-Sleep -Milliseconds $IntervalMilliseconds
}

if ($samples.Count -eq 0) {
    throw "No memory samples were collected."
}

[pscustomobject]@{
    RootPid = $root.Id
    Samples = $samples.Count
    IntervalMs = $IntervalMilliseconds
    MedianProcessTreeMiB = Get-PercentileValue -Values $samples -Percentile 0.50
    P95ProcessTreeMiB = Get-PercentileValue -Values $samples -Percentile 0.95
    PeakProcessTreeMiB = ($samples | Measure-Object -Maximum).Maximum
    FinalProcessTreeMiB = $samples[-1]
    PeakRootMiB = ($rootSamples | Measure-Object -Maximum).Maximum
    FinalRootMiB = $rootSamples[-1]
    PeakChildrenMiB = ($childSamples | Measure-Object -Maximum).Maximum
    FinalChildrenMiB = $childSamples[-1]
} | Format-List

Write-Host "Top per-process peaks (individual peaks may occur at different timestamps):"
$processPeaks.GetEnumerator() |
    Sort-Object Value -Descending |
    Select-Object -First 10 @{Name = "Process"; Expression = { $_.Key } }, @{Name = "PeakMiB"; Expression = { $_.Value } } |
    Format-Table -AutoSize
