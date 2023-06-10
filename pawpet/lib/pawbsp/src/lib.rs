#![no_std]
pub extern crate atsamd_hal;
pub use atsamd_hal as hal;
pub use hal::ehal;
pub use hal::pac;

use hal::clock::GenericClockController;
use hal::sercom::{
    spi,
};

use hal::time::Hertz;

use hal::typelevel::NoneT;
use hal::usb::{usb_device::bus::UsbBusAllocator, UsbBus};

hal::bsp_peripherals!(
    SERCOM2 { FlashSercom }
    SERCOM4 { SharpSercom }
);

/// Definitions related to pins and pin aliases
pub mod pins {
    use super::hal;

    hal::bsp_pins!(
        // PA00 - EXT OSC32K 
        // PA01 - EXT OSC32K
        // PA02
        // PA03
        PA04 {
            name: vmon_enable
            aliases: {
                PushPullOutput: EnableVBAT
            }
        }
        // PA05
        PA06 {
            name: disp_cs
            aliases:
            {
                PushPullOutput: SharpCs
            }
        }
        PA07 {
            name: vmon_read
            aliases: {
                AlternateB: InputVBAT
            }
        }
        PA08 {
            name: flash_mosi
            aliases: 
            {
                AlternateD: FlashMosi
            }
        }
        PA09 {
            name: flash_sclk
            aliases: 
            {
                AlternateD: FlashSclk
            }
        }
        PA10 {
            name: disp_comin
            aliases: {
                AlternateE: DispComInE
            }
        }
        // PA11 - uartrx
        PA12 {
            name: sharp_miso
            aliases: {
                AlternateD: SharpMiso
            }
        }
        PA13 {
            name: flash_cs
            aliases: {
                PushPullOutput: FlashCs
            }
        }
        PA14 {
            name: flash_miso
            aliases: {
                AlternateC: FlashMiso
            }
        }
        PA15 {
            name: button_c
        }
        PA16 {
            name: button_p
        }
        PA17 {
            name: button_b
        }
        PA18 {
            name: button_a
        }
        PA19 {
            name: button_down
        }
        PA20 {
            name: button_right
        }
        PA21 {
            name: button_left
        }
        PA24 {
            name: usb_dm
            aliases: {
                AlternateG: UsbDm
            }
        }
        PA25 {
            name: usb_dp
            aliases: {
                AlternateG: UsbDp
            }
        }
        // PA27
        // PA28 - usb host enable
        // PA30 - swclk - debug
        // PA31 - swdio - debug
        PB02 {
            name: button_up
        }
        PB08 {
            name: beeper
            aliases: {
                PushPullOutput: Beeper,
                AlternateE: BeeperE,
                AlternateF: BeeperF
            }
        }
        PB10 {
            name: sharp_mosi
            aliases: {
                AlternateD: SharpMosi
            }
        }
        PB11 {
            name: sharp_sclk
            aliases: {
                AlternateD: SharpSclk
            }
        }
        // PB22
        // PB23

    );
}
pub use pins::*;

pub type DispSpiPads = spi::Pads<SharpSercom, NoneT, SharpMosi, SharpSclk>;
pub type FlashSpiPads = spi::Pads<FlashSercom, FlashMiso, FlashMosi, FlashSclk>;

pub type SharpSpi = spi::Spi<spi::Config<DispSpiPads>, spi::Tx>;
pub type FlashSpi = spi::Spi<spi::Config<FlashSpiPads>, spi::Duplex>;

pub fn sharp_spi(
    clocks: &mut GenericClockController,
    baud: Hertz,
    sercom: SharpSercom,
    pm: &mut pac::PM,
    sclk: impl Into<SharpSclk>,
    mosi: impl Into<SharpMosi>,
    miso: impl Into<SharpMiso>,
) -> SharpSpi {
    let gclk0 = clocks.gclk0();
    let clock = clocks.sercom4_core(&gclk0).unwrap();
    let freq = clock.freq();
    let (_miso, mosi, sclk) = (miso.into(), mosi.into(), sclk.into());
    let pads = spi::Pads::default().data_out(mosi).sclk(sclk);
    spi::Config::new(pm, sercom, pads, freq)
        .baud(baud)
        .spi_mode(spi::MODE_0)
        .bit_order(spi::BitOrder::LsbFirst)
        .enable()
}

pub fn flash_spi(
    clocks: &mut GenericClockController,
    baud: Hertz,
    sercom: FlashSercom,
    pm: &mut pac::PM,
    sclk: impl Into<FlashSclk>,
    mosi: impl Into<FlashMosi>,
    miso: impl Into<FlashMiso>,
) -> FlashSpi {
    let gclk0 = clocks.gclk0();
    let clock = clocks.sercom2_core(&gclk0).unwrap();
    let freq = clock.freq();
    let (miso, mosi, sclk) = (miso.into(), mosi.into(), sclk.into());
    let pads = spi::Pads::default().data_in(miso).data_out(mosi).sclk(sclk);
    spi::Config::new(pm, sercom, pads, freq)
        .baud(baud)
        .spi_mode(spi::MODE_0)
        .enable()
}

pub fn usb_allocator(
    usb: pac::USB,
    clocks: &mut GenericClockController,
    pm: &mut pac::PM,
    dm: impl Into<UsbDm>,
    dp: impl Into<UsbDp>,
) -> UsbBusAllocator<UsbBus> {
    let gclk0 = clocks.gclk0();
    let clock = &clocks.usb(&gclk0).unwrap();
    let (dm, dp) = (dm.into(), dp.into());
    UsbBusAllocator::new(UsbBus::new(clock, pm, dm, dp, usb))
}

