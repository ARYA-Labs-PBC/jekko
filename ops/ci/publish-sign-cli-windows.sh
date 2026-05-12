#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

pwsh -NoProfile -Command @'
$files = @(
  "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-arm64\bin\jekko.exe",
  "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64\bin\jekko.exe",
  "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64-baseline\bin\jekko.exe"
)

foreach ($file in $files) {
  $sig = Get-AuthenticodeSignature $file
  if ($sig.Status -ne "Valid") {
    throw "Invalid signature for ${file}: $($sig.Status)"
  }
}

Compress-Archive -Path "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-arm64\bin\*" -DestinationPath "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-arm64.zip" -Force
Compress-Archive -Path "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64\bin\*" -DestinationPath "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64.zip" -Force
Compress-Archive -Path "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64-baseline\bin\*" -DestinationPath "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64-baseline.zip" -Force

$checksumFile = Join-Path $env:GITHUB_WORKSPACE "packages\jekko\dist\jekko-windows-checksums.txt"
Get-FileHash "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-arm64.zip" -Algorithm SHA256 | ForEach-Object { "$($_.Hash.ToLower())  jekko-windows-arm64.zip" } | Set-Content $checksumFile
Get-FileHash "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64.zip" -Algorithm SHA256 | ForEach-Object { "$($_.Hash.ToLower())  jekko-windows-x64.zip" } | Add-Content $checksumFile
Get-FileHash "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64-baseline.zip" -Algorithm SHA256 | ForEach-Object { "$($_.Hash.ToLower())  jekko-windows-x64-baseline.zip" } | Add-Content $checksumFile

if ($env:VERSION_RELEASE -ne '') {
  gh release upload "v$env:VERSION" `
    "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-arm64.zip" `
    "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64.zip" `
    "${env:GITHUB_WORKSPACE}\packages\jekko\dist\jekko-windows-x64-baseline.zip" `
    "$checksumFile" `
    --repo "$env:VERSION_REPO"
}
'@
