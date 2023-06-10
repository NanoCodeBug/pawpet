#![no_std]
#![no_main]

pub extern crate cortex_m_rt;
pub use cortex_m_rt::entry;
pub use cortex_m_rt::exception;

use rtt_target::{debug_rprintln, rtt_init_print};

// extern crate alloc;
// use embedded_alloc::Heap;
// use hal::watchdog::WatchdogTimeout;
// use core::panic::PanicInfo;
// use core::alloc::Layout;

// #[global_allocator]
// static HEAP: Heap = Heap::empty();

use hf2hid::HF2Monitor;

use bsp::hal;
use bsp::pac;
// use core::fmt::Write;
use pawbsp as bsp;

use panic_halt as _;

use cortex_m::interrupt::free as disable_interrupts;
use cortex_m::peripheral::NVIC;
use cortex_m::peripheral::SCB;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid::hid_class::HIDClass;

use bsp::{periph_alias, pin_alias};
use hal::adc::Adc;
use hal::clock::{ClockGenId, ClockSource, GenericClockController};
use hal::dmac::{DmaController, PriorityLevel};
use hal::eic::{pin::Sense, EIC};
use hal::fugit::RateExtU32;

use hal::prelude::*;
use hal::rtc;
use hal::usb::UsbBus;
use pac::{interrupt, CorePeripherals, Peripherals};

use crate::sleepy_dog::SleepyDog;
use pawdevicetraits::*;

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;
static mut USB_HID: Option<HIDClass<UsbBus>> = None;
// static mut WDT_SLEEPYDOG: Option<SleepyDog> = None;

use pawdevices::*;
static mut SYS_TIMER: Option<sys_timer::SysTimer> = None;

use hf2hid;

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

use spi_memory::series25::Flash;

#[interrupt]
fn WDT() {
    sleepy_dog::SleepyDog::get()
        .unwrap()
        .clear_disable_interrupt();
}

#[gen_hid_descriptor(
     (collection = APPLICATION, usage_page = VENDOR_DEFINED_START, usage = 0x01) = {
        (usage = 0x01,) = {
            #[item_settings data,variable,absolute] in_buf=input;
        };

        (usage = 0x01,) = {
        #[item_settings data,variable,absolute] out_buf=output;
        };

        (usage = 0x01,) = {
        #[item_settings data,variable,absolute] feature_buf=feature;
        };
     }
 )]
struct CustomBidirectionalReport {
    out_buf: [u8; 64],
    in_buf: [u8; 64],
    feature_buf: u8,
}

// const HID_DESCRIPTOR: &'static [u8] = &[
//     0x06, 0x97, 0xFF, // usage page vendor 0x97 (usage 0xff97 0x0001)
//     0x09, 0x01, // usage 1
//     0xA1, 0x01, // collection - application
//     0x15, 0x00, // logical min 0
//     0x26, 0xFF, 0x00, // logical max 255
//     0x75, 8, // report size 8
//     0x95, 64, // report count 64
//     0x09, 0x01, // usage 1
//     0x81, 0x02, // input: data, variable, absolute
//     0x95, 64, // report count 64
//     0x09, 0x01, // usage 1
//     0x91, 0x02, // output: data, variable, absolute
//     0x95, 1, // report count 1
//     0x09, 0x01, // usage 1
//     0xB1, 0x02, // feature: data, variable, absolute
//     0xC0, // end
// ];

