
use pawbsp as bsp;

pub struct VCOMToggle {
    _pin: bsp::DispComInE,
    _tc: bsp::pac::TCC1,
}

impl VCOMToggle {
    pub fn new(vcom_pin: bsp::DispComInE, tc: bsp::pac::TCC1, pm: &mut bsp::pac::PM) -> Self {
        pm.apbcmask.modify(|_, w| w.tcc1_().set_bit());

        // PA10
        // E-> TCC1/WO[0] -> PIO_TIMER
        // F-> TCC0/WO[2] -> PIO_TIMER_ALT

        tc.ctrla.write(|w| w.swrst().set_bit());
        while tc.ctrla.read().bits() & 1 != 0 {}

        // TCC1->CTRLA.bit.ENABLE = 0;
        // while( TCC1->SYNCBUSY.bit.ENABLE) {};
        tc.ctrla.write(|w| w.enable().clear_bit());
        while tc.syncbusy.read().swrst().bit_is_set() {}

        // TCC1->CTRLA.bit.RUNSTDBY = 1;
        // TCC1->CTRLA.bit.ENABLE = 1;
        // while( TCC1->SYNCBUSY.bit.ENABLE) {};

        // Divide the GCLOCK signal by 1 giving in this case 125hz (20.83ns) TCC1 timer tick and enable the outputs
        // TCC1->CTRLA.reg |= TCC_CTRLA_PRESCALER_DIV1 | // Divide GCLK by 1
        //                    TCC_CTRLA_ENABLE;          // Enable the TCC0 output
        // while (TCC1->SYNCBUSY.bit.ENABLE) {};

        tc.ctrla
            .modify(|_, w| w.runstdby().set_bit().prescaler().div1().enable().set_bit());
        while tc.syncbusy.read().enable().bit_is_set() {}

        // TCC1->WAVE.reg |= TCC_WAVE_POL(0xF) |      // Reverse the output polarity on all TCC0 outputs
        //                   TCC_WAVE_WAVEGEN_DSBOTH; // Setup dual slope PWM on TCC0
        // while (TCC1->SYNCBUSY.bit.WAVE) {};

        tc.wave.modify(|_, w| {
            w.wavegen()
                .dsboth()
                .pol0()
                .set_bit()
                .pol1()
                .set_bit()
                .pol2()
                .set_bit()
                .pol3()
                .set_bit()
        });
        while tc.syncbusy.read().wave().bit_is_set() {}

        // Each timer counts up to a maximum or TOP value set by the PER register,
        // this determines the frequency of the PWM operation: Freq = 125hz/(2*N*PER)
        // TCC1->PER.reg = 64; // Set the FreqTcc of the PWM on TCC1
        tc.per().write(|w| unsafe { w.bits(64) });
        while tc.syncbusy.read().per().bit_is_set() {}

        // Set the PWM signal to output , PWM ds = 2*N(TOP-CCx)/Freqtcc => PWM=0 => CCx=PER, PWM=50% => CCx = PER/2
        // TCC1->CC[0].reg = 32; // TCC1 CC0 - on D11 50%
        tc.cc()[0].write(|w| unsafe { w.bits(32) });
        while tc.syncbusy.read().per().bit_is_set() {}

        Self {
            _pin: vcom_pin,
            _tc: tc,
        }
    }
}
