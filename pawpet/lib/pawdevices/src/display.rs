use atsamd_hal::dmac::*;

use bsp::SharpCs;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::Pixel;
use pawbsp as bsp;

use bsp::ehal::digital::v2::OutputPin;
use bsp::pac::interrupt;
use bsp::SharpSpi;
use pawdevicetraits::DisplayDevice;

/**
 * TODO: spi variant for sharing, embedded hal has no generic dmac support
 */

#[repr(C)]
#[derive(Copy, Clone)]
#[repr(packed)]
pub struct Line128 {
    id: u8,
    col: [u8; 16],
    eol: u8,
}
#[repr(C)]
#[repr(packed)]
pub struct Buffer128x128 {
    command: u8,
    row: [Line128; 128],
    eoc: u16,
}

const SET: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
const CLR: [u8; 8] = [!1, !2, !4, !8, !16, !32, !64, !128];

// const LENGTH: usize = 1 + 128 * (16 + 2) + 1;
const LENGTH: usize = core::mem::size_of::<Buffer128x128>();

static mut DISP_BUFFER_WRITE: [u8; LENGTH] = [0; LENGTH];
static mut DISP_BUFFER_SEND: [u8; LENGTH] = [0; LENGTH];
static mut DISP_CS: Option<bsp::SharpCs> = None;

type TransferBuffer = &'static mut [u8; LENGTH];

static mut DISP_TRANSFER: Option<
    Transfer<Channel<Ch0, Busy>, BufferPair<TransferBuffer, bsp::SharpSpi>>,
> = None;

pub struct LS013B7DH03 {
    rotation: usize,
}

impl DrawTarget for LS013B7DH03 {
    type Color = BinaryColor;
    type Error = pawdevicetraits::CommError;

    fn draw_iter<I>(&mut self, pixels: I) -> core::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // [`DrawTarget`] implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            let x = coord.x;
            let y = coord.y;
            self.draw_pixel(x as u8, y as u8, color.is_on());
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> core::result::Result<(), Self::Error> {
        // self.cs.set_high().unwrap();
        // self.spi.as_mut().unwrap().write(&[0x04, 0x00]).unwrap();

        let b: &mut Buffer128x128;
        unsafe {
            b = core::mem::transmute(&mut DISP_BUFFER_WRITE);
        }

        let mut c: u8 = 0xFF;
        if color == BinaryColor::On {
            c = 0;
        }

        for y in b.row.iter_mut() {
            for x in y.col.iter_mut() {
                *x = c;
            }
        }

        // self.cs.set_low().unwrap();
        Ok(())
    }
}

impl OriginDimensions for LS013B7DH03 {
    fn size(&self) -> Size {
        Size::new(65, 65)
    }
}

impl pawdevicetraits::DisplayDevice for LS013B7DH03 {
    fn draw_pixel(&mut self, x: u8, y: u8, color: bool) {
        // assert!(x < 64 && y < 64);

        // TODO can be optimized with memset
        self.draw_sub_pixel(x * 2, y * 2, color);
        self.draw_sub_pixel(x * 2 + 1, y * 2, color);
        self.draw_sub_pixel(x * 2, y * 2 + 1, color);
        self.draw_sub_pixel(x * 2 + 1, y * 2 + 1, color);
    }

    fn update(&mut self) -> bool {
        unsafe {
            if DISP_TRANSFER.as_mut().unwrap().complete() {
                // swap send and write buffers
                core::mem::swap(&mut DISP_BUFFER_WRITE, &mut DISP_BUFFER_SEND);

                DISP_CS.as_mut().unwrap().set_high().unwrap();
                DISP_TRANSFER
                    .as_mut()
                    .unwrap()
                    .recycle_destination(&mut DISP_BUFFER_SEND)
                    .unwrap();
                return true;
            } else {
                // dropped frame, requested a redraw before the previous frame has finished sending
                return false;
            }
        };
    }
}

impl LS013B7DH03 {
    pub fn new(spi: SharpSpi, mut dmac_channel: Channel<Ch0, Ready>, cs: SharpCs) -> Self {
        let mut a: &mut Buffer128x128;
        let mut b: &mut Buffer128x128;

        unsafe {
            a = core::mem::transmute(&mut DISP_BUFFER_SEND);
            b = core::mem::transmute(&mut DISP_BUFFER_WRITE);
        }

        a.command = 0x01;
        b.command = 0x01;
        for i in 0..128 {
            a.row[i].id = (i + 1) as u8;
            b.row[i].id = (i + 1) as u8;

            a.row[i].eol = 0x00;
            b.row[i].eol = 0x00;
        }

        unsafe {
            DISP_CS = Some(cs);
            DISP_CS.as_mut().unwrap().set_low().unwrap()
        };

        // let xfer = spi
        //     .unwrap()
        //     .send_with_dma(&mut DISP_BUFFER, channel.unwrap(), callback2);
        dmac_channel
            .as_mut()
            .disable_interrupts(InterruptFlags::new().with_tcmpl(true));
        dmac_channel
            .as_mut()
            .enable_interrupts(InterruptFlags::new().with_tcmpl(true));

        let xfer =
            unsafe { Transfer::new_unchecked(dmac_channel, &mut DISP_BUFFER_SEND, spi, false) };
        let xfer = Some(xfer.begin(TriggerSource::SERCOM4_TX, TriggerAction::BEAT));

        unsafe {
            DISP_TRANSFER = xfer;
        }
        Self { rotation: 1 }
    }

    pub fn set_rotation(&mut self, rot: usize) {
        self.rotation = rot;
    }

    pub fn draw_sub_pixel(&mut self, x: u8, y: u8, color: bool) {
        // shift right one location to not write address space holder

        if (x > 127) || (y > 127) {
            return;
        }

        let mut x = x as usize;
        let mut y = y as usize;

        match self.rotation {
            1 => {
                // normal?
                // _swap_int16_t(x, y);
                core::mem::swap(&mut x, &mut y);
                x = 127 - x;
            }
            2 => {
                // 90
                x = 127 - x;
                y = 127 - y;
            }
            3 => {
                // 180
                core::mem::swap(&mut x, &mut y);
                y = 127 - y;
            }
            _ => {} // 270
        }

        let b: &mut Buffer128x128;
        unsafe {
            b = core::mem::transmute(&mut DISP_BUFFER_WRITE);
        }
        // let i: usize = (y * 128 + x) / 8;
        if color {
            b.row[y].col[x / 8] &= CLR[(x & 7) as usize];
        } else {
            b.row[y].col[x / 8] |= SET[(x & 7) as usize];
        }
    }
}

// only dmac is the display update finish, set CS to low to cause display to update
#[interrupt]
fn DMAC() {
    let dmac = unsafe { &*bsp::pac::DMAC::ptr() };
    let _channel: u16 = dmac.intpend.read().bits().into();

    dmac.chintflag.modify(|_, w| w.tcmpl().set_bit());

    if _channel > 1 {
        unsafe {
            DISP_CS.as_mut().unwrap().set_low().unwrap();
        }
    }
}
