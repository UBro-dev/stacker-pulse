# Builds the Windows installers (.msi and -setup.exe).
$ErrorActionPreference = "Stop"
Write-Host "Building StackerPulse for Windows 10/11 (.msi and .exe)..." -ForegroundColor Yellow

# rustc embeds the absolute path of every source file in panic messages.
# Without this, each .exe would carry C:\Users\<your name>\.cargo\... about
# 450 times. CARGO_ENCODED_RUSTFLAGS (0x1f-separated) survives spaces in paths.
$sep = [char]0x1f
$flags = @("--remap-path-prefix=$env:USERPROFILE=~")
# A CARGO_HOME outside the profile (e.g. D:\cargo) holds the crate sources too.
if ($env:CARGO_HOME) { $flags += "--remap-path-prefix=$env:CARGO_HOME=~/.cargo" }
# The project last: when two prefixes match, rustc applies the last one.
$flags += "--remap-path-prefix=$PSScriptRoot=."
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join $sep
$watch = @($env:USERPROFILE) + @($env:CARGO_HOME | Where-Object { $_ })

Set-Location -Path "$PSScriptRoot\src-tauri"
cargo tauri build --bundles msi,nsis

# Refuse to hand over a binary that still names your account.
# .NET does not follow Set-Location, so use full paths.
$release = Join-Path $PSScriptRoot "src-tauri\target\release"
foreach ($name in @("stacker-pulse.exe", "seedscan-cli.exe")) {
    $exe = Join-Path $release $name
    $text = [System.Text.Encoding]::UTF8.GetString([System.IO.File]::ReadAllBytes($exe))
    foreach ($path in $watch) {
        if ($text.IndexOf($path, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
            throw "$exe still contains $path - do not distribute it."
        }
    }
}
Write-Host "Done: src-tauri\target\release\bundle\ (no home-folder paths inside)" -ForegroundColor Green
