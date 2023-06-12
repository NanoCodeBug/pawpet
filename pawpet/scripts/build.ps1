

param(
    [switch] $clean,
    [switch] $release,
    [switch] $sprites # export and convert sprites
)

$projectRoot = "$PSScriptRoot/.."


# $buildFolder = "$global:HOME/.rustbuilds/pawpet"
$buildFolder = "target"

if ($clean) {
    Remove-Item -Force -Recurse $projectRoot/$buildFolder -ErrorAction SilentlyContinue
    Remove-Item -Force -Recurse $projectRoot/bin -ErrorAction SilentlyContinue
}

New-Item $buildFolder -ItemType Directory -ErrorAction SilentlyContinue

if (Test-Path "$projectRoot/target/atsamd") {
    Write-Host "atsamd-hal repo in target folder, run with -clean to reclone"
}
else {
    Invoke-Expression "git clone https://github.com/atsamd-rs/atsamd --depth=1 $projectRoot/target/atsamd"
    Invoke-Expression "cd $projectRoot/target/atsamd"
    Invoke-Expression "git checkout a971cbfed87fc9224aebe9e6a2f1b9f64eb90450"
    Invoke-Expression "git apply ../../patch/clock.patch"
    Invoke-Expression "cd $projectRoot"
}

# dropbox please stop
# don't @ me about building in my dropbox folder i will cry 
if ($IsLinux) {
    & attr -s com.dropbox.ignored -V 1 $buildFolder    
}
else {
    Set-Content $buildFolder -Stream com.dropbox.ignored -Value 1 -ErrorAction SilentlyContinue
}

$buildType = "debug";
if ($release) {
    $buildType = "release"
}

if ($sprites) {
    $spriteJob = Start-Job -FilePath $projectRoot/scripts/sprites.ps1
}

$buildArgs = @("build")
# $buildArgs += ("-Z build-std=panic_abort -Z build-std-features=panic_immediate_abort")
if ($release) {
    $buildArgs += ("--release")
}

# $buildARgs += "--target-dir `"$buildFolder`""

$gdb = Get-Process -Name "arm-none-eabi-gdb" -ErrorAction Ignore
if ($gdb) {
    Write-Host "Existing gdb session found, killing to release file handles."
    $gdb.Kill()
}

$proc = Start-Process cargo -ArgumentList $buildArgs -NoNewWindow -Wait -PassThru

if ($proc.ExitCode -ne 0) {
    return
}

$outputFolder = "$projectRoot/bin/$buildType"
$targetFolder = "$buildFolder/thumbv6m-none-eabi/$buildType"

New-Item -ItemType Directory $outputFolder -Force | Out-Null

# copy obj file to output
Copy-Item $targetFolder/pawos $outputFolder/pawos.elf -Force

Copy-Item $targetFolder/pawboot $outputFolder/pawboot.elf -Force

# create bin file for output
Start-Process rust-objcopy -ArgumentList ("-O binary $targetFolder/pawos $outputFolder/pawos.bin") -NoNewWindow -Wait
$binSize = $(Get-Item $outputFolder/pawos.bin).Length 

Start-Process rust-objcopy -ArgumentList ("-O binary $targetFolder/pawboot $outputFolder/pawboot.bin") -NoNewWindow -Wait
$bootSize = $(Get-Item $outputFolder/pawboot.bin).Length


# Start-Process cargo -ArgumentList "size --bin pawpet -- -A" -Wait -NoNewWindow
Write-Host 

if ($sprites) {
    # Wait for sprites export and conversion job to complete
    Write-Host -NoNewline "Sprite conversion... "
    Wait-Job $spriteJob.Id | Out-Null
    Write-Host "complete" -ForegroundColor Green
    $res = Receive-Job $spriteJob.Id;
    Write-Host $res.output
}

Write-Host -NoNewline "Binary uses "
Write-Host -NoNewline ("{0:n0}" -f $binSize) -ForegroundColor Green
Write-Host -NoNewline " bytes or "
Write-Host -NoNewline ("{0:n0}%" -f $($binSize / 248KB * 100)) -ForegroundColor Green
Write-Host " of (256kb - 8kb) flash"

Write-Host -NoNewline "Bootloader uses "
Write-Host -NoNewline ("{0:n0}" -f $bootSize) -ForegroundColor Green
Write-Host -NoNewline " bytes or "
Write-Host -NoNewline ("{0:n0}%" -f $($bootSize / 8KB * 100)) -ForegroundColor Green
Write-Host " of 8kb flash"

if ($flash) {
    # TODO flash binary via uf2, hf2, bossac, etc.

    # TODO launch rtt/connec to comm depending on debugger

    # TODO write serial->fs communication system to upload files. see how Thonny does it.
}