

use pawdevicetraits::BatteryMonitorDevice as BatteryMonitor;
use pawdevicetraits::ButtonsDevice as Input;
use pawdevicetraits::DisplayDevice as Display;
use pawdevicetraits::ToneDevice as Tone;
use pawdevicetraits::StorageDevice as Storage;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::DrawTarget,
};

use crate::FramerateMs;

pub trait GameState {
    fn tick(
        &mut self,
        buttons: &mut impl Input,
        tone: &impl Tone,
        battery: &mut impl BatteryMonitor,
    ) -> StateKind;

    fn draw(
        &mut self,
        display: &mut (impl Display + DrawTarget<Color = BinaryColor>),
    );

    fn load(&mut self, storage: &mut impl Storage) {}

    fn need_redraw( &mut self) -> bool
    {
        return true;
    }

    fn get_fps() -> FramerateMs
    {
        return FramerateMs::Fps30;
    }
}

#[derive(PartialEq)]
pub enum StateKind {
    Main,
    Menu,
    Game1,
    Egg,
}

