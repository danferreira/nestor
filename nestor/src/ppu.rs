use std::sync::{Arc, Mutex};

mod addr;
mod control;
pub mod frame;
mod mask;
pub mod palette;
mod scroll;
mod sprite;
mod status;

use crate::ppu::frame::Frame;
use crate::rom::{Mirroring, Rom};
use addr::AddrRegister;
use control::ControlRegister;
use mask::MaskRegister;
use scroll::ScrollRegister;
use sprite::Sprite;
use status::StatusRegister;

const NAMETABLE_SIZE: usize = 0x400;
const PALETTE_SIZE: usize = 0x20;
const OAM_SIZE: usize = 0x100;

const PPUCTRL: u16 = 0x2000;
const PPUMASK: u16 = 0x2001;
const PPUSTATUS: u16 = 0x2002;
const OAMADDR: u16 = 0x2003;
const OAMDATA: u16 = 0x2004;
const PPUSCROLL: u16 = 0x2005;
const PPUADDR: u16 = 0x2006;
const PPUDATA: u16 = 0x2007;

#[derive(Debug, Copy, Clone, PartialEq)]
enum Scanline {
    PreRender,
    Visible,
    PostRender,
    VBlank,
}

impl Scanline {
    pub fn from(scanline: usize) -> Self {
        match scanline {
            261 => Scanline::PreRender,
            0..=239 => Scanline::Visible,
            240 => Scanline::PostRender,
            241..=260 => Scanline::VBlank,

            _ => panic!("Invalid scanline!"),
        }
    }
}

pub struct PPU {
    pub rom: Option<Arc<Mutex<Rom>>>,
    pub vram: [u8; 2 * NAMETABLE_SIZE],
    pub palette_table: [u8; PALETTE_SIZE],

    pub oam_data: [u8; OAM_SIZE],
    secondary_oam_data: Vec<Option<Sprite>>,
    pub oam_addr: u8,

    pub scanline: usize,
    pub cycle: usize,
    frame_count: usize,

    pub mask: MaskRegister,
    pub addr: AddrRegister,
    pub ctrl: ControlRegister,
    pub scroll: ScrollRegister,
    pub status: StatusRegister,

    //internal registers
    v: u16,
    t: u16,
    fine_x: u8,
    w: bool,

    vram_buffer: u8,

    nametable_byte: u8,
    attribute_byte: u8,
    bg_tile_lo: u8,
    bg_tile_hi: u8,

    // Shifters
    bg_shifter_pattern_lo: u16,
    bg_shifter_pattern_hi: u16,
    sprite_shifter_pattern_lo: [u8; 8],
    sprite_shifter_pattern_hi: [u8; 8],
    bg_shifter_attrib_lo: u16,
    bg_shifter_attrib_hi: u16,

    pub nmi_interrupt: Option<u8>,

    suppress_vbl: bool,

    // The last written value to any PPU register
    // For use when reading the PPUSTATUS
    pub data_bus: u8,

    // Odd/even frame state
    odd_frame: bool,

    pub frame: Frame,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            rom: None,
            vram: [0; 2 * NAMETABLE_SIZE],
            oam_data: [0xFF; OAM_SIZE],
            oam_addr: 0,
            secondary_oam_data: vec![None; 8],
            sprite_shifter_pattern_lo: [0; 8],
            sprite_shifter_pattern_hi: [0; 8],
            palette_table: [0; PALETTE_SIZE],

            mask: MaskRegister::new(),
            ctrl: ControlRegister::new(),
            addr: AddrRegister::new(),
            scroll: ScrollRegister::new(),
            status: StatusRegister::new(),

            v: 0,
            t: 0,
            fine_x: 0,
            w: false,

            vram_buffer: 0,

            nametable_byte: 0,
            attribute_byte: 0,
            bg_tile_lo: 0,
            bg_tile_hi: 0,

            bg_shifter_pattern_lo: 0,
            bg_shifter_pattern_hi: 0,

            bg_shifter_attrib_lo: 0,
            bg_shifter_attrib_hi: 0,

