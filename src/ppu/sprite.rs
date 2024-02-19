#[derive(Default, Copy, Clone)]
pub struct Sprite {
    pub tile: u8,

    // 76543210
    // ||||||||
    // ||||||++- Palette (4 to 7) of sprite
    // |||+++--- Unimplemented (read 0)
    // ||+------ Priority (0: in front of background; 1: behind background)
    // |+------- Flip sprite horizontally
    // +-------- Flip sprite vertically
    pub attribute: u8,

    pub x: u8,
    pub y: u16,

    pub is_sprite_0: bool,
}

impl Sprite {
    pub fn from(data: &[u8], is_sprite_0: bool) -> Self {
        Sprite {
            y: data[0] as u16 + 1,
            tile: data[1],
            attribute: data[2],
            x: data[3],
            is_sprite_0,
        }
    }

    pub fn flip_v(&self) -> bool {
        self.attribute & 0x80 == 0x80
    }
    pub fn flip_h(&self) -> bool {
        self.attribute & 0x40 == 0x40
    }

    pub fn priority(&self) -> bool {
        self.attribute & 0x20 != 0x20
    }

    pub fn palette(&self) -> u8 {
        (self.attribute & 0x3) + 0x04
    }

    pub fn pattern_table_8x16(&self) -> u16 {
        (self.tile & 0x01) as u16 * 0x1000
    }

    pub fn tile_number_8x16(&self) -> u8 {
        self.tile & 0xFE
    }
}
