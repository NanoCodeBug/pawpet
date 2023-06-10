use pawdevicetraits::*;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::Pixel;
use std::f64;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;
use std::string;
use std::vec;

use pawdevicetraits::FileWriteError as FileWriteError;

extern crate web_sys;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}


pub struct BatteryMonitorSim {
    value: u16,
}

impl BatteryMonitorSim {
    pub fn new() -> Self {
        Self {
            value: 300,
        }
    }

    pub fn set(&mut self, value: u16){
        self.value = value;
    }
}

impl pawdevicetraits::BatteryMonitorDevice for BatteryMonitorSim {
    fn read(&mut self) -> u16 {
        return self.value;
    }
}

///////////////////////////////////////////////////////////////
pub struct ButtonsSim {
    _pressed: u8,
    _released: u8,
    _held: u8,
    _pollstate: u8,
    _pollstate_prev: u8,
}

impl ButtonsSim {
    pub fn new() -> Self {
        Self {
            _pressed: 0,
            _held: 0,
            _released: 0,
            _pollstate: 0,
            _pollstate_prev: 0,
        }
    }

    pub fn set_buttons(&mut self, state: u8) {
        self._pollstate = state;
    }
}

impl pawdevicetraits::ButtonsDevice for ButtonsSim {
    fn poll_buttons(&mut self) {}
    fn update_buttons(&mut self) {
        // g::g_keyPressed = ~(prevKeysState)&keysState;
        // g::g_keyReleased = prevKeysState & ~(keysState);
        // g::g_keyHeld = prevKeysState & keysState;

        self._pressed = !self._pollstate_prev & self._pollstate;
        self._released = self._pollstate_prev & !self._pollstate;
        self._held = self._pollstate_prev & self._pollstate;

        // // buttons was previuosly pressed,
        // self._released = self._pressed & !self._pollstate;

        // // was pressed and is pressed for update tick, button is held.
        // // TODO link this to a minimum amount of time
        // // TODO might also require debouncing logic, needing both min held and min released times
        // self._held = self._pressed & self._pollstate;

        // // update pressed state
        // self._pressed = self._pollstate;

        self._pollstate_prev = self._pollstate;
        // clear built up state
        self._pollstate = 0;
    }

    fn is_held(&self, b: Buttons) -> bool {
        return (self._held & (b as u8)) > 0;
    }

    fn is_pressed(&self, b: Buttons) -> bool {
        return (self._pressed & (b as u8)) > 0;
    }

    fn is_released(&self, b: Buttons) -> bool {
        return (self._released & (b as u8)) > 0;
    }

    fn get_state(&self) -> u8 {
        return self._pollstate_prev;
    }
}

///////////////////////////////////////////////////////////////
pub struct SysTimerSim {
    ellapsed_us: u64,
}

impl SysTimerSim {
    pub fn new() -> Self {
        Self {
            ellapsed_us: 0,
            // start: Instant::now(),
        }
    }
}

impl pawdevicetraits::SysTimerDevice for SysTimerSim {
    fn start(&mut self, us: u32) {
        self.ellapsed_us = us as u64;

        // TODO this is actually for framerate control
        // pass this call back up to the appropriate js element to set the framerate of the canvas?
        // and register the run() as the animation frame handler
        //
    }
    // remaining time spent waiting for frame to complete
    // TODO way to simulate this at all in sim?
    // value of 0 means we are dropping frames.
    fn wait_remaining<F>(&mut self, mut f: F) -> u32
    where
        F: FnMut(),
    {
        f();
        return 1;
    }
    fn delay_ms(&mut self, ms: u32) {}
    fn tick(&mut self) {}
}

///////////////////////////////////////////////////////////////

pub struct DisplaySim {
    canvas: CanvasRenderingContext2d,
    scale: f64,
}
impl DisplaySim {
    pub fn new(canvas: CanvasRenderingContext2d) -> Self {
        Self { canvas, scale: 2.0 }
    }
}

