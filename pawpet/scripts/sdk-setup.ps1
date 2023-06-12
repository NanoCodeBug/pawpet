$projectRoot = "$PSScriptRoot/.."
$buildFolder = "target"

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

# python3 install
# pip install
# & pip install pillow
& cargo fetch

& rustup target add thumbv6m-none-eabi
& rustup component add llvm-tools-preview

& cargo install cargo-binutils cargo-hf2 
# & cargo install wasm-pack

# linux 
# build-essential libusb-dev libusb-1.0-0-dev libudev-dev pkg-config 