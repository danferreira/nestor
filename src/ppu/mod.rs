use std::cell::RefCell;
use std::rc::Rc;

use crate::cartridge::{Mirroring, Rom};
use crate::render::frame::Frame;
use crate::render::palette::{self, SYSTEM_PALETTE};
use registers::addr::AddrRegister;
use registers::control::ControlRegister;
use registers::mask::MaskRegister;
use registers::scroll::ScrollRegister;
use registers::status::StatusRegister;

pub mod registers;
pub struct NesPPU {
    pub rom: Rc<RefCell<Rom>>,
    pub vram: [u8; 2048],
    pub palette_table: [u8; 32],
    pub oam_data: [u8; 256],
    pub oam_addr: u8,

    pub scanline: i16,
    cycles: usize,
    frames: usize,

    pub mask: MaskRegister,
    pub addr: AddrRegister,
    pub ctrl: ControlRegister,
    pub scroll: ScrollRegister,
    pub status: StatusRegister,

    //internal registers
    v: u16,
    t: u16,
    x: u8,
    w: bool,

    nametable_byte: u8,
    attribute_byte: u8,
    bg_tile_lo: u8,
    bg_tile_hi: u8,

    // Two 16-bit shift registers for the pattern table data
    bg_shifter_pattern_lo: u16,
    bg_shifter_pattern_hi: u16,

    // Two 16-bit shift registers for the attribute table data
    bg_shifter_attrib_lo: u16,
    bg_shifter_attrib_hi: u16,

    internal_data_buf: u8,
    pub nmi_interrupt: Option<u8>,

    // The last written value to any PPU register
    // For use when reading the PPUSTATUS
    pub last_value: u8,

    // Odd/even frame state
    odd_frame: bool,

    pub frame: Frame,
}

impl NesPPU {
    pub fn new(rom: Rc<RefCell<Rom>>) -> Self {
        NesPPU {
            rom,
            vram: [0; 2048],
            oam_data: [0; 64 * 4],
            oam_addr: 0,
            palette_table: [0; 32],
            internal_data_buf: 0,

            mask: MaskRegister::new(),
            ctrl: ControlRegister::new(),
            addr: AddrRegister::new(),
            scroll: ScrollRegister::new(),
            status: StatusRegister::new(),

            v: 0,
            t: 0,
            x: 0,
            w: false,

            nametable_byte: 0,
            attribute_byte: 0,
            bg_tile_lo: 0,
            bg_tile_hi: 0,

            bg_shifter_pattern_lo: 0,
            bg_shifter_pattern_hi: 0,

            bg_shifter_attrib_lo: 0,
            bg_shifter_attrib_hi: 0,

            scanline: 0,
            cycles: 0,
            frames: 0,
            nmi_interrupt: None,
            last_value: 0,

            odd_frame: false,

            frame: Frame::new(256, 240),
        }
    }

    fn write_to_ctrl(&mut self, value: u8) {
        let before_nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);
        let updated_nmi_status = self.ctrl.generate_vblank_nmi();

