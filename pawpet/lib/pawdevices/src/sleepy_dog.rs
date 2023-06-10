use atsamd_hal::pac::WDT;
use cortex_m::asm;
use cortex_m::interrupt::free as disable_interrupts;

// static WDT_SLEEPING: AtomicBool = AtomicBool::new(false);

/// WatchdogTimeout enumerates usable values for configuring
/// the timeout of the watchdog peripheral.


pub use pawdevicetraits::WatchdogTimeouts as SleepyDogTimeout;

static mut WDT_SLEEPYDOG: Option<SleepyDog> = None;

pub struct SleepyDog {
    wdt: WDT,
}

impl pawdevicetraits::WatchdogDevice for SleepyDog
{
    fn feed(&mut self) {
        self.wdt.clear.write(|w| unsafe { w.clear().bits(0xA5) });
    }

    fn disable(&mut self) {
        disable_interrupts(|_| {
            // Disable the watchdog timer.
            self.wdt.ctrl.write(|w| w.enable().clear_bit());

            // Wait for watchdog timer to be disabled.
            while self.wdt.status.read().syncbusy().bit_is_set() {}
        });
    }

    fn clear_disable_interrupt(&mut self) {
        disable_interrupts(|_| {
            self.disable();

            // clear interrupt
            self.wdt.intflag.write(|w| w.ew().set_bit());

            // reset
            self.feed()
        });
    }

    /// Enables a watchdog timer to reset the processor if software is frozen
    /// or stalled.
    /// NOTE: should already be disabled before invoking, otherwise hardfault?
    fn start_timeout(&mut self, period: SleepyDogTimeout) {
        disable_interrupts(|_| {
            // Write the timeout configuration.
            self.wdt
                .config
                .write(|w| unsafe { w.per().bits(period as u8) });

            // Disable early warning interrupt
            self.wdt.intenclr.write(|w| w.ew().clear_bit());
            // self.wdt.intenset.write(|w| w.ew().set_bit());

            // Disable window mode
            // Enable the watchdog timer.
            self.wdt
                .ctrl
                .write(|w| w.enable().set_bit().wen().clear_bit());

            // Wait for watchdog timer to be enabled.
            while self.wdt.status.read().syncbusy().bit_is_set() {}
        });
    }

    fn sleep(&mut self, period: SleepyDogTimeout) {
        disable_interrupts(|_| {
            // WDT_SLEEPING.store(true, Ordering::Relaxed);

            // self.clear_disable_interrupt();
            self.disable();

            // Enable early warning interrupt
            self.wdt.intenset.write(|w| w.ew().set_bit());

            // Enable window mode
            self.wdt.ctrl.write(|w| w.wen().set_bit());

            // Write time for wake interrupt window start
            // Write timeout for wake period interrupt window end (set to max length, 2 minutes to handle wake interrupt)
            self.wdt
                .config
                .write(|w| unsafe { w.per()._16k().window().bits(period as u8) });

            // Enable the watchdog timer.
            self.wdt
                .ctrl
                .write(|w| w.wen().set_bit().enable().set_bit());

            // Wait for watchdog timer to be enabled.
            while self.wdt.status.read().syncbusy().bit_is_set() {}

            // hprintln!(
            //     "WDT: intenset:{:?} ctrl:{:?} config:{:?}",
            //     self.wdt.intenset.read().bits(),
            //     self.wdt.ctrl.read().bits(),
            //     self.wdt.config.read().bits()
            // )
            // .unwrap();
        });
        asm::dsb();
        asm::wfi();
    }
}

impl SleepyDog {
    pub fn new(wdt: WDT) {
        unsafe { WDT_SLEEPYDOG = Some(Self { wdt }) }
    }
    pub fn get() -> Option<&'static mut SleepyDog> {
        unsafe { return WDT_SLEEPYDOG.as_mut() };
    }
    
}
