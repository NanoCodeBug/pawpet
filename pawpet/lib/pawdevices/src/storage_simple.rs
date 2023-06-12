use pawbsp as bsp;
use rtt_target::debug_rprintln;

// use core::{cell::RefCell, cmp, mem, slice};
use cortex_m::interrupt::free as disable_interrupts;
use crc::{Algorithm, Crc};
use pawdevicetraits::StorageDevice;
use spi_memory::series25::Flash;
use spi_memory::{BlockDevice, Read};

use pawdevicetraits::FileWriteError;
// MX25R1635FM1IL0

// #define MX25R1635F                                                                                                     \
//     {                                                                                                                  \
//         .total_size = (1 << 21), /* 2 MB / 16 Mb */                                                                    \
//             .start_up_time_us = 800, .manufacturer_id = 0xc2, .memory_type = 0x28, .capacity = 0x15,                   \
//         .max_clock_speed_mhz = 33 /*8*/, .quad_enable_bit_mask = 0x40, .has_sector_protection = false,                 \
//         .supports_fast_read = true, .supports_qspi = true, .supports_qspi_writes = true,                               \
//         .write_status_register_split = false, .single_status_byte = true,                                              \
//     }

/**
 * MX25R1635F
 * 2MB / 16Mb.
 * 4KB sector size
 */
use heapless::String;

#[repr(C)]
#[derive(Default)]
pub struct IndexBlockEntry {
    flags: u32,
    _name: [u8; 16],
    length: u32,
    addr: u32,
    crc: u32,
}

impl IndexBlockEntry {
    pub fn get_name_str(&self) -> &str {
        return unsafe { core::str::from_utf8_unchecked(&self._name) }
            .trim_end_matches(char::from(0));
    }

    pub fn set_name(&mut self, key: &[u8]) {
        debug_assert!(key.len() <= self._name.len());
        for i in 0..16 {
            self._name[i] = 0;
            if i < key.len() {
                self._name[i] = key[i];
            }
        }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut buffer: [u8; 32] = [0; 32];
        buffer[0..4].copy_from_slice(&self.flags.to_le_bytes());
        buffer[4..20].copy_from_slice(&self._name);
        buffer[20..24].copy_from_slice(&self.length.to_le_bytes());
        buffer[24..28].copy_from_slice(&self.addr.to_le_bytes());
        buffer[28..32].copy_from_slice(&self.crc.to_le_bytes());

        return buffer;
    }

    pub fn from_bytes(&mut self, data: &[u8]) {
        self.flags = u32::from_le_bytes(data[0..4].try_into().unwrap());
        self.set_name(&data[4..20]);
        self.length = u32::from_le_bytes(data[20..24].try_into().unwrap());
        self.addr = u32::from_le_bytes(data[24..28].try_into().unwrap());
        self.crc = u32::from_le_bytes(data[28..32].try_into().unwrap());
    }
}

pub struct CacheEntry {
    key: String<16>,
    offset: usize,
}

impl CacheEntry {
    pub fn new(key: &str, offset: usize) -> Self {
        Self {
            key: key.into(),
            offset,
        }
    }
}

static mut FLASH_ACCESS: Option<Flash<bsp::FlashSpi, bsp::FlashCs>> = None;
static mut IMAGE_CACHE_BUFFER: [u8; 8096] = [0; 8096];

const EMPTY_FLAG: u32 = 0xFFFFFFFF; // all 1's
const INVALID_FLAG: u32 = 0x0000AA99;
const VALID_FLAG: u32 = 0x0000AA9F;

pub enum IndexEntryError {
    EmptyEntry,
    InvalidatedEntry,
    CorruptEntry,
    NotFound,
}

struct SimpleFilesystem {
    total_blocks: u32,
    block_size: u32,
    write_size: u32,

    index_start: (u32, u32),
    max_index_blocks: u32,
    max_entries_per_block: u32,

    crc_hasher: Crc<crc::NoTable<u32>>,
}

