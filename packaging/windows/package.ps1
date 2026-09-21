$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$Root = Resolve-Path (Join-Path $PSScriptRoot "../..")
$PackageVersion = if ($env:PACKAGE_VERSION) { $env:PACKAGE_VERSION } elseif ($env:VERSION) { $env:VERSION } else { "0.1.0" }
$ReleaseVersion = if ($env:RELEASE_VERSION) { $env:RELEASE_VERSION } else { $PackageVersion }

cargo build --release --locked -p nexa-app
$Exe = Join-Path $Root "target/release/nexa-office.exe"
$Out = Join-Path $Root "target/package"
New-Item -ItemType Directory -Force -Path $Out | Out-Null

$Msi = Join-Path $Out "NexaOffice-$ReleaseVersion.msi"
$Zip = Join-Path $Out "NexaOffice-$ReleaseVersion-windows.zip"
Remove-Item $Msi, $Zip -ErrorAction SilentlyContinue

if (-not (Get-Command wix -ErrorAction SilentlyContinue)) {
    throw "WiX v4 CLI is required to build the Windows installer"
}

wix build (Join-Path $Root "packaging/windows/nexa-office.wxs") -d "NexaExe=$Exe" -d "NexaVersion=$PackageVersion" -o $Msi
Compress-Archive -Path $Exe -DestinationPath $Zip -Force

Write-Output "package=$Msi"
Write-Output "portable=$Zip"