            scanline: 0,
            cycle: 0,
            frame_count: 0,

            nmi_interrupt: None,
            suppress_vbl: false,

            data_bus: 0,

            odd_frame: false,

            frame: Frame::new(256, 240),
        }
    }

    pub fn load_rom(&mut self, rom: Arc<Mutex<Rom>>) {
        self.rom = Some(rom);
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());

        self.v = self.v.wrapping_add(self.ctrl.vram_addr_increment() as u16);
    }

    // The coarse X component of v needs to be incremented when the next tile is reached.
    // Bits 0-4 are incremented, with overflow toggling bit 10.
    // This means that bits 0-4 count from 0 to 31 across a single nametable, and bit 10 selects the current nametable horizontally.
    // https://www.nesdev.org/wiki/PPU_scrolling#Coarse_X_increment
    fn increment_x(&mut self) {
        if self.mask.show_background() {
            // if coarse X == 31
            if self.v & 0x001f == 31 {
                // coarse X = 0
                self.v &= !0x001f;

                // switch horizontal nametable
                self.v ^= 0x0400;
            } else {
                // increment coarse X
                self.v += 1;
            }
        }
    }

    // If rendering is enabled, fine Y is incremented at dot 256 of each scanline, overflowing to coarse Y,
    // and finally adjusted to wrap among the nametables vertically.
    // Bits 12-14 are fine Y. Bits 5-9 are coarse Y. Bit 11 selects the vertical nametable.
    // https://www.nesdev.org/wiki/PPU_scrolling#Y_increment
    fn increment_y(&mut self) {
        if self.mask.show_background() {
            // if fine Y < 7
            if (self.v & 0x7000) != 0x7000 {
                self.v += 0x1000; // increment fine Y
            } else {
                self.v &= !0x7000; // fine Y = 0
                let mut y = (self.v & 0x03E0) >> 5; // let y = coarse Y

                if y == 29 {
                    y = 0; // coarse Y = 0
                    self.v ^= 0x0800; // switch vertical nametable
                } else if y == 31 {
                    y = 0; // coarse Y = 0, nametable not switched
                } else {
                    y += 1; // increment coarse Y
                }

                self.v = (self.v & !0x03E0) | (y << 5) // put coarse Y back into v
            }
        }
    }

    // https://www.nesdev.org/wiki/PPU_scrolling#At_dot_257_of_each_scanline
    // If rendering is enabled, the PPU copies all bits related to horizontal position from t to v:
    // v: ....A.. ...BCDEF <- t: ....A.. ...BCDEF
    fn transfer_x(&mut self) {
        if self.mask.show_background() {
            self.v = self.v & 0x7BE0 | self.t & 0x041F;
        }
    }

    // https://www.nesdev.org/wiki/PPU_scrolling#During_dots_280_to_304_of_the_pre-render_scanline_(end_of_vblank)
    // If rendering is enabled, at the end of vblank, shortly after the horizontal bits are copied from t to v at dot 257,
    // the PPU will repeatedly copy the vertical bits from t to v from dots 280 to 304,
    // completing the full initialization of v from t:
    // v: GHIA.BC DEF..... <- t: GHIA.BC DEF.....
    fn transfer_y(&mut self) {
        if self.mask.show_background() {
            self.v = self.v & 0x041F | self.t & 0x7BE0;
        }
    }

    // https://wiki.nesdev.com/w/index.php/PPU_scrolling#Tile_and_attribute_fetching
    // the index of the tile from the pattern table
    fn fetch_nametable_byte(&mut self) -> u8 {
        let addr = 0x2000 | (self.v & 0x0FFF);
        self.mem_read(addr)
    }

    // https://wiki.nesdev.com/w/index.php/PPU_scrolling#Tile_and_attribute_fetching
    // The high bits of v are used for fine Y during rendering,
    // and addressing nametable data only requires 12 bits,
    // with the high 2 CHR address lines fixed to the 0x2000 region.
    fn fetch_attribute_table_byte(&self) -> u8 {
        let addr = 0x23c0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07);
        let attr_byte = self.mem_read(addr);

        let shift = ((self.v >> 4) & 4) | (self.v & 2);
        (attr_byte >> shift) & 3
    }

    fn fetch_bg_tile_lo(&mut self) -> u8 {
        let fine_y = (self.v >> 12) & 7;
        let table = self.ctrl.bknd_pattern_addr();
        let tile = self.nametable_byte as u16;

        let addr = table + fine_y + (tile * 16);
        self.mem_read(addr)
    }

    fn fetch_bg_tile_high(&mut self) -> u8 {
        let fine_y = (self.v >> 12) & 7;
        let table = self.ctrl.bknd_pattern_addr();
        let tile = self.nametable_byte as u16;

        let addr = table + fine_y + (tile * 16) + 8; // Notice the +8 here
        self.mem_read(addr)
    }

    fn load_shift_registers(&mut self) {
        // Load the latched tile data into the shift registers
        self.bg_shifter_pattern_lo = (self.bg_shifter_pattern_lo & 0xFF00) | self.bg_tile_lo as u16;
        self.bg_shifter_pattern_hi = (self.bg_shifter_pattern_hi & 0xFF00) | self.bg_tile_hi as u16;

        self.bg_shifter_attrib_lo &= 0xFF00;

        if self.attribute_byte & 0b01 != 0 {
            self.bg_shifter_attrib_lo |= 0xFF;
        } else {
            self.bg_shifter_attrib_lo |= 0x00;
        }

        self.bg_shifter_attrib_hi &= 0xFF00;

        if self.attribute_byte & 0b10 != 0 {
            self.bg_shifter_attrib_hi |= 0xFF;
        } else {
            self.bg_shifter_attrib_hi |= 0x00;
        }
    }

    fn update_shift_registers(&mut self) {
        if self.mask.show_background() {
            // Shifting background tile pattern row
            self.bg_shifter_pattern_lo <<= 1;
            self.bg_shifter_pattern_hi <<= 1;

            // Shifting palette attributes by 1
            self.bg_shifter_attrib_lo <<= 1;
            self.bg_shifter_attrib_hi <<= 1;
        }

        if self.mask.show_sprites() && self.cycle >= 2 && self.cycle < 258 {
            for (i, sprite) in self.secondary_oam_data.iter_mut().enumerate() {
                if let Some(sprite) = sprite {
                    if sprite.x > 0 {
                        sprite.x -= 1;
                    } else {
                        self.sprite_shifter_pattern_lo[i] <<= 1;
                        self.sprite_shifter_pattern_hi[i] <<= 1;
                    }
                }
            }
        }
    }

    fn fetch_internal_registers(&mut self) {
        match self.cycle % 8 {
            1 => {
                self.load_shift_registers();
                self.nametable_byte = self.fetch_nametable_byte();
            }
            3 => {
                self.attribute_byte = self.fetch_attribute_table_byte();
            }
            5 => {
                self.bg_tile_lo = self.fetch_bg_tile_lo();
            }
            7 => {
                self.bg_tile_hi = self.fetch_bg_tile_high();
            }
            0 => self.increment_x(),
            _ => (),
        }
    }

    fn sprite_evaluation(&mut self) {
        if self.mask.rendering_enabled() {
            self.secondary_oam_data.clear();

            let next_scanline = (self.scanline + 1) as i16;
            let mut sprite_count = 0;

            for n in 0..64 {
                let offset = n * 4;
                let oam_entry = &self.oam_data[offset..offset + 4];
                let sprite_y = (oam_entry[0] as u16) + 1;
                let diff = next_scanline - sprite_y as i16;
                let sprite_size = self.ctrl.sprite_size();

                if diff >= 0 && diff < sprite_size as i16 {
                    if sprite_count >= 8 {
                        self.status.set_sprite_overflow(true);
                        break;
                    }

                    sprite_count += 1;

                    let sprite: Sprite = Sprite::from(oam_entry, n == 0);
                    self.secondary_oam_data.push(Some(sprite));
                }
            }
        }
    }

    fn load_sprites(&mut self) {
        if self.mask.show_sprites() {
            let next_scanline = (self.scanline + 1) as u16;

            for (i, sprite) in self.secondary_oam_data.iter().enumerate() {
                if let Some(ref sprite) = sprite {
                    let sprite_height = self.ctrl.sprite_size();

                    // Determine fine y for vertical flipping
                    let fine_y = if sprite.flip_v() {
                        (sprite_height - 1) - (next_scanline - sprite.y) as u8
                    } else {
                        (next_scanline - sprite.y) as u8
                    };

                    // In 8x16 mode the PPU ignores the sprite pattern table selection in the CTRL register
                    // the bottom tile is the next one
                    // The tile number selection is then the upper bits of the tile attribute (upper tile)
                    // The table selection instead comes from the first bit of the sprite's tile attribute
                    // Re-adjusting the fine y will also be necessary
                    let (pattern_table, tile, fine_y) = if sprite_height == 16 {
                        let bottom_tile = fine_y > 7;
                        (
                            sprite.pattern_table_8x16(),
                            sprite.tile_number_8x16() + if bottom_tile { 1 } else { 0 },
                            fine_y - if bottom_tile { 8 } else { 0 },
                        )
                    } else {
                        (self.ctrl.sprt_pattern_addr(), sprite.tile, fine_y)
                    };
                    let pattern = self.read_pattern(pattern_table, tile, fine_y);

                    // Reverse bit pattern if the sprite is horizontally flipped
                    let pattern = if sprite.flip_h() {
                        (self.flip_byte(pattern.0), self.flip_byte(pattern.1))
                    } else {
                        pattern
                    };

                    // Reverse bit pattern if the sprite is horizontally flipped
                    let (address_lo, address_hi) = pattern;

                    self.sprite_shifter_pattern_lo[i] = address_lo;
                    self.sprite_shifter_pattern_hi[i] = address_hi;
                }
            }
        }
    }

    fn read_pattern(&self, base: u16, tile_no: u8, fine_y: u8) -> (u8, u8) {
        let tile_offset = (tile_no as u16 * 16) + fine_y as u16;

        let lo = self.mem_read(base + tile_offset);
        let hi = self.mem_read(base + tile_offset + 8);

        (lo, hi)
    }

    fn flip_byte(&self, mut b: u8) -> u8 {
        b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
        b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
        b = (b & 0xAA) >> 1 | (b & 0x55) << 1;

        b
    }

    fn render_pixel(&mut self) {
        let (bg_pixel, bg_palette) = self.render_background();
        let (fg_pixel, fg_palette, fg_priority, is_sprite_0) = self.render_foreground();

        let mut pixel = 0x00;
        let mut palette = 0x00;

        if bg_pixel == 0 && fg_pixel == 0 {
            pixel = 0x00;
            palette = 0x00;
        } else if bg_pixel == 0 && fg_pixel > 0 {
            pixel = fg_pixel;
            palette = fg_palette;
        } else if bg_pixel > 0 && fg_pixel == 0 {
            pixel = bg_pixel;
            palette = bg_palette;
        } else if bg_pixel > 0 && fg_pixel > 0 {
            if fg_priority {
                pixel = fg_pixel;
                palette = fg_palette;
            } else {
                pixel = bg_pixel;
                palette = bg_palette;
            }

            // https://www.nesdev.org/wiki/PPU_OAM#Sprite_zero_hits
            if self.mask.rendering_enabled() && self.cycle != 255 {
                self.status.set_sprite_zero_hit(is_sprite_0)
            }
        }

        let mut color = self.fetch_color_from_palette(palette, pixel);

        if self.mask.is_grayscale() {
            color &= &0x30
        }

        let rgb = palette::SYSTEM_PALETTE[color as usize];

        self.frame
            .set_pixel(self.cycle - 1, self.scanline as usize, rgb);
    }

    fn render_background(&self) -> (u8, u8) {
        let mut bg_pixel = 0x00; // The 2-bit pixel to be rendered
        let mut bg_palette = 0x00; // The 3-bit index of the palette the pixel indexes

        if self.mask.show_background() && (self.cycle >= 8 || self.mask.leftmost_8pxl_background())
        {
            //get the bit value from the pattern shift registers
            let bit_mux = 0x8000 >> self.fine_x;

            // fetch the pattern bits
            let p0_pixel = (self.bg_shifter_pattern_lo & bit_mux) > 0;
            let p1_pixel = (self.bg_shifter_pattern_hi & bit_mux) > 0;

            //combine the pattern bits
            bg_pixel = (p1_pixel as u8) << 1 | (p0_pixel as u8);

            // fetch the palette bits
            let palette0_pixel = (self.bg_shifter_attrib_lo & bit_mux) > 0;
            let palette1_pixel = (self.bg_shifter_attrib_hi & bit_mux) > 0;

            //combine the pattern bits
            bg_palette = (palette1_pixel as u8) << 1 | (palette0_pixel as u8);
        }

        (bg_pixel, bg_palette)
    }

    fn render_foreground(&self) -> (u8, u8, bool, bool) {
        let mut fg_pixel = 0x00;
        let mut fg_palette = 0x00;
        let mut fg_priority = false;
        let mut is_sprite_0 = false;

        if self.mask.show_sprites() && (self.cycle >= 8 || self.mask.leftmost_8pxl_sprite()) {
            // Iterate through all sprites for this scanline. This is to maintain
            // sprite priority. As soon as we find a non transparent pixel of
            // a sprite we can abort

            for (i, sprite) in self.secondary_oam_data.iter().enumerate() {
                if let Some(sprite) = sprite {
                    // Scanline cycle has "collided" with sprite, shifters taking over
                    if sprite.x == 0 {
                        // Note Fine X scrolling does not apply to sprites, the game
                        // should maintain their relationship with the background. So
                        // we'll just use the MSB of the shifter

                        // Determine the pixel value...
                        let fg_pixel_lo = (self.sprite_shifter_pattern_lo[i] & 0x80) > 0;
                        let fg_pixel_hi = (self.sprite_shifter_pattern_hi[i] & 0x80) > 0;
                        fg_pixel = ((fg_pixel_hi as u8) << 1) | fg_pixel_lo as u8;

                        // Extract the palette from the bottom two bits. Recall
                        // that foreground palettes are the latter 4 in the
                        // palette memory.
                        fg_palette = sprite.palette();
                        fg_priority = sprite.priority();
                        is_sprite_0 = sprite.is_sprite_0;

                        // If pixel is not transparent, we render it, and dont
                        // bother checking the rest because the earlier sprites
                        // in the list are higher priority
                        if fg_pixel != 0 {
                            break;
                        }
                    }
                }
            }
        }

        (fg_pixel, fg_palette, fg_priority, is_sprite_0)
    }

    fn fetch_color_from_palette(&self, palette: u8, pixel: u8) -> u8 {
        let palette_addr = 0x3F00 + (palette as u16 * 4) + pixel as u16;
        self.mem_read(palette_addr)
    }

    fn mirror_nametable(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0x0FFF;
        let nametable_index = mirrored_vram / 0x400;
        match (
            &self.rom.as_ref().unwrap().lock().unwrap().mirroring,
            nametable_index,
        ) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => mirrored_vram - 0x800,
            (Mirroring::Horizontal, 1) | (Mirroring::Horizontal, 2) => mirrored_vram - 0x400,
            (Mirroring::Horizontal, 3) => mirrored_vram - 0x800,
            _ => mirrored_vram,
        }
    }

    fn mirror_palette(&self, address: u16) -> usize {
        let address = (address as usize) % 0x20;

        match address {
            0x10 | 0x14 | 0x18 | 0x1C => address - 0x10,
            _ => address,
        }
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0..=0x1fff => self
                .rom
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .mapper
                .read(address),
            0x2000..=0x3eff => self.vram[self.mirror_nametable(address) as usize],
            0x3f00..=0x3fff => self.palette_table[self.mirror_palette(address)],
            _ => panic!("unexpected access to mirrored space {}", address),
        }
    }

    fn mem_write(&mut self, address: u16, data: u8) {
        match address {
            0..=0x1fff => self
                .rom
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .mapper
                .write(address, data),
            0x2000..=0x2fff => self.vram[self.mirror_nametable(address) as usize] = data,
            0x3000..=0x3eff => {
                unimplemented!("address {} shouldn't be used in reallity", address)
            }
            0x3f00..=0x3fff => self.palette_table[self.mirror_palette(address)] = data,
            _ => panic!("unexpected access to mirrored space {}", address),
        }
    }

    pub fn cpu_read(&mut self, address: u16) -> u8 {
        match address {
            PPUCTRL | PPUMASK | OAMADDR | PPUSCROLL | PPUADDR => self.data_bus,
            PPUSTATUS => {
                let mut data = self.status.snapshot();

                data &= 0xE0; // Clear the lower 5 bits
                data |= self.data_bus & 0x1f; // Set the lower 5 bits to the last value written to PPU

                self.status.reset_vblank_status();
                self.scroll.reset_latch();
                self.addr.reset_latch();

                // w:                  <- 0
                self.w = false;

                if self.scanline == 241 && self.cycle == 0 {
                    self.suppress_vbl = true;
                }

                self.data_bus |= data & 0xE0;
                data
            }
            OAMDATA => self.oam_data[self.oam_addr as usize],
            PPUDATA => {
                // let addr = self.addr.get();
                let address = self.v & 0x3fff;
                self.increment_vram_addr();

                // TODO: Verify behavior
                if address >= 0x3F00 {
                    self.vram_buffer = self.vram[self.mirror_nametable(address) as usize];
                    self.mem_read(address)
                } else {
                    let result = self.vram_buffer;
                    self.vram_buffer = self.mem_read(address);
                    result
                }
            }
            0x2008..=0x3FFF => {
                let mirror_down_addr = address & 0x2007;
                self.cpu_read(mirror_down_addr)
            }
            _ => {
                println!("Ignoring mem access at {:04X}", address);
                0
            }
        }
    }

    pub fn cpu_write(&mut self, address: u16, data: u8) {
        self.data_bus = data;

        match address {
            PPUCTRL => {
                let before_nmi_status = self.ctrl.generate_vblank_nmi();
                self.ctrl.update(data);
                // t: ...GH.. ........ <- d: ......GH
                // <used elsewhere> <- d: ABCDEF..
                self.t = (self.t & 0xF3FF) | ((data as u16 & 0x03) << 10);

                let updated_nmi_status = self.ctrl.generate_vblank_nmi();

                if !before_nmi_status && updated_nmi_status && self.status.is_in_vblank() {
                    self.nmi_interrupt = Some(1)
                }
            }
            PPUMASK => {
                self.mask.update(data);
            }
            PPUSTATUS => (),

            OAMADDR => self.oam_addr = data,
            OAMDATA => {
                self.oam_data[self.oam_addr as usize] = data;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            PPUSCROLL => {
                self.scroll.write(data);

                if !self.w {
                    // t: ....... ...ABCDE <- d: ABCDE...
                    // x:              FGH <- d: .....FGH
                    // w:                  <- 1
                    self.t = (self.t & 0x7FE0) | data as u16 >> 3;
                    self.fine_x = data & 0x07;
                    self.w = true;
                } else {
                    // t: FGH..AB CDE..... <- d: ABCDEFGH
                    // w:                  <- 0
                    self.t &= 0x0C1F;
                    self.t |= ((data & 0x07) as u16) << 12;
                    self.t |= ((data & 0xF8) as u16) << 2;
                    self.w = false;
                }
            }

            PPUADDR => {
                self.addr.update(data);

                if !self.w {
                    // t: .CDEFGH ........ <- d: ..CDEFGH
                    //        <unused>     <- d: AB......
                    // t: Z...... ........ <- 0 (bit Z is cleared)
                    // w:                  <- 1
                    self.t = (self.t & 0x00FF) | ((data & 0x3F) as u16) << 8;
                    self.w = true;
                } else {
                    // t: ....... ABCDEFGH <- d: ABCDEFGH
                    // v: <...all bits...> <- t: <...all bits...>
                    // w:                  <- 0
                    self.t = (self.t & 0xFF00) | data as u16;
                    self.v = self.t;
                    self.w = false;
                }
            }
            PPUDATA => {
                // let addr = self.addr.get();
                let address = self.v & 0x3fff;

                self.mem_write(address, data);

                if self.mask.show_background() && (self.scanline < 240 || self.scanline == 261) {
                    self.increment_x();
                    self.increment_y();
                } else {
                    self.increment_vram_addr();
                }
            }
            0x2008..=0x3FFF => {
                let mirror_down_addr = address & 0b00100000_00000111;
                self.cpu_write(mirror_down_addr, data);
            }
            _ => {
                panic!("Ignoring mem write-access at {:04X}", address);
            }
        }
    }

    pub fn poll_nmi_interrupt(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    pub fn tick(&mut self) -> bool {
        let scanline_step = Scanline::from(self.scanline as usize);

        match (scanline_step, self.cycle) {
            (_, 0) => {
                //  idle cycle
            }
            (Scanline::PreRender, 1..=256) => {
                if self.cycle == 1 {
                    self.status.reset_vblank_status();
                    self.status.set_sprite_zero_hit(false);
                    self.status.set_sprite_overflow(false);
                    self.nmi_interrupt = None;
                    self.suppress_vbl = false;

                    self.sprite_shifter_pattern_lo = [0; 8];
                    self.sprite_shifter_pattern_hi = [0; 8];
                }

                self.fetch_internal_registers();

                if self.cycle == 256 {
                    self.increment_y();
                }
            }
            (Scanline::PreRender, 257) => self.transfer_x(),
            (Scanline::PreRender, 280..=304) => self.transfer_y(),
            (Scanline::PreRender, 321..=336) => {
                self.update_shift_registers();
                self.fetch_internal_registers()
            }
            (Scanline::PreRender, 337) => {
                self.fetch_nametable_byte();
            }
            (Scanline::PreRender, 339) => {
                self.fetch_nametable_byte();

                // The "Skipped on BG+odd" tick is implemented by jumping directly
                // from (339, 261) to (0, 0), meaning the last tick of the last NT
                // fetch takes place at (0, 0) on odd frames replacing the idle tick
                if self.mask.rendering_enabled() && self.odd_frame {
                    self.cycle = 340;
                }
            }
            (Scanline::Visible, 1..=256) => {
                self.update_shift_registers();
                self.fetch_internal_registers();

                if self.cycle == 256 {
                    self.increment_y();
                }

                self.render_pixel();
            }
            (Scanline::Visible, 257) => {
                self.transfer_x();
                self.update_shift_registers();
                self.sprite_evaluation();
            }
            (Scanline::Visible, 321..=336) => {
                self.update_shift_registers();
                self.fetch_internal_registers()
            }
            (Scanline::Visible, 337 | 339) => {
                // Unused NT fetches
                self.fetch_nametable_byte();
            }
            (Scanline::Visible, 340) => self.load_sprites(),
            (Scanline::PostRender, _) => {
                //Idle. Do nothing
            }
            (Scanline::VBlank, 1) => {
                if self.scanline == 241 {
                    if !self.suppress_vbl {
                        self.status.set_vblank_status(true);
                        if self.ctrl.generate_vblank_nmi() {
                            self.nmi_interrupt = Some(1);
                        }
                    }
                }
            }
            _ => (),
        }

        // cycle:    0 - 340
        // scanline: 0 - 261
        self.cycle += 1;
        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = 0;
                self.frame_count += 1;
                self.odd_frame = !self.odd_frame;

                return true;
            }
        }

        return false;
    }
}