fn init() -> (
    rtc::Rtc<rtc::ClockMode>,
    Flash<bsp::FlashSpi, bsp::FlashCs>,
    display::LS013B7DH03,
    buttons::PawButtons,
    tone::Tone,
    battery_monitor::BatteryMonitor,
) {
    rtt_init_print!();
    debug_rprintln!("Hello, world!");

    // {
    //     use core::mem::MaybeUninit;
    //     const HEAP_SIZE: usize = 1024;
    //     static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    //     unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    // }

    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let pins = bsp::Pins::new(peripherals.PORT);

    // setup core objects
    let gclk = peripherals.GCLK;
    let sysctrl = &mut peripherals.SYSCTRL;
    let nvmctrl = &mut peripherals.NVMCTRL;
    let eic = peripherals.EIC;
    let rtc = peripherals.RTC;

    // TODO enable watchdog in early setup process to catch initialization deadlocks?
    sleepy_dog::SleepyDog::new(peripherals.WDT);
    sleepy_dog::SleepyDog::get().unwrap().disable();

    let mut clocks =
        GenericClockController::with_internal_32kosc(gclk, &mut peripherals.PM, sysctrl, nvmctrl);

    // GCLK 3 used for watchdog and EIC, 32k/256 = 125 hz per tick
    let eic_watchdog_clock = clocks
        .configure_gclk_divider_and_source(ClockGenId::GCLK3, 256, ClockSource::OSCULP32K, false)
        .unwrap();

    clocks.configure_standby(ClockGenId::GCLK3, true);

    let eic_clock = clocks.eic(&eic_watchdog_clock).unwrap();
    let mut eic = EIC::init(&mut peripherals.PM, eic_clock, eic);

    let _wdt_clock = clocks.wdt(&eic_watchdog_clock).unwrap();

    // Errata 1.5.8: disable flash sleep or discard first two reads from flash on wake
    nvmctrl.ctrlb.modify(|_, w| w.sleepprm().disabled());

    // RTC Setup
    // get the internal 32k running at 1024 Hz for the RTC
    let timer_clock = clocks
        .configure_gclk_divider_and_source(ClockGenId::GCLK2, 32, ClockSource::XOSC32K, true)
        .unwrap();
    clocks.configure_standby(ClockGenId::GCLK2, true);
    let rtc_clock = clocks.rtc(&timer_clock).unwrap();
    let rtc = rtc::Rtc::clock_mode(rtc, rtc_clock.freq(), &mut peripherals.PM);

    // TODO validate that this bit is not cleared on wakeup
    core.SCB.set_sleepdeep();

    // Setup input pins
    let mut button_p = pins.button_p.into_pull_up_ei();
    button_p.sense(&mut eic, Sense::FALL);
    button_p.enable_interrupt(&mut eic);
    button_p.enable_interrupt_wake(&mut eic);

    let button_a = pins.button_a.into_pull_up_input();
    let button_b = pins.button_b.into_pull_up_input();
    let button_c = pins.button_c.into_pull_up_input();
    let button_up = pins.button_up.into_pull_up_input();
    let button_right = pins.button_right.into_pull_up_input();
    let button_down = pins.button_down.into_pull_up_input();
    let button_left = pins.button_left.into_pull_up_input();

    let buttons = buttons::PawButtons::new(
        button_a,
        button_b,
        button_c,
        button_p,
        button_up,
        button_right,
        button_down,
        button_left,
    );

    // power monitor
    let mut vmon_enable: bsp::EnableVBAT = pins.vmon_enable.into_push_pull_output();
    vmon_enable.set_high().unwrap();

    let vmon_read: bsp::InputVBAT = pin_alias!(pins.vmon_read).into();

    let adc = Adc::adc(peripherals.ADC, &mut peripherals.PM, &mut clocks);

    let bat_mon = battery_monitor::BatteryMonitor::new(adc, vmon_enable, vmon_read);

    // setup sercom2 for flash
    let flash_sercom = periph_alias!(peripherals.flash_sercom);
    let flash_spi = bsp::flash_spi(
        &mut clocks,
        8.MHz(),
        flash_sercom,
        &mut peripherals.PM,
        pins.flash_sclk,
        pins.flash_mosi,
        pins.flash_miso,
    );

    let flash_cs: bsp::FlashCs = pin_alias!(pins.flash_cs).into();

    let flash = Flash::init(flash_spi, flash_cs).unwrap();

    // setup sercom4 for display
    let spi_sercom = periph_alias!(peripherals.sharp_sercom);
    let sharp_spi = bsp::sharp_spi(
        &mut clocks,
        2.MHz(),
        spi_sercom,
        &mut peripherals.PM,
        pins.sharp_sclk,
        pins.sharp_mosi,
        pins.sharp_miso,
    );

    let disp_cs = pins.disp_cs.into_push_pull_output();

    // DMA setup
    let dmac = peripherals.DMAC;

    // Initialize DMA Controller
    let mut dmac = DmaController::init(dmac, &mut peripherals.PM);

    // Get individual handles to DMA channels
    let channels = dmac.split();
    let chan0 = channels.0.init(PriorityLevel::LVL0);

    // setup clock to 125 hz, using divsel feature
    let vcom_gclock = clocks
        .configure_gclk_divider_and_source(ClockGenId::GCLK4, 256, ClockSource::OSCULP32K, true)
        .unwrap();

    clocks.configure_standby(ClockGenId::GCLK4, true);

    let _vcom_clock = clocks.tcc0_tcc1(&vcom_gclock).unwrap();
    let vcom_pin: bsp::DispComInE = bsp::pin_alias!(pins.disp_comin).into();

    let _vcom_toggler =
        vcom_toggle::VCOMToggle::new(vcom_pin, peripherals.TCC1, &mut peripherals.PM);

    let gclk0 = clocks.gclk0();
    let tc_clock = &clocks.tc4_tc5(&gclk0).unwrap();
    let tone_beeper: bsp::BeeperE = bsp::pin_alias!(pins.beeper_e).into();

    let tone = tone::Tone::new(tone_beeper, peripherals.TC4, tc_clock, &mut peripherals.PM);
    tone.tone(0);

    unsafe {
        core.NVIC.set_priority(interrupt::WDT, 0);
        NVIC::unmask(interrupt::WDT);

        core.NVIC.set_priority(interrupt::USB, 1);
        NVIC::unmask(interrupt::USB);

        core.NVIC.set_priority(interrupt::EIC, 2);
        NVIC::unmask(interrupt::EIC);

        core.NVIC.set_priority(interrupt::DMAC, 3);
        NVIC::unmask(interrupt::DMAC);

        // core.NVIC.set_priority(interrupt::, 4);
        // NVIC::unmask(interrupt::SYSCTRL);
    }

    let mut display = display::LS013B7DH03::new(sharp_spi, chan0, disp_cs);
    display.clear(BinaryColor::Off).ok();
    display.update();
    display.set_rotation(1);

    // initialize USB
    let bus_allocator = unsafe {
        USB_ALLOCATOR = Some(bsp::usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
        ));
        USB_ALLOCATOR.as_ref().unwrap()
    };

    unsafe {
        USB_HID = Some(HIDClass::new(
            bus_allocator,
            CustomBidirectionalReport::desc(),
            10,
        ));
        // bootloader is 239a 0015
        USB_BUS = Some(
            UsbDeviceBuilder::new(bus_allocator, UsbVidPid(0x239a, 0xAA00))
                .manufacturer("Nano Heavy Industries")
                .product("Paw Pet")
                .serial_number("PAWPET001")
                .build(),
        );
    }

    let timer = sys_timer::SysTimer::new(core.SYST, &mut clocks);
    unsafe {
        SYS_TIMER = Some(timer);
    };
    // TODO disable unused peripherals according to what is enabled by default
    // TODO conditionally disable peripherals on sleep? both sercoms could probably be turned off between wakeups, dma, etc.

    // TODO setup fat file system access

    return (rtc, flash, display, buttons, tone, bat_mon);
}

