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

pub struct EggState {
    egg: PawAnimation,
    creature: PawAnimation,
    bg: PawImage,
    tick_to_hatch: u16,
}
impl EggState {
    pub fn new() -> Self {
        Self {
            egg: PawAnimation::new((0, 4), 8),
            creature: PawAnimation::new((0, 2), 8),
            bg: PawImage::new_no_data(),
            tick_to_hatch: 200,
        }
    }
}
impl GameState for EggState {
    fn tick(
        &mut self,
        buttons: &mut impl Input,
        _tone: &impl Tone,
        _battery: &mut impl BatteryMonitor,
    ) -> StateKind {
        self.egg.tick();
        self.creature.tick();

        if buttons.is_pressed(Buttons::A) {
            return StateKind::Menu;
        }

        if self.tick_to_hatch > 0 {
            self.tick_to_hatch -= 1;
        }

        return StateKind::Egg;
    }

    fn get_fps() -> FramerateMs
    {
        return FramerateMs::Fps30;
    }

    fn load(&mut self, storage: &mut impl Storage) {
        self.egg.set_image(storage.load_image("egg_wobble"));
        self.creature.set_image(storage.load_image("pet1_idle"));
        self.bg.set_image(storage.load_image("window"));
    }

    fn draw(&mut self, display: &mut (impl Display + DrawTarget<Color = BinaryColor>)) {
        display.clear(BinaryColor::Off).ok();

        self.bg.draw(display, 2, 8);

        Line::new(Point::new(0, 40), Point::new(64, 40))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display);

        if self.tick_to_hatch == 0 {
            self.creature.draw(display, 16, 28);
        } else {
            self.egg.draw(display, 16, 28);
        }
    }
}
