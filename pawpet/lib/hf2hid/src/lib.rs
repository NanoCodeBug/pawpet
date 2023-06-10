#![no_std]

use core::mem::size_of;
use core::mem::ManuallyDrop;

static VERSION_INFO: &'static [u8] = b"PawPet, HW: 3.0, FW: 1.0a";

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum HF2Commands {
    BinInfo = 0x01,
    Info = 0x02,
    // ResetIntoApp=0x03,
    ResetIntoBootloader = 0x04,
    // StartFlash=0x05,
    // WriteFlashPage=0x06,
    // CheckSumPages=0x07,
    // ReadWords=0x08,
    // WriteWords=0x09,
    DMesg = 0x10,

    // custom messages, random 4 byte values
    GetFileSysInfo = 0x83fd,
    FormatFileSys = 0xba8e,
    WriteFile = 0xff68,
    ReadFile = 0x8bf7,
}

pub enum HF2Responses {
    Success = 0x00,
    NotRecognized = 0x01,
    ExecutionError = 0x02,
}

/////////////////////////////////////////
#[repr(C, packed(1))]
struct HF2Packet {
    header: u8,
    content: HF2PacketContents,
}

#[repr(C, packed(1))]
union HF2PacketContents {
    command: ManuallyDrop<HF2Command>,
    response: ManuallyDrop<HF2Response>,
    raw_data: [u8; 63],
}

#[repr(C, packed(1))]
pub struct HF2Command {
    command_id: u32,
    tag: u16,
    _reserved0: u8,
    _reserved1: u8,
    data: [u8; 55],
}

#[repr(C, packed(1))]
union HF2ResponsesUnion {
    bin: ManuallyDrop<HF2BinInfoResult>,
    data: [u8; 59],
}

#[repr(C, packed(1))]
pub struct HF2BinInfoResult {
    mode: u32,             // 0x02  app mode
    flash_page_size: u32,  // 256 bytes
    flash_num_pages: u32,  // 1024 pages
    max_message_size: u32, // 320 // max_message_size >= flash_page_size + 64.
    family_id: u32,        // 0x68ed_2b88 ATSAMD21
}

#[repr(C, packed(1))]
pub struct HF2Response {
    tag: u16,
    status: u8,
    status_info: u8,
    content: HF2ResponsesUnion,
}

/////////////////////////////////////////////
pub enum PacketType {
    SinglePacket,
    MultiPacket,
}

pub struct HF2Monitor {
    data: HF2Packet,
    command: Option<HF2Commands>,
    data_packet_index: usize,
}

impl HF2Monitor {
    pub fn new() -> Self {
        Self {
            data: HF2Packet {
                header: 0,

                content: HF2PacketContents {
                    command: ManuallyDrop::new(HF2Command {
                        command_id: 0,
                        tag: 0,
                        _reserved0: 0,
                        _reserved1: 0,
                        data: [0; 55],
                    }),
                },
            },
            command: None,
            data_packet_index: 0,
        }
    }

    pub fn try_recv_packet<F>(&mut self, mut recv: F) -> (Option<HF2Commands>, PacketType)
    where
        F: FnMut(&mut [u8; 64]) -> usize,
    {
        self.data_packet_index = 0;
        
        let mut report: [u8; 64] = [0; 64];
        let _length = recv(&mut report);

        let packet_length = (report[0] & 0x3F) as usize;
        let packet_type = report[0] & 0xC0;

        assert!(packet_length <= 63);

        let final_packet = packet_type & 0x40 > 0;

        let command_id = u32::from_le_bytes(report[1..5].try_into().unwrap());
        let tag = u16::from_le_bytes(report[5..7].try_into().unwrap());
        // two reserved bytes

        let command: &mut HF2Command;

        unsafe {
            command = &mut (*self.data.content.command);
        }

        command.command_id = command_id;
        command.tag = tag;

        // data segment present in packet
        if packet_length > 8 {
            // header byte (1) + command_packet size (8) =
            for i in 0..packet_length - 8 {
                command.data[i] = report[i + 1 + 8];
            }
            // command.data[0..(packet_length - 8)].copy_from_slice(&report[9..packet_length-1]);
        }

        match command_id {
            0x01 => {
                self.command = Some(HF2Commands::BinInfo);
            }
            0x02 => {
                self.command = Some(HF2Commands::Info);
            }
            0x04 => {
                self.command = Some(HF2Commands::ResetIntoBootloader);
            }
            0x10 => {
                self.command = Some(HF2Commands::DMesg);
            }
            0x83fd => {
                self.command = Some(HF2Commands::GetFileSysInfo);
            }
            0xff68 => {
                self.command = Some(HF2Commands::WriteFile);
            }
            0xba8e => {
                self.command = Some(HF2Commands::FormatFileSys);
            }
            0x8bf7 => {
                self.command = Some(HF2Commands::ReadFile);
            }
            _ => self.command = None,
        }

        if !final_packet {
            return (self.command, PacketType::MultiPacket);
        }

        return (self.command, PacketType::SinglePacket);
    }

