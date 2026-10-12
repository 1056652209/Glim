$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
$updaterEnabled = $env:GLIM_ENABLE_UPDATER -eq '1'
$cargoArgs = @('build', '--release', '--locked', '--target', 'x86_64-pc-windows-msvc')
if ($updaterEnabled) {
    if (-not $env:GLIM_UPDATE_PUBLIC_KEY) { throw 'Configure GLIM_UPDATE_PUBLIC_KEY before enabling Windows updates' }
    $cargoArgs += @('--features', 'windows-updater')
}
cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { throw 'Windows release build failed' }
$metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'glim-app').version
$stage = Join-Path $PWD 'dist/windows-x64/Glim'
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item (Join-Path $metadata.target_directory 'x86_64-pc-windows-msvc/release/glim.exe') (Join-Path $stage 'Glim.exe') -Force
$smoke = Start-Process -FilePath (Join-Path $stage 'Glim.exe') -ArgumentList '--help' -Wait -PassThru -RedirectStandardOutput (Join-Path $PWD 'dist/windows-help.txt') -RedirectStandardError (Join-Path $PWD 'dist/windows-help-error.txt')
if ($smoke.ExitCode -ne 0) { throw "Packaged executable startup failed: $($smoke.ExitCode)" }
Copy-Item LICENSE (Join-Path $stage 'LICENSE.txt') -Force
@"
Glim $version - Windows x64
Extract this folder and open Glim.exe. Git features require Git for Windows on PATH.
Shortcuts: Ctrl+O open file, Ctrl+Shift+O open folder, Ctrl+S save, Ctrl+Shift+N new window.
Windows supports the main file/text/image/Git workspace. macOS-native document, media,
font and SQLite preview windows and the default-file-type manager are not included.
Automatic updates are available in updater-enabled releases when this portable folder is writable.
Updates verify an Ed25519-signed manifest and archive SHA-256, back up application files,
wait for Glim to quit, and preserve other files in this folder and AppData preferences.
Older Windows releases require one manual installation of 0.1.14 or later.
Protected installation folders still require a manual update; the updater does not request elevation.
This executable is not Authenticode signed. Update signing does not replace SmartScreen trust.
Download only from the project release page.
https://github.com/NeoPanic/Glim/releases
"@ | Set-Content (Join-Path $stage 'Read Me.txt') -Encoding UTF8
[xml]$bundleInfo = Get-Content 'assets/macos/Info.plist' -Raw
$buildNumber = [long]$bundleInfo.SelectSingleNode("//key[text()='CFBundleVersion']/following-sibling::string[1]").InnerText
$publicKey = if ($env:GLIM_UPDATE_PUBLIC_KEY) { $env:GLIM_UPDATE_PUBLIC_KEY.Trim() } else { '' }
$releaseMetadata = @{
    schema = 1; application = 'io.github.neopanic.glim'; platform = 'windows-x64'
    version = $version; build = $buildNumber; public_key = $publicKey; updates_enabled = $updaterEnabled
} | ConvertTo-Json
[System.IO.File]::WriteAllText((Join-Path $stage 'glim-release.json'), $releaseMetadata, [System.Text.UTF8Encoding]::new($false))
$archive = Join-Path $PWD "dist/Glim-$version-windows-x64.zip"
Compress-Archive -Path $stage -DestinationPath $archive -Force
$hash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  $([System.IO.Path]::GetFileName($archive))" | Set-Content "$archive.sha256" -Encoding ascii
Write-Output $archive
