# PawPet

![PawPet](pawpets.png)

A virtual pet using the atsamd21g and sharp memory lcd, runs off of 2xAAA for a couple of months. 
See [PawPet on Hacakday.io](https://hackaday.io/project/183032-paw-pet) for full gallery.

This is a rewrite in Rust of the original (and unfinished) arduino based code. That codebase is preserved at [PawPet Arduino](https://github.com/NanoCodeBug/pawpet-arduino).

# Simulator
A webasm simulator of the pawpet using wasm-pack.

This is not a cycle accurate emulator, it hooks the update/render functions to an html5 canvas and maps the core peripherals to web-asm compatible hooks.

1. Install the [Rust](https://www.rust-lang.org/tools/install) SDK

2. Install [wasm-pack](https://rustwasm.github.io/wasm-pack/) (also avaiable via `cargo install wasm-pack`)

3. Run the build script
```
cd pawsim
scripts/build.ps1
```

`pawsim/index.html` can then be live hosted from the root of the `pawsim` folder, see [pawpet web simulator](https://www.nanocodebug.com/pawpet/index.html) for a working web simulator. 

# Firmware Build Instructions

## Windows
1. Install the [Rust](https://www.rust-lang.org/tools/install) SDK

2. Run sdk-setup, which will setup the rust target dependencies
```
cd pawpet
tools/sdk-setup.ps1
```

3. Run build script
```
cd pawpet
tools/build.ps1
```

## Linux
1. Install [powershell](https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-linux?view=powershell-7.3) (i'm sorry, I'll fix this soon)

2. Install the [Rust](https://www.rust-lang.org/tools/install) SDK

2. Run sdk-setup, which will setup the rust target dependencies
```
cd pawpet
tools/sdk-setup.ps1
```

3. Run build script
```
cd pawpet
tools/build.ps1
```

## Bootloader Flashing
- TODO new hf2 bootloader that supports updating internal flash and extracting save files
- TODO have bootloader support fuse setting

1. disable boot prot fuses
2. flash bootloader generated by `pawboot` module
3. set boot brotection fuses to 16kb (bootprot fuse value 0x2)
4. disable BOD33 fuse 

## Updating Pawpet
- `pawcon` tool supports restarting the pawpet into update mode and flashing new content to its internal storage

# Licensing 

## Code
All code is licensed under the MIT license.

## Assets
All graphics, art, and 3d models are licensed under the Creative Commons Attribution-NonCommercial 4.0 International Public License.