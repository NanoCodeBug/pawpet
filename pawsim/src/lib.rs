use games::PawRunner;
use hardware::BatteryMonitorSim;
use hardware::ButtonsSim;
use hardware::DisplaySim;
use hardware::StorageSim;
use hardware::SysTimerSim;
use hardware::ToneSim;
use hardware::WatchdogSim;
use std::f64;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use games::*;

mod hardware;
mod utils;

#[wasm_bindgen]
pub struct PawPetSim {
    state: PawRunner,
    display: DisplaySim,
    watchdog: WatchdogSim,
    systimer: SysTimerSim,
    battery: BatteryMonitorSim,
    buttons: ButtonsSim,
    tone: ToneSim,
    storage: StorageSim
}

#[wasm_bindgen]
impl PawPetSim {

    #[wasm_bindgen(constructor)]
    pub fn new() -> PawPetSim {
        utils::set_panic_hook();

        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();

        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        context.begin_path();

        // Draw the outer circle.
        context
            .arc(75.0, 75.0, 50.0, 0.0, f64::consts::PI * 2.0)
            .unwrap();

        let m = PawRunner::new();
        let display = hardware::DisplaySim::new(context);
        let watchdog = hardware::WatchdogSim::new();

        let systimer = hardware::SysTimerSim::new();

        let battery = hardware::BatteryMonitorSim::new();
        let buttons = hardware::ButtonsSim::new();
        let tone = hardware::ToneSim::new();
        let storage = hardware::StorageSim::new();

        Self {
            state: m,
            display,
            watchdog,
            systimer,
            battery,
            buttons,
            tone,
            storage
        }
    }

    pub fn tick(&mut self) {
        self.state.tick(
            &mut self.display,
            &mut self.buttons,
            &mut self.watchdog,
            &mut self.tone,
            &mut self.systimer,
            &mut self.battery,
            &mut self.storage,
        );

        // TODO check suspend and other state info and update the simulator UX
    }

    pub fn set_buttons(&mut self, state: u8)
    {
        self.buttons.set_buttons(state);
    }
    pub fn set_battery(&mut self, value: u16)
    {
        self.battery.set(value);
    }

    pub fn load_file(&mut self, value:& [u8], name: String)
    {
        self.storage.load_file(value, name);
    }

    pub fn get_framerate_ms(&mut self) -> u32
    {
        return self.state.get_framerate_ms()
    }
}
