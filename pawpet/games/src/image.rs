use core::{mem::size_of, ptr::null};
use pawdevicetraits::DisplayDevice;
use rtt_target::debug_rprintln;

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
    image_ptr_offset: usize,
    encoding: u8,
    width: u8,
    height: u8,
}

impl PawImage {
    pub fn new(data: Option<&'static [u8]>) -> Self {
        let mut s = Self {
            off_color: false,
            on_color: true,
            alpha_color: None,
            data,
            frame: 0,
            image_ptr_offset: 0,
            encoding: 0,
            width: 0,
            height: 0,
        };
        s.set_image(data);
        return s;
    }

    pub fn new_no_data() -> Self {
        Self {
            off_color: false,
            on_color: true,
            alpha_color: None,
            data: None,
            frame: 0,
            image_ptr_offset: 0,
            encoding: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn set_image(&mut self, data: Option<&'static [u8]>) {
        self.data = data;

        self.update_image_data_offset();
    }

    pub fn set_colors(&mut self, on_color: bool, off_color: bool, alpha_color: Option<bool>) {
        self.on_color = on_color;
        self.off_color = off_color;
        self.alpha_color = alpha_color;
    }

    pub fn update_image_data_offset(&mut self) {
        if self.data.is_none() {
            return;
        }

        let data = self.data.unwrap();

        let meta_ptr: *const PawImageHeader =
            unsafe { core::mem::transmute(data[0..size_of::<PawImageHeader>()].as_ptr()) };

        let meta: &PawImageHeader = unsafe { &*meta_ptr };

        self.image_ptr_offset = size_of::<PawImageHeader>() + 2;

        // sprite map/animation
        if meta.tile_count > 1 {
            let frame_offsets_list = &data[size_of::<PawImageHeader>()..data.len()];
            let frame_offsets_length = meta.tile_count * 2;

            let frame = (self.frame as usize) * 2;
            let frame_offset_bytes = [frame_offsets_list[frame], frame_offsets_list[frame + 1]];
            let frame_offset = u16::from_le_bytes(frame_offset_bytes) as usize;

            self.image_ptr_offset =
                size_of::<PawImageHeader>() + (frame_offsets_length as usize) + frame_offset;
        }

        self.width = meta.width as u8;
        self.height = meta.height as u8;
        self.encoding = meta.encoding as u8;
    }

    pub fn set_frame(&mut self, frame: u8) {
        self.frame = frame;

        self.update_image_data_offset();
    }

    pub fn draw(&self, disp: &mut impl DisplayDevice, x: u8, y: u8) {
        if self.data.is_none() {
            return;
        }

        let data = self.data.unwrap();

        let image_ptr =
            unsafe { core::mem::transmute(data[self.image_ptr_offset..data.len()].as_ptr()) };

        match self.encoding {
            0 => {
                //bitmap no alpha
                self.draw_bitmap(disp, image_ptr, x, y);
            }
            1 => {
                //bitmap with alpha
                self.draw_bitmap_alpha(disp, image_ptr, x, y);
            }
            2 => {
                //span
                self.draw_span(disp, image_ptr, x, y);
            }
            3 => {
                // span with alpha
                self.draw_span_alpha(disp, image_ptr, x, y);
            }
            _ => {}
        }
    }

    pub fn draw_span(&self, disp: &mut impl DisplayDevice, data: *const u8, dx: u8, dy: u8) {
        let dyt = dy + self.height as u8;
        let dxt = dx + self.width as u8;

        let mut curr_byte = 0;
        let mut pixel_data = unsafe { *data.offset(curr_byte) };
        let mut length = pixel_data & 0x7F;

        for y in dy..dyt {
            for x in dx..dxt {
                if length == 0 {
                    curr_byte += 1;
                    pixel_data = unsafe { *data.offset(curr_byte) };
                    length = pixel_data & 0x7F;
                }

                if (pixel_data >> 7) & 0x1 > 0 {
                    disp.draw_pixel(x, y, self.on_color);
                } else {
                    disp.draw_pixel(x, y, self.off_color);
                }

                length -= 1;
            }
        }
    }
    pub fn draw_span_alpha(
        &self,
        disp: &mut impl DisplayDevice,
        data: *const u8,
        dx: u8,
        dy: u8,
    ) {
        let mut curr_byte = 0;
        let mut pixel_data = unsafe { *data.offset(curr_byte) };

        let mut length = pixel_data & 0x3F;
        let mut color = (pixel_data >> 6) & 0x3;

        let dyt = dy + self.height as u8;
        let dxt = dx + self.width as u8;

        for y in dy..dyt {
            for x in dx..dxt {
                if length == 0 {
                    curr_byte += 1;
                    pixel_data = unsafe { *data.offset(curr_byte) };
                    length = pixel_data & 0x3F;
                    color = (pixel_data >> 6) & 0x3;
                }

                match color {
                    0 => {
                        disp.draw_pixel(x, y, self.off_color);
                    }
                    1 => {
                        disp.draw_pixel(x, y, self.on_color);
                    }
                    2 => {
                        if self.alpha_color.is_some() {
                            disp.draw_pixel(x, y, self.alpha_color.unwrap());
                        }
                    }
                    _ => {}
                }
                length -= 1;
            }
        }
    }

    pub fn draw_bitmap(&self, disp: &mut impl DisplayDevice, data: *const u8, dx: u8, dy: u8) {
        let mut pack: u8 = 0;
        let mut bit_index: u8 = 0;
        let mut packed_u8_index: isize = 0;

        let dyt = dy + self.height as u8;
        let dxt = dx + self.width as u8;

        for y in dy..dyt {
            for x in dx..dxt {
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
                    disp.draw_pixel(x, y, self.on_color);
                } else {
                    disp.draw_pixel(x, y, self.off_color);
                }
            }
        }
    }
    pub fn draw_bitmap_alpha(
        &self,
        disp: &mut impl DisplayDevice,
        data: *const u8,
        dx: u8,
        dy: u8,
    ) {
        let mut pack: u8 = 0;
        let mut bit_index: u8 = 0;
        let mut packed_u8_index: isize = 0;
        let dyt = dy + self.height as u8;
        let dxt = dx + self.width as u8;

        for y in dy..dyt {
            for x in dx..dxt {
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
                        disp.draw_pixel(x, y, self.off_color);
                    }
                    1 => {
                        disp.draw_pixel(x, y, self.on_color);
                    }
                    2 => {
                        if self.alpha_color.is_some() {
                            disp.draw_pixel(x, y, self.alpha_color.unwrap());
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

    pub fn set_image(&mut self, data: Option<&'static [u8]>) {
        self.image.set_image(data);
    }

    pub fn tick(&mut self) {
        self.tick = self.tick + 1;

        if self.tick > self.ticks_per_frame {
            self.tick = 0;
            if self.image.frame + 1 >= self.loop_bounds.1 {
                self.image.set_frame(0);
            } else {
                self.image.set_frame(self.image.frame + 1);
            }
        }
    }

    pub fn set_colors(&mut self, on_color: bool, off_color: bool, alpha_color: Option<bool>) {
        self.image.set_colors(on_color, off_color, alpha_color);
    }

    pub fn set_frame(&mut self, frame: u8) {
        self.image.set_frame(frame);
    }

    pub fn draw(&self, disp: &mut impl DisplayDevice, dx: u8, dy: u8) {
        self.image.draw(disp, dx, dy);
    }
}
