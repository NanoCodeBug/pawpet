
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


Get-ChildItem $projectRoot/../sprites/*.paw
Copy-Item $projectRoot/../sprites/*.paw -Destination $projectRoot/assets


wasm-pack build --target web