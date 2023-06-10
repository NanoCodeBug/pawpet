#![no_std]
#![no_main]

use hf2hid;

use bsp::hal;
use bsp::pac;
use pawbsp as bsp;

use panic_halt as _;

use cortex_m::interrupt::free as disable_interrupts;
use cortex_m::peripheral::NVIC;
use cortex_m::peripheral::SCB;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_hid::hid_class::HIDClass;

pub use cortex_m_rt::entry;
use hal::clock::{ GenericClockController};

use hal::usb::UsbBus;
use pac::{interrupt, CorePeripherals, Peripherals};

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;
static mut USB_HID: Option<HIDClass<UsbBus>> = None;

const HID_DESCRIPTOR: &'static [u8] = &[
    0x06, 0x97, 0xFF, // usage page vendor 0x97 (usage 0xff97 0x0001)
    0x09, 0x01, // usage 1
    0xA1, 0x01, // collection - application
    0x15, 0x00, // logical min 0
    0x26, 0xFF, 0x00, // logical max 255
    0x75, 8, // report size 8
    0x95, 64, // report count 64
    0x09, 0x01, // usage 1
    0x81, 0x02, // input: data, variable, absolute
    0x95, 64, // report count 64
    0x09, 0x01, // usage 1
    0x91, 0x02, // output: data, variable, absolute
    0x95, 1, // report count 1
    0x09, 0x01, // usage 1
    0xB1, 0x02, // feature: data, variable, absolute
    0xC0, // end
];

fn init() {
    let mut peripherals = Peripherals::take().expect("");
    let mut core = CorePeripherals::take().expect("");
    let pins = bsp::Pins::new(peripherals.PORT);

    // setup core objects
    let gclk = peripherals.GCLK;
    let sysctrl = &mut peripherals.SYSCTRL;
    let nvmctrl = &mut peripherals.NVMCTRL;
    let _eic = peripherals.EIC;
    let _rtc = peripherals.RTC;

    let mut clocks =
        GenericClockController::with_internal_32kosc(gclk, &mut peripherals.PM, sysctrl, nvmctrl);

    unsafe {
        core.NVIC.set_priority(interrupt::USB, 1);
        NVIC::unmask(interrupt::USB);
    }

    // initialize USB
    let bus_allocator = unsafe {
        USB_ALLOCATOR = Some(bsp::usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
        ));
        USB_ALLOCATOR.as_ref().expect("")
    };

    unsafe {
        USB_HID = Some(HIDClass::new(
            bus_allocator,
            HID_DESCRIPTOR,
            100,
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

    // TODO disable unused peripherals according to what is enabled by default
    // TODO conditionally disable peripherals on sleep? both sercoms could probably be turned off between wakeups, dma, etc.

    // TODO setup fat file system access
    // TODO listen for serial commands to send/receive fat fs data
}

#[entry]
fn main() -> ! {
    // const DBL_TAP_MAGIC: u32 = 0xf01669ef;
    // // base address + ram size - 4 bytes
    // const DBL_TAP_PTR: *mut u32 = (0x20000000 + 0x00008000 - 4) as *mut u32;

    // check if magic pointer is set, otherwise reboot to app.
    // clear magic pointer

    // double tap detect for bootloader
    // check for external reset was triggered to go to bootloader?

    init();

    let _tick: u32 = 0;
    // let id = flash.read_jedec_id()..unwrap();
    let _remaining_frametime_us: u32 = 0;
    let _dropped_frame_count: u32 = 0;
    let _sleep_mode: u32 = 0;

    let mut hid_mon: hf2hid::HF2Monitor = hf2hid::HF2Monitor::new();
    loop {
        {
            // todo, move to usb firmware updating state to avoid allocating monitor objects on stack,

            let hid_action = hid_mon.try_recv_packet(poll_packet);

            let command = hid_action.0;
            let packet_type = hid_action.1;

            if command.is_some() {

                let command = command.unwrap();
                // debug_rprintln!("usb: {:X?}", command as u32);

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
                        reset_to_app();
                    }
                    hf2hid::HF2Commands::BinInfo => {
                        hid_mon.send_bin_info_packet(send_packet);
                    }
                    hf2hid::HF2Commands::Info => {
                        hid_mon.send_version_info_packet(send_packet);
                    }
                    // hf2hid::HF2Commands::DMesg => {}
                    // hf2hid::HF2Commands::ListKeys => {}
                    // hf2hid::HF2Commands::FormatFileSys => {
                    //     hid_mon.send_empty_success_packet(send_packet);
                    //     storage.format_storage();
                    // }
                    // hf2hid::HF2Commands::WriteFile => {
                    //     // TODO: support single packet files for write-file
                    //     // aka files that are less than 54 bytes - probably never going to happen
                    //     // would be super wasteful of the disk min block size of 256

                    //     if matches!(packet_type, hf2hid::PacketType::SinglePacket) {
                    //         hid_mon.send_empty_error_packet(send_packet);
                    //     } else {
                    //         let key = core::str::from_utf8(&report_buff[0..16])
                    //             .unwrap()
                    //             .trim_end_matches(char::from(0));

                    //         debug_rprintln!("write image '{}' len {}", key, report_len as u32);

                    //         let res = storage.write_image(&report_buff[16..report_len], &key);
                    //         if res.is_ok() {
                    //             hid_mon.send_empty_success_packet(send_packet);
                    //         } else {
                    //             hid_mon.send_empty_error_packet(send_packet);
                    //         }
                    //     }
                    // }
                    _ => {
                        hid_mon.send_empty_not_recognized_packet(send_packet);
                    }
                }
            }
        }
    }
}

pub fn reset_to_app() {
    disable_interrupts(|_| {
        NVIC::mask(interrupt::USB);
        NVIC::unpend(interrupt::USB);

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
