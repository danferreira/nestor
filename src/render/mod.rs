pub mod frame;
pub mod palette;

use crate::cartridge::Mirroring;
use crate::ppu::NesPPU;
use frame::Frame;

struct Viewport {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Viewport {
    fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Viewport { x1, y1, x2, y2 }
    }
}

fn bg_pallette(ppu: &NesPPU, attribute_table: &[u8], tile_x: usize, tile_y: usize) -> [u8; 4] {
    let group = tile_y / 4 * 8 + tile_x / 4;
    let attribute_byte = attribute_table[group];

    let shift = ((tile_y & 0x02) << 1) | (tile_x & 0x02);
    let palette_idx = (attribute_byte >> shift) & 0x03;

    let palette_base = (palette_idx as usize) * 4;

    [
        ppu.palette_table[0],
        ppu.palette_table[palette_base + 1],
        ppu.palette_table[palette_base + 2],
        ppu.palette_table[palette_base + 3],
    ]
}

fn sprite_palette(ppu: &NesPPU, palette_number: u8) -> [u8; 4] {
    let palette_base = 0x10 + (palette_number as usize) * 4;
    [
        ppu.palette_table[palette_base],
        ppu.palette_table[palette_base + 1],
        ppu.palette_table[palette_base + 2],
        ppu.palette_table[palette_base + 3],
    ]
}

fn render_nametable(
    ppu: &NesPPU,
    nametable: &[u8],
    viewport: &Viewport,
    shift_x: isize,
    shift_y: isize,
    frame: &mut Frame,
) {
    let attribute_table = &nametable[0x3c0..0x400];

    for i in 0..0x3c0 {
        let tile_index = nametable[i] as usize;
        let bank = ppu.ctrl.bknd_pattern_addr() as usize;

        let tile_block = &ppu.chr_rom[(bank + tile_index * 16)..(bank + tile_index * 16 + 16)];

        let tile_x = i % 32;
        let tile_y = i / 32;

        let palette = bg_pallette(ppu, attribute_table, tile_x, tile_y);

        for y in 0..8 {
            let mut lower = tile_block[y];
            let mut upper = tile_block[y + 8];

            for x in (0..8).rev() {
                let value = (lower & 0x01) | ((upper & 0x01) << 1);

                lower = lower >> 1;
                upper = upper >> 1;

                let rgb = match value {
                    0..=3 => palette::SYSTEM_PALETTE[palette[value as usize] as usize],
                    _ => panic!("can't be"),
                };

                let x = tile_x * 8 + x;
                let y = tile_y * 8 + y;

                if x >= viewport.x1 && x < viewport.x2 && y >= viewport.y1 && y < viewport.y2 {
                    frame.set_pixel(
                        (shift_x + x as isize) as usize,
                        (shift_y + y as isize) as usize,
                        rgb,
                    );
                }
            }
        }
    }
}

fn render_sprites(ppu: &NesPPU, frame: &mut Frame) {
    for oam_entry in ppu.oam_data.chunks(4).rev() {
        let y_position = oam_entry[0] as usize;
        let tile_idx = oam_entry[1] as usize;
        let attributes = oam_entry[2];
        let x_position = oam_entry[3] as usize;

        let flip_horizontal = (attributes & 0x40) == 0x40;
        let flip_vertical = (attributes & 0x80) == 0x80;

        let palette_idx = attributes & 0x03;
        let sprite_palette = sprite_palette(ppu, palette_idx);
        let bank = ppu.ctrl.sprt_pattern_addr() as usize;

        let tile_block = &ppu.chr_rom[(bank + tile_idx * 16)..(bank + tile_idx * 16 + 16)];

        for y in 0..8 {
            let mut lower = tile_block[y];
            let mut upper = tile_block[y + 8];

            for x in (0..8).rev() {
                let value = (lower & 0x01) | ((upper & 0x01) << 1);

                lower = lower >> 1;
                upper = upper >> 1;

                let rgb = match value {
                    0 => continue,
                    1..=3 => palette::SYSTEM_PALETTE[sprite_palette[value as usize] as usize],
                    _ => panic!("can't be"),
                };

                let fixed_x = if flip_horizontal { 7 - x } else { x };
                let fixed_y = if flip_vertical { 7 - y } else { y };

                frame.set_pixel(x_position + fixed_x, y_position + fixed_y, rgb)
            }
        }
    }
}

pub fn render(ppu: &NesPPU, frame: &mut Frame) {
    let (main_nametable, second_nametable) = match (&ppu.mirroring, ppu.ctrl.nametable_addr()) {
        (Mirroring::Vertical, 0x2000) | (Mirroring::Vertical, 0x2800) => {
            (&ppu.vram[0..0x400], &ppu.vram[0x400..0x800])
        }
        (Mirroring::Vertical, 0x2400) | (Mirroring::Vertical, 0x2C00) => {
            (&ppu.vram[0x400..0x800], &ppu.vram[0x0..0x400])
        }
        (Mirroring::Horizontal, 0x2000) | (Mirroring::Horizontal, 0x2400) => {
            (&ppu.vram[0..0x400], &ppu.vram[0x400..0x800])
        }
        (Mirroring::Horizontal, 0x2800) | (Mirroring::Horizontal, 0x2C00) => {
            (&ppu.vram[0x400..0x800], &ppu.vram[0x0..0x400])
        }

        _ => panic!("Not supported mirroring type {:?}", ppu.mirroring),
    };

    let scroll_x = ppu.scroll.scroll_x as usize;
    let scroll_y = ppu.scroll.scroll_y as usize;

    render_nametable(
        ppu,
        main_nametable,
        &Viewport::new(scroll_x, scroll_y, 256, 240),
        -(scroll_x as isize),
        -(scroll_y as isize),
        frame,
    );

    if scroll_x > 0 {
        render_nametable(
            ppu,
            second_nametable,
            &Viewport::new(0, 0, scroll_x, 240),
            (256 - scroll_x) as isize,
            0,
            frame,
        );
    } else if scroll_y > 0 {
        render_nametable(
            ppu,
            second_nametable,
            &Viewport::new(0, 0, 256, scroll_y),
            0,
            (240 - scroll_y) as isize,
            frame,
        );
    }

    render_sprites(ppu, frame);
}
