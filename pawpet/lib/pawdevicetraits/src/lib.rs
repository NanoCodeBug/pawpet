#![no_std]

pub trait BatteryMonitorDevice {
    fn read(&mut self) -> u16;
}

pub enum Buttons {
    A = 1,
    B = 2,
    C = 4,
    P = 8,
    Up = 16,
    Right = 32,
    Down = 64,
    Left = 128,
}

pub trait ButtonsDevice {
    fn poll_buttons(&mut self);
    fn update_buttons(&mut self);
    fn is_held(&self, b: Buttons) -> bool;
    fn is_pressed(&self, b: Buttons) -> bool;
    fn is_released(&self, b: Buttons) -> bool;
    fn get_state(&self) -> u8;
}

pub trait SysTimerDevice {
    fn start(&mut self, ms: u32);
    fn wait_remaining<F>(&mut self, f: F) -> u32
    where
        F: FnMut();
    fn delay_ms(&mut self, ms: u32);
    fn tick(&mut self);
}

#[derive(Debug)]
pub struct CommError;

pub trait DisplayDevice {
    fn draw_pixel(&mut self, x: u8, y: u8, color: bool);
    fn update(&mut self) -> bool;
}

pub static TONE_NOTES: [u32; 80] = [
    55, 58, 62, 65, 69, 73, 78, 82, 87, 93, 98, 104, 110, 117, 123, 131, 139, 147, 156, 165, 175,
    185, 196, 208, 220, 233, 247, 262, 277, 294, 311, 330, 349, 370, 392, 415, 440, 466, 494, 523,
    554, 587, 622, 659, 698, 740, 784, 831, 880, 932, 988, 1047, 1109, 1175, 1245, 1319, 1397,
    1480, 1568, 1661, 1760, 1865, 1976, 2093, 2217, 2349, 2489, 2637, 2794, 2960, 3136, 3322, 3520,
    3729, 3951, 4186, 4435, 4699, 4978, 5274,
];

pub trait ToneDevice {
    fn tone(&self, freq: u32);
    fn no_tone(&self);
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum WatchdogTimeouts {
    Millis64 = 0, // 8 cycles of 256 hz
    Millis128 = 0x1,
    Millis256 = 0x2,
    Millis512 = 0x3,
    Seconds1 = 0x4,
    Seconds2 = 0x5,
    Seconds4 = 0x6,
    Seconds8 = 0x7,
    Seconds16 = 0x8,
    Seconds32 = 0x9,
    Seconds64 = 0xA,
    Seconds128 = 0xB,
}

pub trait WatchdogDevice {
    fn feed(&mut self);
    fn disable(&mut self);
    fn clear_disable_interrupt(&mut self);
    fn start_timeout(&mut self, period: WatchdogTimeouts);
    fn sleep(&mut self, period: WatchdogTimeouts);
}

pub enum FileWriteError {
    ChecksumFailed,
    FilesystemFull,
}

pub trait StorageDevice {
    fn load_image(&mut self, key: &str) ->  Option<&'static [u8]>;
    fn write_image(&mut self, data: &[u8], key: &str) -> Result<(), FileWriteError>;
    fn clear_cache(&mut self);
    fn format_storage(&mut self);
}
