use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::primitives::PrimitiveStyle;
use heapless::String;

use pawdevicetraits::*;

use pawdevicetraits::BatteryMonitorDevice as BatteryMonitor;
use pawdevicetraits::ButtonsDevice as Input;
use pawdevicetraits::DisplayDevice as Display;
use pawdevicetraits::StorageDevice as Storage;
use pawdevicetraits::ToneDevice as Tone;
use crate::FramerateMs;
use crate::image::PawAnimation;
use crate::image::PawImage;

use embedded_graphics::{
    mono_font::ascii::FONT_6X10,
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyleBuilder, Rectangle},
    text::Text,
    Drawable,
};

use crate::include_bytes_align_as;

use crate::GameState;
use crate::StateKind;

pub struct EmptyState {
}
impl EmptyState {
    pub fn new() -> Self {
        Self {
        }
    }
}
impl GameState for EmptyState {
    fn tick(
        &mut self,
        buttons: &mut impl Input,
        _tone: &impl Tone,
        _battery: &mut impl BatteryMonitor,
    ) -> StateKind {


        if buttons.is_pressed(Buttons::A) {
            return StateKind::Menu;
        }

        return StateKind::Empty;
    }

    fn get_fps() -> FramerateMs
    {
        return FramerateMs::Fps30;
    }

    fn load(&mut self, storage: &mut impl Storage) {

    }

    fn draw(&mut self, display: &mut (impl Display + DrawTarget<Color = BinaryColor>)) {
        display.clear(BinaryColor::Off).ok();


    }
}
