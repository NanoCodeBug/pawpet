

param(
    [switch] $release,
    [switch] $bmp,
    [switch] $segger
)

$gdb = "C:\Program Files (x86)\Arm GNU Toolchain arm-none-eabi\12.2 rel1\bin\arm-none-eabi-gdb.exe"

$buildType = "debug";
if ($release) {
    $buildType = "release"
}

if ($segger) {
    $jlink = Get-Process -Name JLinkGDBServer -ErrorAction Ignore
    if (-not $jlink) { 
        Start-Process -FilePath 'C:\Program Files\SEGGER\JLink\JLinkGDBServer.exe' -ArgumentList ("-if", "SWD", "-device", "ATSAMD21G18") -NoNewWindow 
    }

    & $gdb -x scripts\gdb_segger .\bin\$buildType\pawos.elf
}
elseif ($bmp) {
    & $gdb -x scripts\gdb_bmp .\bin\$buildType\pawos.elf
}