impl SimpleFilesystem {
    pub fn new() -> Self {
        const CUSTOM_ALG: Algorithm<u32> = Algorithm {
            width: 32,
            poly: 0x04c11db7,
            init: 0xffffffff,
            refin: false,
            refout: false,
            xorout: 0xffffffff,
            check: 0xfc891918,
            residue: 0xc704dd7b,
        };

        let crc_hasher = Crc::<crc::NoTable<u32>>::new(&CUSTOM_ALG);

        Self {
            total_blocks: 512 - 32, // out of 512 blocks, reserve the last 32 blocks for save file ring buffer
            block_size: 4096,
            crc_hasher,
            max_index_blocks: 8, // 1024 entries before file system needs formatting
            max_entries_per_block: 4096 / core::mem::size_of::<IndexBlockEntry>() as u32,
            write_size: 256,
            index_start: (0, 0),
        }
    }

    // goes through all indexes and recordes highest valid index to skip over old entries
    // calling mount multiple times is valid
    // if distance between entries is large, defrag should be called to rebuild in the index
    pub fn mount(&mut self) {
        debug_assert!(self.write_size % core::mem::size_of::<IndexBlockEntry>() as u32 == 0);
        debug_assert!(self.block_size % self.write_size == 0);

        let mut count_empty = 0;
        let mut count_invalid = 0;
        let mut count_corrupt = 0;

        // use custom algorithm
        let mut found_files = 0;
        let mut index_entry: IndexBlockEntry = Default::default();
        let mut index_start: (u32, u32) = (0, 0);
        'search: for b in 0..self.max_index_blocks {
            for i in 0..self.max_entries_per_block {
                let valid = self.get_entry_in_index(b, i, &mut index_entry);
                if valid.is_ok() {
                    if index_start.0 + index_start.1 == 0 {
                        index_start = (b, i);
                    }
                    debug_rprintln!(
                        "\tentry @{} b: {} i: {} n: '{}'",
                        index_entry.addr,
                        b,
                        i,
                        index_entry.get_name_str()
                    );

                    found_files += 1;
                } else {
                    let err = valid.unwrap_err();
                    if matches!(err, IndexEntryError::EmptyEntry) {
                        count_empty = self.max_index_blocks * self.max_entries_per_block - ((b+1) * (i+1));
                        break 'search;
                    } else if matches!(err, IndexEntryError::InvalidatedEntry) {
                        count_invalid += 1;
                    } else if matches!(err, IndexEntryError::CorruptEntry) {
                        count_corrupt += 1;
                    }
                }
            }
        }
        self.index_start = index_start;
        debug_rprintln!(
            "\tmount valid {} invalid {} corrupt {} empty {} ",
            found_files,
            count_invalid,
            count_corrupt,
            count_empty
        );
    }

    // searches until matching entry or empty entry found
    pub fn find_entry(
        &self,
        name: &str,
        index_entry: &mut IndexBlockEntry,
    ) -> Result<(u32, u32), IndexEntryError> {
        for b in self.index_start.0..self.max_index_blocks {
            for i in self.index_start.1..self.max_entries_per_block {
                let res = self.get_entry_in_index(b, i, index_entry);
                if res.is_ok() {
                    if index_entry.get_name_str() == name {
                        return Ok((b, i));
                    }
                } else if res.is_err() && matches!(res.unwrap_err(), IndexEntryError::EmptyEntry) {
                    return Err(IndexEntryError::NotFound);
                }
            }
        }
        return Err(IndexEntryError::NotFound);
    }

    // TODO: consider hash map style for indexes? makes look up worst case of max_indexes per block instead of max indexes
    // TODO: store some cache structure to make lookups faster
    pub fn new_file_erase_old(&mut self, name: &str) -> Result<(u32, u32, u32), FileWriteError> {
        let mut found_empty: Option<(u32, u32)> = None;
        let mut largest_addr = 0;

        let mut index_entry: IndexBlockEntry = Default::default();

        'search: for b in self.index_start.0..self.max_index_blocks {
            for i in self.index_start.1..self.max_entries_per_block {
                let res = self.get_entry_in_index(b, i, &mut index_entry);
                if res.is_ok() {
                    // update largest found addr
                    let found_addr_end = index_entry.addr + index_entry.length;
                    if found_addr_end > largest_addr {
                        largest_addr = found_addr_end;
                    }

                    // found previous entry matching name, invalidate
                    if index_entry.get_name_str() == name {
                        self.invalidate_entry(b, i);
                    }
                } else {
                    let error = res.err().unwrap();
                    if matches!(error, IndexEntryError::InvalidatedEntry) {
                        // invalidated entries contain valid data locations, update largest addr
                        let found_addr_end = index_entry.addr + index_entry.length;

                        if found_addr_end > largest_addr {
                            largest_addr = found_addr_end;
                        }
                    } else if matches!(error, IndexEntryError::EmptyEntry) {
                        // empty means no more entries in index
                        if found_empty.is_none() {
                            found_empty = Some((b, i));
                            break 'search;
                        }
                    }
                }
            }
        }

        if largest_addr > 0 {
            //addr should aligned to the 256 byte boundry for bulk writes, write size should be multiple of block size
            let align_bytes = largest_addr % self.write_size;
            if align_bytes > 0 {
                largest_addr += self.write_size - align_bytes;
            }
        } else {
            largest_addr = self.max_index_blocks * self.block_size;
        }

        match found_empty {
            Some(index) => {
                return Ok((index.0, index.1, largest_addr));
            }
            None => {
                return Err(FileWriteError::FilesystemFull);
            }
        }
    }

    pub fn find_empty_entry(
        &self,
        index_entry: &mut IndexBlockEntry,
    ) -> Result<(u32, u32), IndexEntryError> {
        for b in self.index_start.0..self.max_index_blocks {
            for i in self.index_start.1..self.max_entries_per_block {
                let res = self.get_entry_in_index(b, i, index_entry);
                if res.is_err() && matches!(res.unwrap_err(), IndexEntryError::EmptyEntry) {
                    return Ok((b, i));
                }
            }
        }
        return Err(IndexEntryError::NotFound);
    }

    /**
     * Untested, consider always running N writes to ensure format doesn't need to be called  
     * */
    pub fn defrag_index(&mut self) {
        // TODO:
        // automatically erase index blocks that have no more valid entries
        // have defrag allocate a blocks worth of entries? 4kb to gurantee it can always free one block
        // automatically run N writes to keep index fragmentation minimal?
        // TODO: does not check if there is fragmentation before defragmenting

        let mut moved_entry: IndexBlockEntry = Default::default();
        let mut empty_entry: IndexBlockEntry = Default::default();

        let first_empty = self.find_empty_entry(&mut empty_entry).ok().unwrap();
        let mut empty = first_empty;

        'search: for b in self.index_start.0..self.max_index_blocks {
            for i in self.index_start.1..self.max_entries_per_block {
                let res = self.get_entry_in_index(b, i, &mut moved_entry);

                // reached start of defragged entries
                if b == first_empty.0 && i == first_empty.1 {
                    self.index_start = first_empty;
                    break 'search;
                }

                if res.is_ok() {
                    // write entry to new location
                    self.write_entry(empty.0, empty.1, &moved_entry);
                    // invalidate old entry
                    self.invalidate_entry(b, i);
                    // find next empty entry
                    empty = self.find_empty_entry(&mut empty_entry).ok().unwrap();
                }
            }
        }
    }

    pub fn invalidate_entry(&mut self, block: u32, index: u32) {
        SimpleFilesystem::write(
            block * self.block_size + index * core::mem::size_of::<IndexBlockEntry>() as u32,
            &INVALID_FLAG.to_le_bytes(),
            2,
        );
    }

    pub fn get_entry_in_index(
        &self,
        block: u32,
        index: u32,
        entry: &mut IndexBlockEntry,
    ) -> Result<(), IndexEntryError> {
        let mut block_buff: [u8; core::mem::size_of::<IndexBlockEntry>()] =
            [0; core::mem::size_of::<IndexBlockEntry>()];

        SimpleFilesystem::read(
            block * self.block_size + index * core::mem::size_of::<IndexBlockEntry>() as u32,
            &mut block_buff,
        );

        let flags = u32::from_le_bytes(block_buff[0..4].try_into().unwrap());

        // check flags, if all 1's then empty
        if flags == EMPTY_FLAG {
            // TODO check that all bytes are 1's before returning empty, could be corrupt
            return Err(IndexEntryError::EmptyEntry);
        }

        let name_str = core::str::from_utf8(&block_buff[2..16]);
        if name_str.is_err() {
            return Err(IndexEntryError::CorruptEntry);
        }

        entry.from_bytes(&block_buff);

        // check flags, if zero then the entry has been invalidated
        if flags == INVALID_FLAG {
            return Err(IndexEntryError::InvalidatedEntry);
        }

        if flags != VALID_FLAG {
            return Err(IndexEntryError::CorruptEntry);
        }

        return Ok(());
    }

    pub fn read_file(&self, index_entry: &IndexBlockEntry, data: &mut [u8]) -> bool {
        SimpleFilesystem::read(index_entry.addr, data);
        let data_crc = self.crc_hasher.checksum(data);
        if data_crc == index_entry.crc {
            debug_rprintln!("\tdata file read crc passed");
            return true;
        } else {
            // failed read, try again?
            debug_rprintln!("\tdata failed crc check");
            return false;
        }
    }

    pub fn write_file(&self, addr: u32, data: &[u8]) -> Result<u32, FileWriteError> {
        let start_addr_block = addr / self.block_size;
        let stop_addr_block = (addr + data.len() as u32) / self.block_size;

        if start_addr_block > self.total_blocks || stop_addr_block > self.total_blocks {
            debug_rprintln!("\tDISK FULL");
            return Err(FileWriteError::FilesystemFull);
        }

        debug_rprintln!("\twriting file @{} len {}", addr, data.len());

        if addr % self.block_size == 0 {
            SimpleFilesystem::erase(start_addr_block);
        }
        if start_addr_block != stop_addr_block {
            for x in (start_addr_block + 1)..=stop_addr_block {
                SimpleFilesystem::erase(x);
            }
        }

        let mut write_digest = self.crc_hasher.digest();

        // max 256 byte writes, must be aligned, cannot write outside current page

        let mut i: usize = 0;
        while i + (self.write_size as usize) < data.len() {
            let write_slice = &data[i..i + (self.write_size as usize)];

            write_digest.update(write_slice);
            SimpleFilesystem::write(addr + i as u32, write_slice, write_slice.len());

            i += self.write_size as usize;
        }
        let last_bit = data.len() - i;
        if last_bit != 0 {
            let write_slice = &data[i..data.len()];

            write_digest.update(write_slice);
            SimpleFilesystem::write(addr + i as u32, write_slice, write_slice.len());
        }
        let write_hash = write_digest.finalize();

        let mut buffer: [u8; 256] = [0; 256];
        let mut read_digest = self.crc_hasher.digest();

        let mut i = 0;
        while i + buffer.len() < data.len() {
            SimpleFilesystem::read(addr + i as u32, &mut buffer);

            read_digest.update(&buffer);

            i += buffer.len();
        }
        let last_bit = data.len() - i;
        if last_bit != 0 {
            SimpleFilesystem::read(addr + i as u32, &mut buffer[0..last_bit]);
            read_digest.update(&buffer[0..last_bit]);
        }
        let read_hash = read_digest.finalize();

        if write_hash == read_hash {
            return Ok(write_hash);
        }

        debug_rprintln!("\tread hash {}\n\twrite hash {}", read_hash, write_hash);
        debug_rprintln!("\tBAD CHECKSUM");
        return Err(FileWriteError::ChecksumFailed);
    }

    pub fn write_entry(&mut self, block: u32, index: u32, entry: &IndexBlockEntry) {
        const ENTRY_SIZE: usize = core::mem::size_of::<IndexBlockEntry>();
        let addr = (block * self.block_size) + (ENTRY_SIZE as u32) * index;

        debug_rprintln!(
            "\twriting entry @{} b: {} i: {} n: '{}'",
            addr,
            block,
            index,
            entry.get_name_str()
        );

        SimpleFilesystem::write(addr, &entry.to_bytes(), ENTRY_SIZE);

        // TODO, make flag section last written section
        // TODO, make get_entry only return empty if entire entry is 1's

        // TODO, read back entry for integrity
    }

    pub fn format(&mut self) {
        for b in 0..self.max_index_blocks {
            SimpleFilesystem::erase(b);
        }
    }

    fn read(addr: u32, buffer: &mut [u8]) {
        // debug_rprintln!("read {:x} {}", addr, buf.len());

        let _res =
            unsafe { disable_interrupts(|_| FLASH_ACCESS.as_mut().unwrap().read(addr, buffer)) };
    }

    fn write(addr: u32, buffer: &[u8], length: usize) {
        let _res = unsafe {
            disable_interrupts(|_| FLASH_ACCESS.as_mut().unwrap().write_bytes(addr, &buffer[0..length]))
        };
    }

    fn erase(block: u32) {
        let addr = block as usize * 4096 as usize;

        debug_rprintln!("\terase block {} ", block as usize);

        let _res = unsafe {
            disable_interrupts(|_| {
                FLASH_ACCESS
                    .as_mut()
                    .unwrap()
                    .erase_sectors((addr) as u32, 1)
            })
        };
    }
}

