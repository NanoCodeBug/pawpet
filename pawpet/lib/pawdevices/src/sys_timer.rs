use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;

use bsp::hal;

use pawbsp as bsp;

use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;
use hal::clock::GenericClockController;
use hal::time::Hertz;

// use cortex_m::interrupt::free as disable_interrupts;

/// System timer (SysTick) as a delay provider
pub struct SysTimer {
    _sysclock: Hertz,
    syst: SYST,
    total_ticks: AtomicU32,

    tick_start: u32,
    tick_stop: u32,
}

const SYST_CSR_ENABLE: u32 = 1 << 0;
const SYST_CSR_TICKINT: u32 = 1 << 1;
const SYST_CSR_CLKSOURCE: u32 = 1 << 2;

impl pawdevicetraits::SysTimerDevice for SysTimer {
    //  33_300 - 30 fps
    //  66_600 - 15 fps
    // 200_000 -  5 fps

    // sets a counter in us up to 349,525 us
    fn start(&mut self, ms: u32) {
        self.total_ticks.store(0, Ordering::Relaxed);
        self.tick_start = 0;
        self.tick_stop = ms;
    }

    // waits the remaining time and returns how much time was waited
    fn wait_remaining<F>(&mut self, mut f: F) -> u32
    where
        F: FnMut(),
    {
        // let mut remaining_ticks = self.syst.cvr.read();
        // let mut cycles = 0;

        // while !self.syst.has_wrapped() {
        //     cycles += 1;
        // }

        // if cycles <= 1 {
        //     remaining_ticks = 0;
        // }

        // // self.syst.disable_counter();

        // return remaining_ticks / (self.sysclock.0 / 1_000_000);

        let time_ellapsed_ms = self.total_ticks.load(Ordering::Relaxed);

        if time_ellapsed_ms > self.tick_stop {
            return time_ellapsed_ms;
        }

        // let remaining = self.tick_stop - ticks;

        while self.tick_stop > self.total_ticks.load(Ordering::Relaxed) {
            f();
        }

        return time_ellapsed_ms;
    }

    // TODO, support longer blocking wait periods
    // TODO,

    fn delay_ms(&mut self, ms: u32) {
        self.start(ms);
        self.wait_remaining(|| {});
    }

    fn tick(&mut self) {
        // self.total_ticks += 1;
        let ticks = self.total_ticks.load(Ordering::Relaxed);
        self.total_ticks.store(ticks + 1, Ordering::Relaxed);
    }
}

impl SysTimer {
    /// Configures the system timer (SysTick) as a delay provider
    pub fn new(mut syst: SYST, clocks: &mut GenericClockController) -> Self {
        syst.set_clock_source(SystClkSource::Core);

        const MAX_RVR: u32 = 0x00FF_FFFF; // 16,777,215

        let sysclock: Hertz = clocks.gclk0().into();

        let tick_rate = 1_000 * (sysclock.to_Hz() / 1_000_000);
        assert!(tick_rate <= MAX_RVR);

        syst.set_reload(tick_rate);
        syst.clear_current();
        unsafe {
            syst.csr
                .write(SYST_CSR_ENABLE | SYST_CSR_CLKSOURCE | SYST_CSR_TICKINT)
        }

        SysTimer {
            syst,
            _sysclock: sysclock,
            total_ticks: AtomicU32::new(0),
            tick_start: 0,
            tick_stop: 0,
        }
    }

    pub fn disable(&mut self) {
        unsafe { self.syst.csr.write(0) }
    }
    pub fn enable(&mut self) {
        unsafe {
            self.syst
                .csr
                .write(SYST_CSR_ENABLE | SYST_CSR_CLKSOURCE | SYST_CSR_TICKINT)
        }
    }
}

// TODO
// use 1ms interrupt driven system intead
// reset for global variable on loops
// make sure to disable interrupt in sleep per eratta?