// static STORAGE_DATA: &'static [u8] = include_bytes!("../../games/sprites/pet_sit.paw");

#[entry]
fn main() -> ! {
    let (_rtc, mut flash, mut display, mut buttons, tone, mut battery) = init();

    let timer = unsafe { SYS_TIMER.as_mut().unwrap() };

    let id = flash.read_jedec_id().unwrap();
    let rot = flash.read_status().unwrap();
    debug_rprintln!("JDEC {:?} {:?}", id, rot);

    let mut storage = pawdevices::storage_simple::SimpleFlashStorage::new(flash);
    // storage.write_image(STORAGE_DATA, b"petsit");

    // storage.format_storage();

    tone.tone(500);
    timer.delay_ms(250);
    tone.no_tone();
    timer.delay_ms(250);
    tone.tone(700);
    timer.delay_ms(250);
    tone.no_tone();

    let mut hid_mon: HF2Monitor = HF2Monitor::new();

    sleepy_dog::SleepyDog::get()
        .unwrap()
        .start_timeout(sleepy_dog::SleepyDogTimeout::Seconds2);

    let mut paw_runner = games::PawRunner::new();
    let watchdog: &mut SleepyDog = &mut sleepy_dog::SleepyDog::get().unwrap();

    loop {
        // TODO technically systimer start should be up here to maintain proper framerate

        // HF2 monitor loop
        // TODO move to a custom gamestate specific to the firmware version?
        // TODO move entirely to the bootloader
        // OR keep min-small implementation that responds to reboot to bootloader request only
        {
            // todo, move to usb firmware updating state to avoid allocating monitor objects on stack,

            let hid_action = hid_mon.try_recv_packet(poll_packet);

            let command = hid_action.0;
            let packet_type = hid_action.1;

            if command.is_some() {
                watchdog.disable();

                let command = command.unwrap();
                debug_rprintln!("usb: {:X?}", command as u32);

                let mut report_buff: [u8; 4096] = [0; 4096];
                let mut report_len: usize = 0;

                if matches!(packet_type, hf2hid::PacketType::MultiPacket) {
                    // hid packet is multipacket, query until all data found
                    let mut packet_complete = false;
                    while !packet_complete {
                        // debug_rprintln!("multi len: {}", report_len as u32);

                        packet_complete = hid_mon.recv_multi_data_packet(
                            poll_packet,
                            &mut report_buff,
                            &mut report_len,
                        );
                    }

                    // handl multipacket command, could be moved down to next match case at cost of stack size on small packet receive
                    // match command {
                    //     hf2hid::HF2Commands::WriteKeyValue => {}
                    //     _ => {}
                    // }
                }

                match command {
                    hf2hid::HF2Commands::ResetIntoBootloader => {
                        reset_to_boot();
                    }
                    hf2hid::HF2Commands::BinInfo => {
                        hid_mon.send_bin_info_packet(send_packet);
                    }
                    hf2hid::HF2Commands::Info => {
                        hid_mon.send_version_info_packet(send_packet);
                    }
                    // hf2hid::HF2Commands::DMesg => {}
                    // hf2hid::HF2Commands::ListKeys => {}
                    hf2hid::HF2Commands::FormatFileSys => {
                        debug_rprintln!("ERASE START");
                        hid_mon.send_empty_success_packet(send_packet);
                        storage.format_storage();
                        debug_rprintln!("ERASE DONE");
                    }
                    hf2hid::HF2Commands::WriteFile => {
                        // TODO: support single packet files for write-file
                        // aka files that are less than 54 bytes - probably never going to happen
                        // would be super wasteful of the disk min block size of 256

                        if matches!(packet_type, hf2hid::PacketType::SinglePacket) {
                            hid_mon.send_empty_error_packet(send_packet);
                        } else {
                            let key = core::str::from_utf8(&report_buff[0..16])
                                .unwrap()
                                .trim_end_matches(char::from(0));

                            debug_rprintln!("write image '{}' len {}", key, report_len as u32);

                            let res = storage.write_image(&report_buff[16..report_len], &key);
                            if res.is_ok() {
                                hid_mon.send_empty_success_packet(send_packet);
                            } else {
                                hid_mon.send_empty_error_packet(send_packet);
                            }
                        }
                    }
                    _ => {
                        hid_mon.send_empty_not_recognized_packet(send_packet);
                    }
                }
                watchdog.start_timeout(sleepy_dog::SleepyDogTimeout::Seconds2);
            }
        }

        // TODO provide logging function to the gamestate, set by wrapper
        paw_runner.tick(
            &mut display,
            &mut buttons,
            watchdog,
            &tone,
            timer,
            &mut battery,
            &mut storage,
        );

        let sleep_request = paw_runner.sleep_request();
        if sleep_request.is_some() {
            timer.disable();
            sleep(&mut buttons, sleep_request.unwrap());
            timer.enable();
        }
    }
}

