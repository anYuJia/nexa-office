$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "../..")
cargo build --release --locked -p nexa-app
$Exe = Join-Path $Root "target/release/nexa-office.exe"
$Out = Join-Path $Root "target/package"
New-Item -ItemType Directory -Force -Path $Out | Out-Null

if (Get-Command wix -ErrorAction SilentlyContinue) {
    wix build (Join-Path $Root "packaging/windows/nexa-office.wxs") -d "NexaExe=$Exe" -o (Join-Path $Out "NexaOffice-0.1.0.msi")
} else {
    Compress-Archive -Path $Exe -DestinationPath (Join-Path $Out "NexaOffice-0.1.0-windows.zip") -Force
}
