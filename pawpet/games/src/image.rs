use core::mem::size_of;
use pawdevicetraits::DisplayDevice;

#[repr(C)]
#[repr(packed)]
pub struct PawImageHeader {
    width: u16,
    height: u16,
    encoding: u16, // 2 - span array binary, 3 span array alhpa, 0 bitmap binary, 1 bitmap alpha
    tile_count: u16,
    // array of tile offsets u16
}

pub struct PawImage {
    on_color: bool,
    off_color: bool,
    alpha_color: Option<bool>,
    data: Option<&'static [u8]>,
    frame: u8,
}

impl PawImage {
    pub fn new(data: Option<&'static [u8]>) -> Self {
        Self {
            off_color: false,
            on_color: true,
            alpha_color: None,
            data,
            frame: 0,
        }
    }

    pub fn new_no_data() -> Self {
        Self {
            off_color: false,
            on_color: true,
            alpha_color: None,
            data : None,
            frame: 0,
        }
    }

    pub fn set_image(&mut self, data: Option<&'static [u8]>)
    {
        self.data = data;
    }

    pub fn set_colors(&mut self, on_color: bool, off_color: bool, alpha_color: Option<bool>) {
        self.on_color = on_color;
        self.off_color = off_color;
        self.alpha_color = alpha_color;
    }

    pub fn set_frame(&mut self, frame: u8) {
        self.frame = frame;
    }

    pub fn draw(&self, disp: &mut impl DisplayDevice, dx: u8, dy: u8) {
        if self.data.is_none() {
            return;
        }
        let data = self.data.unwrap();

        let meta_ptr: *const PawImageHeader;
        unsafe {
            meta_ptr = core::mem::transmute(data[0..size_of::<PawImageHeader>()].as_ptr());
        }
        let meta: &PawImageHeader = unsafe { &*meta_ptr };

        // let frame_offsets: *const u8 = unsafe {
        //     core::mem::transmute(self.data[size_of::<PawImageHeader>()..self.data.len()].as_ptr())
        // };
        let frame_offsets_slice = &data[size_of::<PawImageHeader>()..data.len()];

        let tileoffset = meta.tile_count * 2;

        let data_ptr_start: *const u8 = unsafe {
            core::mem::transmute(
                data[(size_of::<PawImageHeader>() + (tileoffset as usize))..data.len()].as_ptr(),
            )
        };
        let frame = (self.frame as usize) * 2;
        let frame_offset = [frame_offsets_slice[frame], frame_offsets_slice[frame + 1]];
        let tile_loc_offset_bytes = u16::from_le_bytes(frame_offset);

        let image_ptr = unsafe { data_ptr_start.offset(tile_loc_offset_bytes as isize) };

        match meta.encoding {
            0 => {
                //bitmap no alpha
                self.draw_bitmap(disp, image_ptr, meta, dx, dy);
            }
            1 => {
                //bitmap with alpha
                self.draw_bitmap_alpha(disp, image_ptr, meta, dx, dy);
            }
            2 => {
                //span
                self.draw_span(disp, image_ptr, meta, dx, dy);
            }
            3 => {
                // span with alpha
                self.draw_span_alpha(disp, image_ptr, meta, dx, dy);
            }
            _ => {}
        }
    }

    pub fn draw_span(
        &self,
        disp: &mut impl DisplayDevice,
        data: *const u8,
        meta: &PawImageHeader,
        dx: u8,
        dy: u8,
    ) {
        let bitmap_length = meta.width * meta.height;
        let mut pixels_read = 0;
        let mut curr_byte = 0;

        while pixels_read < bitmap_length {
            let pixel_data = unsafe { *data.offset(curr_byte) };

            let length = pixel_data & 0x7F;
            let color = (pixel_data >> 7) & 0x1;

            for _p in 0..length {
                let x = (pixels_read % meta.width) as u8;
                let y = (pixels_read / meta.width) as u8;

                match color {
                    0 => {
                        disp.draw_pixel((dx + x) as u8, (dy + y) as u8, self.off_color);
                    }
                    1 => {
                        disp.draw_pixel((dx + x) as u8, (dy + y) as u8, self.on_color);
                    }
                    _ => {}
                }
                pixels_read += 1;
            }
            curr_byte += 1;
        }
    }
    pub fn draw_span_alpha(
        &self,
        disp: &mut impl DisplayDevice,
        data: *const u8,
        meta: &PawImageHeader,
        dx: u8,
        dy: u8,
    ) {
        let bitmap_length = meta.width * meta.height;
        let mut pixels_read = 0;
        let mut curr_byte = 0;

        while pixels_read < bitmap_length {
            let pixel_data = unsafe { *data.offset(curr_byte) };

            let length = pixel_data & 0x3F;
            let color = (pixel_data >> 6) & 0x3;

            for _p in 0..length {
                let x = (pixels_read % meta.width) as u8;
                let y = (pixels_read / meta.width) as u8;

                match color {
                    0 => {
                        disp.draw_pixel((dx + x) as u8, (dy + y) as u8, self.off_color);
                    }
                    1 => {
                        disp.draw_pixel((dx + x) as u8, (dy + y) as u8, self.on_color);
                    }
                    _ => {}
                }
                pixels_read += 1;
            }
            curr_byte += 1;
        }
    }

    pub fn draw_bitmap(
        &self,
        disp: &mut impl DisplayDevice,
        data: *const u8,
        meta: &PawImageHeader,
        dx: u8,
        dy: u8,
    ) {
        let mut pack: u8 = 0;
        let mut bit_index: u8 = 0;
        let mut packed_u8_index: isize = 0;
        for y in 0..meta.height as u8 {
            for x in 0..meta.width as u8 {
                // shift saved byte to next bit
                if bit_index > 0 {
                    pack >>= 1;
                    bit_index -= 1;
                }
                // read next byte
                else {
                    bit_index = 7;
                    pack = unsafe { *data.offset(packed_u8_index) };
                    packed_u8_index += 1;
                }

                if pack & 0x1 > 0 {
                    disp.draw_pixel((dx + x) as u8, (dy + y) as u8, self.on_color);
                } else {
                    disp.draw_pixel((dx + x) as u8, (dy + y) as u8, self.off_color);
                }
            }
        }
    }
    pub fn draw_bitmap_alpha(
        &self,
        disp: &mut impl DisplayDevice,
        data: *const u8,
        meta: &PawImageHeader,
        dx: u8,
        dy: u8,
    ) {
        let mut pack: u8 = 0;
        let mut bit_index: u8 = 0;
        let mut packed_u8_index: isize = 0;
        for y in 0..meta.height as u8 {
            for x in 0..meta.width as u8 {
                // shift saved byte to next bit
                if bit_index > 0 {
                    pack >>= 2;
                    bit_index -= 1;
                }
                // read next byte
                else {
                    bit_index = 3;
                    pack = unsafe { *data.offset(packed_u8_index) };
                    packed_u8_index += 1;
                }

                let pixel = pack & 0x3;
                match pixel {
                    0 => {
                        disp.draw_pixel((dx + x) as u8, (dy + y) as u8, self.off_color);
                    }
                    1 => {
                        disp.draw_pixel((dx + x) as u8, (dy + y) as u8, self.on_color);
                    }
                    2 => {
                        if self.alpha_color.is_some() {
                            disp.draw_pixel(
                                (dx + x) as u8,
                                (dy + y) as u8,
                                self.alpha_color.unwrap(),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

}

pub struct PawAnimation {
    image: PawImage,
    ticks_per_frame: u16,
    tick: u16,
    loop_bounds: (u8, u8), // dir: pinpong/forward
}

impl PawAnimation {
    pub fn new(bounds: (u8, u8), ticks_per_frame: u16) -> Self {
        Self {
            image: PawImage::new(None),
            ticks_per_frame: ticks_per_frame,
            tick: 0,
            loop_bounds: bounds,
        }
    }

    pub fn set_image(&mut self, data: Option<&'static [u8]>)
    {
        self.image.data = data;
    }

    pub fn tick(&mut self) {
        self.tick = self.tick + 1;

        if self.tick > self.ticks_per_frame {
            self.tick = 0;
            if self.image.frame + 1 >= self.loop_bounds.1 {
                self.image.frame = 0;
            } else {
                self.image.frame = self.image.frame + 1;
            }
        }
    }

    pub fn set_colors(&mut self, on_color: bool, off_color: bool, alpha_color: Option<bool>) {
        self.image.set_colors(on_color, off_color, alpha_color);
    }

    pub fn set_frame(&mut self, frame: u8) {
        self.image.frame = frame;
    }

    pub fn draw(&self, disp: &mut impl DisplayDevice, dx: u8, dy: u8) {
        self.image.draw(disp, dx, dy);
    }
    
}