pub fn sleep(buttons: &mut buttons::PawButtons, period: WatchdogTimeouts) {
    buttons.enable_interrupt();
    NVIC::mask(interrupt::USB);
    NVIC::unpend(interrupt::USB);
    // NVIC::mask(-1);
    // NVIC::unpend(-1);

    sleepy_dog::SleepyDog::get().unwrap().sleep(period);

    // ensure wakeup is handled without interrupts from other sources
    disable_interrupts(|_| {
        sleepy_dog::SleepyDog::get().unwrap().disable();
        sleepy_dog::SleepyDog::get()
            .unwrap()
            .start_timeout(sleepy_dog::SleepyDogTimeout::Seconds2);

        buttons.disable_interrupt();
        unsafe {
            NVIC::unmask(interrupt::USB);
            // NVIC::unmask(interrupt::SYSCTRL);
        }
    });
}

pub fn reset_to_boot() {
    disable_interrupts(|_| {
        NVIC::mask(interrupt::USB);
        NVIC::unpend(interrupt::USB);
        sleepy_dog::SleepyDog::get().unwrap().disable();

        const DBL_TAP_MAGIC: u32 = 0xf01669ef;
        // base address + ram size - 4 bytes
        const DBL_TAP_PTR: *mut u32 = (0x20000000 + 0x00008000 - 4) as *mut u32;
        unsafe {
            core::ptr::write_volatile(DBL_TAP_PTR, DBL_TAP_MAGIC);
        }
        SCB::sys_reset();
    });
}

