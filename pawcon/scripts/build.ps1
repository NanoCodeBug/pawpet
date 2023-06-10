
param(
    [switch] $clean,
    [switch] $release
)

$projectRoot = "$PSScriptRoot\.."


# $buildFolder = "$global:HOME\.rustbuilds\pawpet"
$buildFolder = "$projectRoot\target"


if ($clean)
{
    Remove-Item -Force -Recurse $buildFolder -ErrorAction SilentlyContinue
}

New-Item $buildFolder -ItemType Directory -ErrorAction SilentlyContinue


Set-Content $buildFolder -Stream com.dropbox.ignored -Value 1

Get-ChildItem $projectRoot/../pawpet/sprites/*.paw
Copy-Item $projectRoot/../pawpet/sprites/*.paw -Destination $projectRoot/sprites


cargo build