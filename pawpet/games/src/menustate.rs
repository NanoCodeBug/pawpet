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

pub struct MenuState {
    str: String<64>,
    selection: usize,
    menu_items: [String<8>; 5],
    redraw: bool,
}
impl MenuState {
    pub fn new() -> Self {
        Self {
            str: String::new(),
            selection: 0,
            menu_items: [
                "Egg".into(),
                "Empty".into(),
                "Item 3".into(),
                "Item 4".into(),
                "Item 5".into(),
            ],
            redraw: true,
        }
    }
}

impl GameState for MenuState {
    fn tick(
        &mut self,
        buttons: &mut impl Input,
        _tone: &impl Tone,
        _battery: &mut impl BatteryMonitor,
    ) -> StateKind {
        self.redraw = true;

        if buttons.is_pressed(Buttons::Up) {
            if self.selection > 0 {
                self.selection -= 1;
            }
            self.redraw = true;
        } else if buttons.is_pressed(Buttons::Down) {
            self.selection += 1;
            if self.selection >= self.menu_items.len() {
                self.selection = self.menu_items.len() - 1;
            }

            self.redraw = true;
        } else if buttons.is_pressed(Buttons::P) {
            match self.selection
            {
                0 => {
                    return StateKind::Egg
                },
                1 => {
                    return StateKind::Empty
                },
                _ => {}
            }

        }

        return StateKind::Menu;
    }

    fn draw(&mut self, display: &mut (impl Display + DrawTarget<Color = BinaryColor>)) {
        display.clear(BinaryColor::Off).ok();

        let white_text: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let black_text: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::Off)
            .background_color(BinaryColor::On)
            .build();

        // self.str.clear();
        // write!(self.str, "{:08b}\n{}", buttons.get_pressed(),self.selection).ok();

        for i in 0..self.menu_items.len() {
            let x = 0;
            let y = 10 * (i as i32) + 15;
            let mut style = white_text;

            if self.selection == i {
                style = black_text;
            }

            Text::new(&self.menu_items[i], Point::new(x, y), style)
                .draw(display)
                .ok();
        }
        Text::new(&self.str, Point::new(2, 8), white_text)
            .draw(display)
            .ok();

    }

    fn need_redraw(&mut self) -> bool {
        return self.redraw;
    }
}
