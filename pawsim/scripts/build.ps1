
param(
    [switch] $clean,
    [switch] $release
)

$projectRoot = "$PSScriptRoot\.."


# $buildFolder = "$global:HOME\.rustbuilds\pawpet"
$buildFolder = "$projectRoot\target"
$pkgFolder = "$projectRoot\pkg"

if ($clean)
{
    Remove-Item -Force -Recurse $buildFolder -ErrorAction SilentlyContinue
    Remove-Item -Force -Recurse $pkgFolder -ErrorAction SilentlyContinue
}

New-Item $buildFolder -ItemType Directory -ErrorAction SilentlyContinue
New-Item $pkgFolder -ItemType Directory -ErrorAction SilentlyContinue

Set-Content $buildFolder -Stream com.dropbox.ignored -Value 1
Set-Content $pkgFolder -Stream com.dropbox.ignored -Value 1

wasm-pack build --target web

Remove-Item -Recurse -Force -Path $projectRoot/bin/*
$version = Get-Date -Format FileDateTimeUniversal

New-Item -ItemType Directory $projectRoot/bin -Force
New-Item -ItemType Directory $projectRoot/bin/$version -Force
$binPath = "$projectRoot/bin/$version"

Copy-Item $projectRoot/pkg/wasm_pawpet_bg.wasm $binPath
Copy-Item $projectRoot/pkg/wasm_pawpet.js $binPath
Copy-Item -Recurse -Force $projectRoot/assets $binPath

$f = Get-Content $projectRoot/index.html
$f = $f -replace "<VERSION>", $version
Set-Content $projectRoot/bin/index.html -Value $f

$f = Get-Content $projectRoot/index.js
$f = $f -replace "<VERSION>", $version
Set-Content $binPath/index.js -Value $f

Copy-Item $projectRoot/../sprites/*.paw -Destination $binPath/assets
