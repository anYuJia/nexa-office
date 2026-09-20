param(
    [string]$Binary = "target\release\nexa-office.exe",
    [int]$SettleSeconds = 30
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Binary)) {
    throw "Binary not found: $Binary. Run cargo build --release --locked -p nexa-app"
}

$process = Start-Process -FilePath $Binary -PassThru
try {
    Start-Sleep -Seconds $SettleSeconds
    $process.Refresh()

    [pscustomobject]@{
        Pid = $process.Id
        PrivateMemoryMiB = [math]::Round($process.PrivateMemorySize64 / 1MB, 2)
        WorkingSetMiB = [math]::Round($process.WorkingSet64 / 1MB, 2)
        TotalProcessorSeconds = [math]::Round($process.TotalProcessorTime.TotalSeconds, 3)
    } | Format-List

    Write-Host "Repeat samples and record PrivateMemoryMiB as the primary Windows memory metric."
}
finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id
    }
}