impl DisplayDevice for DisplaySim {
    fn draw_pixel(&mut self, x: u8, y: u8, color: bool) {
        let scale = self.scale;

        let x64: f64 = (x as f64) * scale;
        let y64: f64 = (y as f64) * scale;

        if color {
            self.canvas.set_fill_style(&JsValue::from_str(&"black"));
        } else {
            self.canvas.set_fill_style(&JsValue::from_str(&"white"));
        }
        self.canvas.fill_rect(x64, y64, scale, scale)
    }

    // display successfully updated (was not blocked by pending transfer in irl hardware)
    fn update(&mut self) -> bool {
        return true;
    }
}

#[derive(Debug)]
pub struct CommError;

impl DrawTarget for DisplaySim {
    type Color = BinaryColor;
    type Error = CommError;

    fn draw_iter<I>(&mut self, pixels: I) -> core::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // [`DrawTarget`] implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            let x = coord.x;
            let y = coord.y;
            if x >= 0 && x < 64 && y >= 0 && y < 64 {
                self.draw_pixel(x as u8, y as u8, color.is_on());
            }
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> core::result::Result<(), Self::Error> {
        self.canvas.set_fill_style(&JsValue::from_str(&"white"));

        if color == BinaryColor::On {
            self.canvas.set_fill_style(&JsValue::from_str(&"black"));
        }

        self.canvas
            .fill_rect(0.0, 0.0, self.scale * 64.0, self.scale * 64.0);
        Ok(())
    }
}

impl OriginDimensions for DisplaySim {
    fn size(&self) -> Size {
        Size::new(64, 64)
    }
}

///////////////////////////////////////////////////////////////
pub struct ToneSim {}

impl ToneSim {
    pub fn new() -> Self {
        Self {}
    }
}
impl ToneDevice for ToneSim {
    fn tone(&self, freq: u32) {}
    fn no_tone(&self) {}
}

///////////////////////////////////////////////////////////////
pub struct WatchdogSim {}
impl WatchdogSim {
    pub fn new() -> Self {
        Self {}
    }
}
impl WatchdogDevice for WatchdogSim {
    fn feed(&mut self) {}
    fn disable(&mut self) {}
    fn clear_disable_interrupt(&mut self) {}
    fn start_timeout(&mut self, period: WatchdogTimeouts) {}
    fn sleep(&mut self, period: WatchdogTimeouts) {}
}

///////////////////////////////////////////////////////////////
static mut FLASH_STORAGE: [u8; 0x20_0000] = [0; 0x20_0000];

pub struct CacheEntry {
    name: String,
    offset: usize,
}

impl CacheEntry {
    pub fn new(name: &str, offset: usize) -> Self {
        Self {
            name: name.into(),
            offset,
        }
    }
}

pub struct StorageSim {
    disk: Vec<CacheEntry>,
    offset: usize
}

impl StorageSim {
    pub fn new() -> Self {
        Self {
            disk: Vec::new(),
            offset: 0,
        }
    }

    pub fn load_file(&mut self, value: &[u8], name: String)
    {
        let offset = self.offset;
        
        unsafe{
            for i in 0..value.len()
            {
                FLASH_STORAGE[offset+i] = value[i];
            }
        }
        self.disk.push(CacheEntry::new(&name, offset));
        
        log!("load {} {:?}", name, &value[0..8]);

        self.offset += value.len();
    }
}
impl StorageDevice for StorageSim {
    fn load_image(&mut self, key: &str) ->  Option<&'static [u8]>
    {

        // enough for an empty image header
        for v in self.disk.iter()
        {
            if v.name == key
            {
                unsafe{
                    let buff = &FLASH_STORAGE[v.offset..FLASH_STORAGE.len()];
                    return Some(buff);
                }
            }
        }

        log!("image not found {}", key);

        return  None;
    }
    
    fn write_image(&mut self, data: &[u8], key: &str) -> Result<(), FileWriteError>
    {
        return Ok(());
        // not needed, files will be updated through actual disk
    }

    fn clear_cache(&mut self)
    {
        // no cache for now, thoughs imulated 16kb cache would be ideal.
    }

    fn format_storage(&mut self) 
    {
        // no cache for now, thoughs imulated 16kb cache would be ideal.
    }
}