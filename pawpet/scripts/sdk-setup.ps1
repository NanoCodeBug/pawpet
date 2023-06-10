
# python3 install
# pip install
# libusb-dev
# libclang 15 dev
# clang 15

# curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh


# & pip install pillow
& cargo fetch

& rustup target add thumbv6m-none-eabi
& rustup component add llvm-tools-preview

& cargo install cargo-binutils cargo-hf2 
# & cargo install wasm-pack

#linux 
# pkg-config, libusb-dev, libudev-dev, libusb-1.0-0-dev

#windows
# opensll 

# http://slproweb.com/products/Win32OpenSSL.html
# set OPENSSL_DIR=C:\OpenSSL-Win64

# vcpkg install openssl:x64-windows
# set VCPKG_ROOT=c:\path\to\vcpkg\installation
# cargo build