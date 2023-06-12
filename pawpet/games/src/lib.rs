#![no_std]
#![allow(dead_code)]

pub mod menustate;


mod gamestate;
mod image;
use crate::image::PawImage;
use pawdevicetraits::BatteryMonitorDevice as BatteryMonitor;
use pawdevicetraits::ButtonsDevice as Input;
use pawdevicetraits::DisplayDevice as Display;
use pawdevicetraits::StorageDevice as Storage;
use pawdevicetraits::SysTimerDevice as SysTimer;
use pawdevicetraits::ToneDevice as Tone;
use pawdevicetraits::WatchdogDevice as Watchdog;

use embedded_graphics::{pixelcolor::BinaryColor, prelude::DrawTarget};

use core::fmt::Write;
use core::mem::ManuallyDrop;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use heapless::String;

use pawdevicetraits::*;

use gamestate::*;

use embedded_graphics::{
    mono_font::ascii::FONT_5X8,
    // mono_font::ascii::FONT_6X10,
    prelude::*,
    // primitives::{Circle, PrimitiveStyleBuilder, Rectangle, Triangle},
    text::Text,
    Drawable,
};

pub struct FileManager {}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FramerateMs {
    Fps30 = 33,
    Fps15 = 66,
    Fps5 = 200,
}

// ticks per update, arbitrary tick rate that should scale across fps
// leave overhead for 60 fps mode if possible

mod eggstate;
mod game1;
mod emptystate;

use menustate::MenuState;
use game1::PawGame1;
use eggstate::EggState;
use emptystate::EmptyState;

pub union StateUnion {
    menu: ManuallyDrop<MenuState>,
    game1: ManuallyDrop<PawGame1>,
    egg: ManuallyDrop<EggState>,
    empty: ManuallyDrop<EmptyState>,
}

pub struct PawRunner {
    tick: u32,
    ticks_to_sleep: u32,

    sleep_mode: Option<WatchdogTimeouts>,
    framerate: FramerateMs,
    info_text: MonoTextStyle<'static, BinaryColor>,
    state_obj: StateUnion,
    state: StateKind,

    debug: bool,
    total_frametime_ms: u32,
    dropped_frame_count: u32,
    blocked_update: u32,

    frametime_ms: [u32; 8],
    frametime_index: usize,
    battery: PawImage,
}

const INACTIVITY_SLEEP_SEC: u32 = 60;
static BATTERY_SPRITES: &'static [u8] = include_bytes_align_as!(u32, "../../../sprites/battery.paw");

impl PawRunner {
    pub fn new() -> Self {
        let info_text = MonoTextStyleBuilder::new()
            .font(&FONT_5X8)
            .text_color(BinaryColor::Off)
            .background_color(BinaryColor::On)
            .build();

        Self {
            tick: 0,
            ticks_to_sleep: FramerateMs::Fps30 as u32 * INACTIVITY_SLEEP_SEC, // ~1 minute of inactivity
            total_frametime_ms: 0,
            dropped_frame_count: 0,
            sleep_mode: None,
            info_text,
            state_obj: StateUnion {
                menu: ManuallyDrop::new(MenuState::new()),
            },
            framerate: FramerateMs::Fps30,
            state: StateKind::Menu,
            debug: false,
            blocked_update: 0,
            frametime_ms: [0; 8],
            frametime_index: 0,
            battery: PawImage::new(Some(BATTERY_SPRITES)),
        }
    }

    pub fn get_framerate_ms(&self) -> u32 {
        return self.framerate as u32;
    }

    pub fn sleep_request(&self) -> Option<WatchdogTimeouts> {
        return self.sleep_mode;
    }

