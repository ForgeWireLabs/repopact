[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath,

    [string]$LogPath = (Join-Path ([IO.Path]::GetTempPath()) "repopact-wi057-desktop-processes.csv"),

    [ValidateRange(100, 5000)]
    [int]$PollMilliseconds = 250
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$resolvedExecutable = (Resolve-Path -LiteralPath $ExecutablePath).Path
$executable = Get-Item -LiteralPath $resolvedExecutable
if (-not $executable.PSIsContainer -and $executable.Name -eq "repopact-desktop.exe") {
    # Expected debug/release host executable.
} else {
    throw "ExecutablePath must point to repopact-desktop.exe"
}

$logDirectory = Split-Path -Parent ([IO.Path]::GetFullPath($LogPath))
if ($logDirectory) {
    New-Item -ItemType Directory -Path $logDirectory -Force | Out-Null
}
$resolvedLog = [IO.Path]::GetFullPath($LogPath)

function Get-DescendantProcess {
    param([Parameter(Mandatory = $true)][int]$RootPid)

    $all = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Select-Object ProcessId, ParentProcessId, Name, ExecutablePath)
    $frontier = @($RootPid)
    $seen = [Collections.Generic.HashSet[int]]::new()
    [void]$seen.Add($RootPid)
    $descendants = [Collections.Generic.List[object]]::new()
    while ($frontier.Count -gt 0) {
        $children = @($all | Where-Object {
            $frontier -contains ([int]$_.ParentProcessId) -and
            -not $seen.Contains([int]$_.ProcessId)
        })
        $frontier = @()
        foreach ($child in $children) {
            [void]$seen.Add([int]$child.ProcessId)
            [void]$descendants.Add($child)
            $frontier += [int]$child.ProcessId
        }
    }
    return @($descendants)
}

function Get-ProcessSample {
    param([Parameter(Mandatory = $true)][int]$RootPid)

    $descendants = @(Get-DescendantProcess -RootPid $RootPid)
    $gitDescendants = @($descendants | Where-Object {
        $name = [IO.Path]::GetFileNameWithoutExtension([string]$_.Name)
        $name -eq "git" -or $name.StartsWith("git-", [StringComparison]::OrdinalIgnoreCase)
    })
    [PSCustomObject]@{
        TimestampUtc = [DateTime]::UtcNow.ToString("o")
        WorkbenchPid = $RootPid
        DescendantCount = $descendants.Count
        GitCount = $gitDescendants.Count
    }
}

function Stop-OwnedProcessTree {
    param([Parameter(Mandatory = $true)][int]$RootPid)

    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        $descendants = @(Get-DescendantProcess -RootPid $RootPid)
        foreach ($child in @($descendants | Sort-Object ProcessId -Descending)) {
            try {
                Stop-Process -Id ([int]$child.ProcessId) -Force -ErrorAction Stop
            } catch [System.Management.Automation.ActionPreferenceStopException] {
                # A child may have exited between the snapshot and cleanup.
            } catch [Microsoft.PowerShell.Commands.ProcessCommandException] {
                # A child may have exited between the snapshot and cleanup.
            }
        }
        $root = Get-Process -Id $RootPid -ErrorAction SilentlyContinue
        if ($null -eq $root) {
            return
        }
        try {
            Stop-Process -Id $RootPid -Force -ErrorAction Stop
        } catch [System.Management.Automation.ActionPreferenceStopException] {
            # The root may have exited between the query and cleanup.
        } catch [Microsoft.PowerShell.Commands.ProcessCommandException] {
            # The root may have exited between the query and cleanup.
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Unable to clean up the Workbench process tree rooted at PID $RootPid"
}

$header = "timestamp_utc,workbench_pid,descendant_count,git_descendant_count"
Set-Content -LiteralPath $resolvedLog -Value $header -Encoding utf8
$workbench = Start-Process -FilePath $resolvedExecutable `
    -WorkingDirectory $executable.DirectoryName -PassThru
$workbenchPid = $workbench.Id
$maximumGitCount = 0

Write-Host "RepoPact Workbench PID: $workbenchPid"
Write-Host "Process samples: $resolvedLog"
Write-Host "Manual operator steps: select RepoPact, navigate, Refresh, switch to scratch/adopted, navigate, Refresh, switch back."
Write-Host "This harness does not click the UI or run Git commands. Close the Workbench or press Ctrl+C when finished."

try {
    while ($null -ne (Get-Process -Id $workbenchPid -ErrorAction SilentlyContinue)) {
        $sample = Get-ProcessSample -RootPid $workbenchPid
        if ($sample.GitCount -gt $maximumGitCount) {
            $maximumGitCount = $sample.GitCount
        }
        Add-Content -LiteralPath $resolvedLog -Value (
            "{0},{1},{2},{3}" -f $sample.TimestampUtc, $sample.WorkbenchPid,
            $sample.DescendantCount, $sample.GitCount) -Encoding utf8
        Write-Host ("{0} Git descendants={1} total descendants={2}" -f
            $sample.TimestampUtc, $sample.GitCount, $sample.DescendantCount)
        Start-Sleep -Milliseconds $PollMilliseconds
    }
} finally {
    if ($null -ne (Get-Process -Id $workbenchPid -ErrorAction SilentlyContinue)) {
        Stop-OwnedProcessTree -RootPid $workbenchPid
    }
    Write-Host "Maximum observed Git descendants: $maximumGitCount"
    Write-Host "Cleanup complete for Workbench PID $workbenchPid"
}
