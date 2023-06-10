

use pawbsp as bsp;

pub static TONE_NOTES: [u32; 80] = [
    55, 58, 62, 65, 69, 73, 78, 82, 87, 93, 98, 104, 110, 117, 123, 131, 139, 147, 156, 165, 175,
    185, 196, 208, 220, 233, 247, 262, 277, 294, 311, 330, 349, 370, 392, 415, 440, 466, 494, 523,
    554, 587, 622, 659, 698, 740, 784, 831, 880, 932, 988, 1047, 1109, 1175, 1245, 1319, 1397,
    1480, 1568, 1661, 1760, 1865, 1976, 2093, 2217, 2349, 2489, 2637, 2794, 2960, 3136, 3322, 3520,
    3729, 3951, 4186, 4435, 4699, 4978, 5274,
];
pub struct Tone {
    _tc: bsp::pac::TC4,
    _beeper: bsp::BeeperE,
    _div: u32,
}

impl pawdevicetraits::ToneDevice for Tone
{
    fn tone(&self, freq: u32) {
        let tcc = self._tc.count16();
        if freq == 0 {
            self.no_tone();
            return;
        }
        tcc.cc[0].write(|w| unsafe { w.cc().bits((self._div / freq) as u16) });
        tcc.ctrla.modify(|_, w| w.enable().set_bit());
    }

    fn no_tone(&self) {
        let tcc = self._tc.count16();
        tcc.cc[0].write(|w| unsafe { w.cc().bits(0) });

        tcc.ctrla.modify(|_, w| w.enable().clear_bit());
    }
}

impl Tone {
    pub fn new(
        beeper: bsp::BeeperE,
        tc4: bsp::pac::TC4,
        clock: &atsamd_hal::clock::Tc4Tc5Clock,
        pm: &mut bsp::pac::PM,
    ) -> Tone {
        let div = clock.freq().to_Hz() / 64;
        let tcc = tc4.count16();

        pm.apbcmask.modify(|_, w| w.tc4_().set_bit());
        tcc.ctrla.write(|w| w.swrst().set_bit());
        while tcc.ctrla.read().bits() & 1 != 0 {}
        tcc.ctrla.modify(|_, w| w.enable().clear_bit());
        tcc.ctrla.modify(|_, w| w.prescaler().div64());
        tcc.ctrla.modify(|_, w| w.wavegen().mfrq());
        tcc.cc[0].write(|w| unsafe { w.cc().bits(0) });
        tcc.cc[1].write(|w| unsafe { w.cc().bits(0) });
        tcc.ctrla.modify(|_, w| w.enable().set_bit());

        Self {
            _div: div,
            _tc: tc4,
            _beeper: beeper,
        }
    }
}