pub struct SimpleFlashStorage {
    cache_entries: [Option<CacheEntry>; 32],
    cache_offset: usize,
    fs: SimpleFilesystem,
}

impl SimpleFlashStorage {
    pub fn new(flash: Flash<bsp::FlashSpi, bsp::FlashCs>) -> Self {
        unsafe {
            FLASH_ACCESS = Some(flash);
        }
        let mut fs = SimpleFilesystem::new();
        fs.mount();

        Self {
            cache_entries: Default::default(),
            cache_offset: 0,
            fs,
        }
    }
}

impl StorageDevice for SimpleFlashStorage {
    fn load_image(&mut self, key: &str) -> Option<&'static [u8]> {
        let mut first_empty: &mut Option<CacheEntry> = &mut None;

        for p in self.cache_entries.iter_mut() {
            if p.is_some() {
                let e = p.as_ref().unwrap();
                if e.key == key {
                    unsafe {
                        let buff = &IMAGE_CACHE_BUFFER[e.offset..IMAGE_CACHE_BUFFER.len()];
                        return Some(buff);
                    }
                }
            } else {
                first_empty = p;
            }
        }
        debug_rprintln!("cache miss '{}'", key);

        // load image into cache and into first empty cache spot
        if first_empty.is_none() {
            let start_empty_buff: &mut [u8] =
                unsafe { &mut IMAGE_CACHE_BUFFER[self.cache_offset..IMAGE_CACHE_BUFFER.len()] };

            // do file read here
            let mut index_entry: IndexBlockEntry = Default::default();

            let res = self.fs.find_entry(key, &mut index_entry);

            if res.is_ok() {
                debug_rprintln!(
                    "\tselected entry '{}' len {} addr {}",
                    key,
                    index_entry.length,
                    index_entry.addr
                );

                // no more space in cache
                if index_entry.length > start_empty_buff.len() as u32 {
                    debug_rprintln!("CACHE FULL");
                    return None;
                }

                let file_buff = &mut start_empty_buff[0..index_entry.length as usize];

                if self.fs.read_file(&index_entry, file_buff) {
                    *first_empty = Some(CacheEntry::new(key, self.cache_offset));
                    self.cache_offset += index_entry.length as usize;

                    debug_rprintln!(
                        "\tbuffer address {:p}",
                        file_buff,
                    );

                    return Some(file_buff);
                }
            }
        }

