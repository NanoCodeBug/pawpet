
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use heapless::String;

use pawdevicetraits::*;

use pawdevicetraits::BatteryMonitorDevice as BatteryMonitor;
use pawdevicetraits::ButtonsDevice as Input;
use pawdevicetraits::DisplayDevice as Display;
use pawdevicetraits::ToneDevice as Tone;
use pawdevicetraits::StorageDevice as Storage;

use crate::image::PawImage;
use crate::image::PawAnimation;

use embedded_graphics::{
    mono_font::ascii::FONT_6X10,
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
    Drawable,
};

use crate::include_bytes_align_as;

use crate::GameState;
use crate::StateKind;

pub struct PawGame1 {
}
impl PawGame1 {
    pub fn new() -> Self {
        Self {}
    }
}
impl GameState for PawGame1 {
    fn tick(
        &mut self,
        _buttons: &mut impl Input,
        _tone: &impl Tone,
        _battery: &mut impl BatteryMonitor,
    ) -> StateKind {
        return StateKind::Game1;
    }

    fn draw(&mut self, _display: &mut (impl Display + DrawTarget<Color = BinaryColor>)) {}
}