        if !before_nmi_status && updated_nmi_status && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(1)
        }

        // t: ...GH.. ........ <- d: ......GH
        // <used elsewhere> <- d: ABCDEF..
        self.t = (self.t & 0xF3FF) | ((value as u16 & 0x03) << 10)
    }

    fn write_to_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    fn write_to_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.write_to_oam_data(*x);
        }
    }

    fn read_status(&mut self) -> u8 {
        let mut data = self.status.snapshot();

        data &= !0x1f; // Clear the lower 5 bits
        data |= self.last_value & 0x1f; // Set the lower 5 bits to the last value written to PPU

        self.status.reset_vblank_status();
        self.addr.reset_latch();
        self.scroll.reset_latch();

        // w:                  <- 0
        self.w = false;

        data
    }

    fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    fn write_to_scroll(&mut self, value: u8) {
        self.scroll.write(value);

        if !self.w {
            // t: ....... ...ABCDE <- d: ABCDE...
            // x:              FGH <- d: .....FGH
            // w:                  <- 1
            self.t = (self.t & 0x7FE0) | value as u16 >> 3;
            self.x = value & 0x07;
            self.w = true;
        } else {
            // t: FGH..AB CDE..... <- d: ABCDEFGH
            // w:                  <- 0
            self.t |= ((value & 0x07) as u16) << 12;
            self.t |= ((value & 0xF8) as u16) << 2;
            self.w = false;
        }
    }

    fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);

        if !self.w {
            // t: .CDEFGH ........ <- d: ..CDEFGH
            //        <unused>     <- d: AB......
            // t: Z...... ........ <- 0 (bit Z is cleared)
            // w:                  <- 1
            // ..FEDCBA ........
            self.t = (self.t & 0x00FF) | ((value & 0x3F) as u16) << 8;
            self.w = true;
        } else {
            // t: ....... ABCDEFGH <- d: ABCDEFGH
            // v: <...all bits...> <- t: <...all bits...>
            // w:                  <- 0
            self.t = (self.t & 0xFF00) | value as u16;
            self.v = self.t;
            self.w = false;
        }
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());

        self.v = self.v.wrapping_add(self.ctrl.vram_addr_increment() as u16);
    }

    fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();

        match addr {
            0..=0x1fff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.rom.borrow().mapper.read(addr);
                result
            }
            0x2000..=0x2fff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            0x3000..=0x3eff => panic!(
                "addr space 0x3000..0x3eff is not expected to be used, requested = {} ",
                addr
            ),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize]
            }

            0x3f00..=0x3fff => self.palette_table[(addr - 0x3f00) as usize],
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }

    fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.get();

        match addr {
            0..=0x1fff => self.rom.borrow_mut().mapper.write(addr, value),
            0x2000..=0x2fff => {
                self.vram[self.mirror_vram_addr(addr) as usize] = value;
            }
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used in reallity", addr),

            //Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize] = value;
            }
            0x3f00..=0x3fff => {
                let mut i = addr as usize % 0x20;

                match i & 0x00ff {
                    // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of
                    // $3F00/$3F04/$3F08/$3F0C
                    0x10 | 0x14 | 0x18 | 0x1c => {
                        i &= 0xff0f;
                    }
                    _ => {}
                }

                self.palette_table[i] = value;
                // self.palette_table[(addr - 0x3f00) as usize] = value;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }

    fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff

        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400; // to the name table index
        match (&self.rom.borrow().mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    fn is_sprite_0_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == self.scanline as usize) && x <= cycle && self.mask.show_sprites()
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

    fn update_shifters(&mut self) {
        if self.mask.show_background() {
            // Shifting background tile pattern row
            self.bg_shifter_pattern_lo <<= 1;
            self.bg_shifter_pattern_hi <<= 1;

            // Shifting palette attributes by 1
            self.bg_shifter_attrib_lo <<= 1;
            self.bg_shifter_attrib_hi <<= 1;
        }
    }

    pub fn tick(&mut self) -> bool {
        let rendering_enabled = self.mask.show_background() || self.mask.show_sprites();
        let pre_render_scanline = self.scanline == -1 || self.scanline == 261;
        let visible_scanline = self.scanline >= 0 && self.scanline <= 239;
        let render_line = pre_render_scanline || visible_scanline;
        let visible_cycles = self.cycles >= 1 && self.cycles <= 256;
        let next_cycles = self.cycles >= 321 && self.cycles <= 336;

        if pre_render_scanline {
            if self.cycles == 1 {
                self.status.reset_vblank_status();
                self.status.set_sprite_zero_hit(false);
                self.status.set_sprite_overflow(false);
            }

            if self.cycles >= 280 && self.cycles <= 304 {
                self.transfer_y()
            }
        }

        if self.scanline == 0 && self.cycles == 0 {
            self.frame = Frame::new(256, 240);
            self.cycles = 1;
            return false;
        }

        if visible_scanline && visible_cycles {
            self.render_pixel();
        }

        if render_line && (visible_cycles || next_cycles) {
            self.update_shifters();
            match (self.cycles - 1) % 8 {
                0 => {
                    self.nametable_byte = self.fetch_nametable_byte();
                }
                2 => {
                    self.attribute_byte = self.fetch_attribute_table_byte();
                }
                4 => {
                    self.bg_tile_lo = self.fetch_bg_tile_lo();
                }
                6 => {
                    self.bg_tile_hi = self.fetch_bg_tile_high();
                }
                7 => {
                    self.load_shift_registers();
                    self.increment_x();
                }
                _ => (),
            }
        }

        if render_line {
            if self.cycles == 256 {
                self.increment_y();
            }

            if self.cycles == 257 {
                self.transfer_x();
            }
        }

        if self.scanline == 241 && self.cycles == 1 {
            self.status.set_vblank_status(true);
            if self.ctrl.generate_vblank_nmi() {
                self.nmi_interrupt = Some(1);
            }
        }

        if rendering_enabled {
            if self.odd_frame && self.scanline == 261 && self.cycles == 339 {
                self.cycles = 0;
                self.scanline = -1;
                self.odd_frame = false;
                return false;
            }
        }

        // cycle:    0 - 340
        // scanline: 0 - 261
        self.cycles += 1;
        if self.cycles >= 341 {
            self.cycles = 0;
            self.scanline += 1;

            if self.scanline >= 261 {
                self.status.reset_vblank_status();
                self.status.set_sprite_zero_hit(false);
                self.nmi_interrupt = None;
                self.scanline = -1;
                self.odd_frame = !self.odd_frame;
                self.frames += 1;
                return true;
            }
        }

        return false;
    }

    fn render_pixel(&mut self) {
        let mut bg_pixel = 0x00; // The 2-bit pixel to be rendered
        let mut bg_palette = 0x00; // The 3-bit index of the palette the pixel indexes
        if self.mask.show_background() {
            //get the bit value from the pattern shift registers
            let bit_mux = 0x8000 >> self.x;

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

        let color = self.fetch_color_from_palette(bg_palette, bg_pixel);
        let rgb = palette::SYSTEM_PALETTE[color as usize];

        // if self.cycles >= 0 {
        println!("cycles: {} scanline: {}", self.cycles, self.scanline);
        if self.cycles > 0 && self.scanline >= 0 {
            self.frame
                .set_pixel(self.cycles - 1, self.scanline as usize, rgb);
        }
        // }
    }

    fn fetch_color_from_palette(&self, palette: u8, pixel: u8) -> u8 {
        let palette_addr = 0x3F00 + (palette as u16 * 4) + pixel as u16;
        self.mem_read(palette_addr)
    }

    pub fn poll_nmi_interrupt(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0..=0x1fff => self.rom.borrow().mapper.read(address),
            0x2000..=0x2fff => self.vram[self.mirror_vram_addr(address) as usize],
            0x3000..=0x3eff => panic!(
                "addr space 0x3000..0x3eff is not expected to be used, requested = {} ",
                address
            ),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = address - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize]
            }

            0x3f00..=0x3fff => self.palette_table[(address - 0x3f00) as usize],
            _ => panic!("unexpected access to mirrored space {}", address),
        }
    }

    pub fn cpu_read(&mut self, address: u16) -> u8 {
        match address {
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => self.last_value,
            0x2002 => self.read_status(),
            0x2004 => self.read_oam_data(),
            0x2007 => self.read_data(),
            0x2008..=0x3FFF => {
                let mirror_down_addr = address & 0b00100000_00000111;
                self.cpu_read(mirror_down_addr)
            }
            _ => {
                println!("Ignoring mem access at {:04X}", address);
                0
            }
        }
    }

    pub fn cpu_write(&mut self, address: u16, data: u8) {
        self.last_value = data;
        match address {
            0x2000 => {
                self.write_to_ctrl(data);
            }
            0x2001 => {
                self.write_to_mask(data);
            }
            0x2002 => panic!("attempt to write to PPU status register"),

            0x2003 => {
                self.write_to_oam_addr(data);
            }
            0x2004 => {
                self.write_to_oam_data(data);
            }
            0x2005 => {
                self.write_to_scroll(data);
            }

            0x2006 => {
                self.write_to_ppu_addr(data);
            }
            0x2007 => {
                self.write_to_data(data);
            }
            0x2008..=0x3FFF => {
                let mirror_down_addr = address & 0b00100000_00000111;
                self.cpu_write(mirror_down_addr, data);
            }
            // https://wiki.nesdev.com/w/index.php/PPU_programmer_reference#OAM_DMA_.28.244014.29_.3E_write
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (data as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.mem_read(hi + i);
                }

                self.write_oam_dma(&buffer);

                // todo: handle this eventually
                // let add_cycles: u16 = if self.cycles % 2 == 1 { 514 } else { 513 };
                // self.tick(add_cycles); //todo this will cause weird effects as PPU will have 513/514 * 3 ticks
            }
            _ => {
                panic!("Ignoring mem write-access at {:04X}", address);
            }
        }
    }
}