        debug_rprintln!("file not found");
        // not found, return a pointer into the start of the image cache buffer
        // this should always be a valid image as either all 0s or the first successfully loaded image
        return None;
    }

    fn write_image(&mut self, data: &[u8], key: &str) -> Result<(), FileWriteError> {
        // first N blocks are reserved for filesystem entries

        let res = self.fs.new_file_erase_old(key);
        let (block, index, next_addr);
        match res {
            Ok((b, i, addr)) => {
                block = b;
                index = i;
                next_addr = addr;
            }
            Err(x) => {
                return Err(x);
            }
        }

        debug_rprintln!("\tusing next index {} {}", block, index);
        debug_rprintln!("\tusing next addr {}", next_addr);

        let mut new_entry: IndexBlockEntry = Default::default();

        new_entry.flags = VALID_FLAG;
        new_entry.set_name(key.as_bytes());
        new_entry.addr = next_addr;
        new_entry.length = data.len() as u32;

        let res = self.fs.write_file(new_entry.addr, &data);
        match res {
            Ok(x) => {
                new_entry.crc = x;
            }
            Err(FileWriteError::ChecksumFailed) => {
                debug_rprintln!("\tcrc check trying next block");
                new_entry.addr = (next_addr / self.fs.block_size) + self.fs.block_size;
                let res = self.fs.write_file(new_entry.addr, &data);
                if res.is_err() {
                    debug_rprintln!("\tWRITE ATTEMPT 2 FAILED");
                    return Err(res.unwrap_err());
                }
            }
            Err(FileWriteError::FilesystemFull) => {
                return Err(res.unwrap_err());
            }
        }

        self.fs.write_entry(block, index, &new_entry);

        return Ok(());
    }

    fn format_storage(&mut self) {
        self.fs.format();
        self.clear_cache();
        self.fs.mount();
    }

    fn clear_cache(&mut self) {
        for p in self.cache_entries.iter_mut() {
            *p = None;
        }
    }
}
