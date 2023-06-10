use pawbsp as bsp;



use bsp::hal;


use hal::ehal::digital::v2::InputPin;

use atsamd_hal::eic::pin::ExtInt0;
use atsamd_hal::gpio::*;


pub struct PawButtons {
    button_a: Pin<PA18, Input<PullUp>>,
    button_b: Pin<PA17, Input<PullUp>>,
    button_c: Pin<PA15, Input<PullUp>>,
    button_p: ExtInt0<Pin<PA16, Interrupt<PullUp>>>,

    button_up: Pin<PB02, Input<PullUp>>,
    button_right: Pin<PA20, Input<PullUp>>,
    button_down: Pin<PA19, Input<PullUp>>,
    button_left: Pin<PA21, Input<PullUp>>,

    _pressed: u8,
    _released: u8,
    _held: u8,
    _pollstate: u8,
    _pollstate_prev: u8,
}

use pawdevicetraits::Buttons as Buttons;

const FLAG_A: u8 = 0;
const FLAG_B: u8 = 1;
const FLAG_C: u8 = 2;
const FLAG_P: u8 = 3;
const FLAG_UP: u8 = 4;
const FLAG_RIGHT: u8 = 5;
const FLAG_DOWN: u8 = 6;
const FLAG_LEFT: u8 = 7;

impl pawdevicetraits::ButtonsDevice for PawButtons
{
    fn poll_buttons(&mut self) {
        self._pollstate |= (self.button_a.is_low().unwrap() as u8) << FLAG_A;
        self._pollstate |= (self.button_b.is_low().unwrap() as u8) << FLAG_B;
        self._pollstate |= (self.button_c.is_low().unwrap() as u8) << FLAG_C;
        self._pollstate |= (self.button_p.is_low().unwrap() as u8) << FLAG_P;
        self._pollstate |= (self.button_up.is_low().unwrap() as u8) << FLAG_UP;
        self._pollstate |= (self.button_right.is_low().unwrap() as u8) << FLAG_RIGHT;
        self._pollstate |= (self.button_down.is_low().unwrap() as u8) << FLAG_DOWN;
        self._pollstate |= (self.button_left.is_low().unwrap() as u8) << FLAG_LEFT;
    }

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

impl PawButtons {
    pub fn new(
        button_a: Pin<PA18, Input<PullUp>>,
        button_b: Pin<PA17, Input<PullUp>>,
        button_c: Pin<PA15, Input<PullUp>>,
        button_p: ExtInt0<Pin<PA16, Interrupt<PullUp>>>,

        button_up: Pin<PB02, Input<PullUp>>,
        button_right: Pin<PA20, Input<PullUp>>,
        button_down: Pin<PA19, Input<PullUp>>,
        button_left: Pin<PA21, Input<PullUp>>,
    ) -> PawButtons {
        Self {
            button_a,
            button_b,
            button_c,
            button_p,
            button_up,
            button_right,
            button_down,
            button_left,
            _pressed: 0,
            _held: 0,
            _released: 0,
            _pollstate: 0,
            _pollstate_prev : 0,
        }
    }
    pub fn enable_interrupt(&self) {
        let eic = unsafe { &*bsp::pac::EIC::ptr() };

        //         #[cfg(feature = "samd21")]
        // ei!(ExtInt[0] {
        //     #[cfg(not(any(feature = "samd21el", feature = "samd21gl")))]
        //     PA00,
        //     PA16,
        //     #[cfg(feature = "min-samd21j")]
        //     PB00,
        //     #[cfg(feature = "min-samd21j")]
        //     PB16,
        // });
        eic.intenset.modify(|_, w| w.extint0().set_bit());
    }

    pub fn disable_interrupt(&self) {
        let eic = unsafe { &*bsp::pac::EIC::ptr() };

        eic.intenclr.modify(|_, w| w.extint0().set_bit());
    }
    
}