fn send_packet(report: &[u8]) {
    disable_interrupts(|_| unsafe {
        USB_HID.as_mut().map(|hid| hid.push_raw_input(&report));
    })
}

fn poll_packet(report: &mut [u8; 64]) -> usize {
    let mut packet_size = 0;
    disable_interrupts(|_| unsafe {
        USB_HID.as_mut().map(|hid| {
            match hid.pull_raw_output(report) {
                Ok(size) => {
                    // hprintln!("{:02X?}", report).unwrap();
                    packet_size = size;
                }
                Err(UsbError::WouldBlock) => {
                    // no pending data
                }
                Err(err) => panic!("Error receiving data {:?}", err),
            }
        });
    });
    packet_size
}

/**
 * TODO: maybe this interrupt isn't frequent enough to catch the poll rate needed?
 * move to a usb-update state that poll this in the update loop between screen refreshes?
 */
#[interrupt]
fn USB() {
    unsafe {
        disable_interrupts(|_| {
            if let Some(usb_dev) = USB_BUS.as_mut() {
                if let Some(hid) = USB_HID.as_mut() {
                    usb_dev.poll(&mut [hid]);

                    // Make the other side happy
                    // let mut buf = [0u8; 16];
                    // let _ = serial.read(&mut buf);
                };
            };
        })
    };
}

#[interrupt]
fn EIC() {
    // Accessing registers from interrupts context is safe
    let eic = unsafe { &*pac::EIC::ptr() };

    // TODO, is critical section needed here? no data is being modified, should be fine for usb interrupts
    disable_interrupts(|_| {
        // check INTFLAG to validate interrupt source if more than one is in use
        if eic.intflag.read().extint0().bit_is_set() {
            // The interrupt request remains active until the interrupt flag is cleared,
            // the interrupt is disabled or the peripheral is reset. An interrupt flag is
            // cleared by writing a one to the corresponding bit in the INTFLAG register.
            // read more: SAM-D21DA1-Family-Data-Sheet-DS40001882G.pdf # 16.6.5 Interrupts
            eic.intflag.modify(|_, w| w.extint0().set_bit());
        }
    });
}

#[exception]
fn SysTick() {
    let timer = unsafe { SYS_TIMER.as_mut().unwrap() };
    timer.tick();
}