    pub fn update_battery_frame(&mut self, mon: &mut impl BatteryMonitor) {
        // 1.5*2 alk, 0.8v cutoff
        // 1.4*2 nimh? 1.0v cutoff
        //
        // discharge curves are non-linear, needs tunning
        // 2.6v >- full
        // 2.4v >- 3/4  - nimh spends most time here
        // 2.2v >- 2/4
        // 2.1v >- 1/4
        // 2.01v >- empty
        // 2.0v < shutdown, refuse to boot, below cutoff voltage
        // 1.9v - display will not turn on at this voltage
        // below 2v, display battery replace logo
        //
        // 2.7v - full
        // 2.6v - 3/4
        // 2.4v - 2/4SWS
        // 2.2v - 1/4
        // 2.0v - empty
        // 1.9v - display will not turn on
        // 1.6v cutoff, display will have already been inoperable
        let bat = mon.read();
        let frame;

        if bat > 260 {
            frame = 0;
        } else if bat > 240 {
            frame = 1;
        } else if bat > 230 {
            frame = 2;
        } else if bat > 220 {
            frame = 3;
        } else {
            frame = 4;
        }

        self.battery.set_frame(frame)
    }

    pub fn tick(
        &mut self,
        display: &mut (impl Display + DrawTarget<Color = BinaryColor>),
        buttons: &mut impl Input,
        watchdog: &mut impl Watchdog,
        tone: &impl Tone,
        timer: &mut impl SysTimer,
        battery: &mut impl BatteryMonitor,
        storage: &mut impl Storage,
        // sleep device? sleep function pointer?
        // filesystem device
        // functor to ui drawing object? ui drawing object to pass down?
        // return -> request sleep?
    ) {
        // TODO move this to after the wait
        // timer.start(self.framerate as u32);

        // FEED THE DOG
        watchdog.feed();

        buttons.poll_buttons();
        buttons.update_buttons();

        // reset tick count if buttons are pressed
        if buttons.get_state() > 0 {
            self.tick = 0;
            self.sleep_mode = None;
        }

        // Update
        // Get next state and render to display
        // Get desired framerate
        // Drop old state if state changes

        // TODO split tick-draw into tick & draw?
        // allows for skipping render if no updates needed.

        let mut new_state: StateKind = StateKind::Main;

        // TOOD make this a proc gen macro
        match self.state {
            StateKind::Main => {}
            StateKind::Menu => unsafe {
                let state = &mut (*self.state_obj.menu);
                new_state = state.tick(buttons, tone, battery);

                state.draw(display);

                if new_state != self.state {
                    ManuallyDrop::drop(&mut self.state_obj.menu);
                }
            },
            StateKind::Game1 => unsafe {
                let state = &mut (*self.state_obj.game1);
                new_state = state.tick(buttons, tone, battery);

                state.draw(display);

                if new_state != self.state {
                    ManuallyDrop::drop(&mut self.state_obj.game1);
                }
            },
            StateKind::Egg => unsafe {
                let state = &mut (*self.state_obj.egg);
                new_state = state.tick(buttons, tone, battery);

                state.draw(display);

                if new_state != self.state {
                    ManuallyDrop::drop(&mut self.state_obj.egg);
                }
            },
            StateKind::Empty => unsafe {
                let state = &mut (*self.state_obj.empty);
                new_state = state.tick(buttons, tone, battery);

                state.draw(display);

                if new_state != self.state {
                    ManuallyDrop::drop(&mut self.state_obj.empty);
                }
            },
        }

        // Update union to next state
        if new_state != self.state {
            match new_state {
                StateKind::Main => {}
                StateKind::Menu => {
                    // TOOD make this a proc gen macro
                    unsafe {
                        *self.state_obj.menu = MenuState::new();
                        (*self.state_obj.menu).load(storage);
                        self.framerate = MenuState::get_fps();
                    }
                }
                StateKind::Game1 => unsafe {
                    *self.state_obj.game1 = PawGame1::new();
                    (*self.state_obj.game1).load(storage);
                    self.framerate = PawGame1::get_fps();
                },
                StateKind::Egg => unsafe {
                    *self.state_obj.egg = EggState::new();
                    (*self.state_obj.egg).load(storage);
                    self.framerate = EggState::get_fps();
                },
                StateKind::Empty => unsafe {
                    *self.state_obj.empty = EmptyState::new();
                    (*self.state_obj.empty).load(storage);
                    self.framerate = EmptyState::get_fps();
                },
            }

            self.ticks_to_sleep = self.framerate as u32 /*(FPS)*/ * INACTIVITY_SLEEP_SEC;
            // ~1 minute of inactivity
        }

        self.state = new_state;

        // let time = disable_interrupts(|_| rtc.current_time());
        // hprintln!("JDEC {:?} {}", id.device_id(), rot).ok();

        // global toggle debug menu
        if buttons.is_held(Buttons::P) && buttons.is_held(Buttons::Left) {
            self.debug = true;
        }
        if buttons.is_held(Buttons::P) && buttons.is_held(Buttons::Right) {
            self.debug = false;
        }

        self.tick += 1;

        // enter sleep, don't exit until button press occurs (or other event)
        if self.tick > self.ticks_to_sleep {
            self.sleep_mode = Some(WatchdogTimeouts::Seconds64);
        }

        // display.clear(BinaryColor::Off).ok();
        // debug stats

        {
            // frame stats tracking
            // let mut cpu_avg_ms: u32 = 0;
            // for p in self.frametime_ms {
            //     cpu_avg_ms += p;
            // }
            // cpu_avg_ms = cpu_avg_ms / self.frametime_ms.len() as u32;

            // let mut cpu_per: u32 = 100;
            // if cpu_avg_ms < self.framerate as u32 {
            //     cpu_per = cpu_avg_ms * 100 / (self.framerate as u32);
            // }

            let mut cpu_peek_ms: u32 = 0;

            for p in self.frametime_ms {
                if p > cpu_peek_ms {
                    cpu_peek_ms = p;
                }
            }

            self.frametime_index += 1;
            self.frametime_index = self.frametime_index % self.frametime_ms.len();
            self.frametime_ms[self.frametime_index] = self.total_frametime_ms;

            if self.debug {
                let mut s: String<256> = String::new();
                // asm::bkpt();

                write!(
                    s,
                    "pkms:{:3.0}\n{:08b}\ndrop:{} {}",
                    cpu_peek_ms,
                    buttons.get_state(),
                    self.dropped_frame_count,
                    self.blocked_update,
                )
                .ok();
                write!(s, "\n{}", battery.read()).ok();

                if self.sleep_mode.is_some() {
                    write!(s, "\nSLEEP").ok();
                }

                Text::new(&s, Point::new(0, 6), self.info_text)
                    .draw(display)
                    .ok();
            } else {
                let mut s: String<64> = String::new();

                write!(s, "{:3.0}", cpu_peek_ms).ok();

                Text::new(&s, Point::new(0, 6), self.info_text)
                    .draw(display)
                    .ok();
            }
        }

        self.update_battery_frame(battery);

        self.battery.draw(display, 48, 0);

        // display was busy
        let display_busy = !display.update();

        self.total_frametime_ms = timer.wait_remaining(|| {
            // spin button polling while waiting for timer to ellapse
            buttons.poll_buttons();
            // TODO, future, instead sleep?
        });
        timer.start(self.framerate as u32);

        // if  self.frametime_index == 0
        // {
        //     for p in self.frametime_us
        //     {
        //         hprintln!("{}", p).unwrap();

        //     }
        //     hprintln!("---").unwrap();

        // }

        if self.total_frametime_ms > (self.framerate as u32) {
            self.dropped_frame_count += 1;
        }

        if display_busy {
            self.blocked_update += 1;
        }
    }
}

#[macro_use]
mod macros {
    #[repr(C)] // guarantee 'bytes' comes after '_align'
    pub struct AlignedAs<Align, Bytes: ?Sized> {
        pub _align: [Align; 0],
        pub bytes: Bytes,
    }
    #[macro_export]
    macro_rules! include_bytes_align_as {
        ($align_ty:ty, $path:literal) => {{
            // const block expression to encapsulate the static
            use $crate::macros::AlignedAs;

            // this assignment is made possible by CoerceUnsized
            static ALIGNED: &AlignedAs<$align_ty, [u8]> = &AlignedAs {
                _align: [],
                bytes: *include_bytes!($path),
            };

            &ALIGNED.bytes
        }};
    }
}