    pub fn send_empty_not_recognized_packet<F>(&mut self, mut send: F)
    where
        F: FnMut(&[u8]),
    {
        let mut response: &mut HF2Response;

        unsafe {
            response = &mut (*self.data.content.response);
        }

        response.status = HF2Responses::NotRecognized as u8;

        self.data.header |= 0x40; // final packet

        let report: &[u8] = unsafe {
            core::slice::from_raw_parts(
                (&self.data as *const HF2Packet) as *const u8,
                core::mem::size_of::<HF2Packet>(),
            )
        };

        send(&report);
    }

    pub fn send_version_info_packet<F>(&mut self, mut send: F)
    where
        F: FnMut(&[u8]),
    {
        let mut response: &mut HF2Response;

        unsafe {
            response = &mut (*self.data.content.response);
        }

        self.data.header = 4;
        response.status = HF2Responses::Success as u8;

        unsafe {
            let info = &mut response.content.data;
            info[0..VERSION_INFO.len()].copy_from_slice(VERSION_INFO);
        }

        self.data.header += VERSION_INFO.len() as u8;
        self.data.header |= 0x40; // final packet

        let report: &[u8] = unsafe {
            core::slice::from_raw_parts(
                (&self.data as *const HF2Packet) as *const u8,
                core::mem::size_of::<HF2Packet>(),
            )
        };

        send(&report);
    }

    pub fn send_bin_info_packet<F>(&mut self, mut send: F)
    where
        F: FnMut(&[u8]),
    {
        let mut response: &mut HF2Response;

        unsafe {
            response = &mut (*self.data.content.response);
        }

        self.data.header = 4;

        response.status = HF2Responses::Success as u8;
        unsafe {
            let mut bininfo: &mut HF2BinInfoResult = &mut (*response.content.bin);

            bininfo.mode = 0x02;
            bininfo.flash_page_size = 256;
            bininfo.flash_num_pages = 1024;
            bininfo.max_message_size = 4096; //320;
            bininfo.family_id = 0x68ed_2b88; //ATSAMD21
        }

        self.data.header += size_of::<HF2BinInfoResult>() as u8;
        self.data.header |= 0x40; // final packet

        let report: &[u8] = unsafe {
            core::slice::from_raw_parts(
                (&self.data as *const HF2Packet) as *const u8,
                core::mem::size_of::<HF2Packet>(),
            )
        };

        send(&report);
    }

    pub fn send_empty_error_packet<F>(&mut self, mut send: F)
    where
        F: FnMut(&[u8]),
    {
        let mut response: &mut HF2Response;

        unsafe {
            response = &mut (*self.data.content.response);
        }

        response.status = HF2Responses::ExecutionError as u8;

        self.data.header = 4;
        self.data.header |= 0x40; // final packet

        let report: &[u8] = unsafe {
            core::slice::from_raw_parts(
                (&self.data as *const HF2Packet) as *const u8,
                core::mem::size_of::<HF2Packet>(),
            )
        };

        send(&report);
    }

    pub fn send_empty_success_packet<F>(&mut self, mut send: F)
    where
        F: FnMut(&[u8]),
    {
        let mut response: &mut HF2Response;

        unsafe {
            response = &mut (*self.data.content.response);
        }

        response.status = HF2Responses::Success as u8;

        self.data.header = 4;
        self.data.header |= 0x40; // final packet

        let report: &[u8] = unsafe {
            core::slice::from_raw_parts(
                (&self.data as *const HF2Packet) as *const u8,
                core::mem::size_of::<HF2Packet>(),
            )
        };

        send(&report);
    }

    pub fn recv_multi_data_packet<F>(
        &mut self,
        mut recv: F,
        data_buffer: &mut [u8; 4096],
        report_len: &mut usize,
    ) -> bool
    where
        F: FnMut(&mut [u8; 64]) -> usize,
    {
        // copy first packet data into buffer
        if self.data_packet_index == 0 {
            let command: &mut HF2Command;

            unsafe {
                command = &mut (*self.data.content.command);
            }

            for i in 0..command.data.len() {
                data_buffer[i] = command.data[i];
            }
            self.data_packet_index = command.data.len();
        }

        let mut report: [u8; 64] = [0; 64];
        let _length = recv(&mut report);

        let packet_length = (report[0] & 0x3F) as usize;
        let packet_type = report[0] & 0xC0;
        let final_packet = (packet_type & 0x40) > 0;

        assert!(packet_length <= 63);

        for i in 0..packet_length {
            // report data starts after 1st header byte
            data_buffer[self.data_packet_index + i] = report[1 + i];
        }
        self.data_packet_index += packet_length;
        *report_len = self.data_packet_index;

        if final_packet {
            return true;
        }

        return false;
    }

    // pub fn send_multi_data_packet(&mut self, report: &mut [u8; 64], data_buffer: &mut [u8; 4096]) {}
}
