use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    bus::Bus,
    cpu::CPU,
    ppu::{frame::Frame, palette},
    rom::{Mirroring, Rom},
    JoypadButton,
};

#[derive(PartialEq, Eq)]
pub enum EmulationStatus {
    Stopped,
    Running,
    Paused,
}

pub struct NES {
    pub cpu: CPU,
    pub rom: Option<Arc<Mutex<Rom>>>,
    pub status: EmulationStatus,
}

impl NES {
    pub fn new() -> Self {
        let bus = Bus::new();
        let cpu = CPU::new(bus);

        Self {
            cpu,
            rom: None,
            status: EmulationStatus::Stopped,
        }
    }

    pub fn emulate_frame(&mut self) -> Option<&Frame> {
        let cycles = self.cpu.run();

        self.cpu.bus.tick(cycles)
    }

    pub fn button_pressed(&mut self, key: JoypadButton, pressed: bool) {
        self.cpu.bus.joypad1.set_button_pressed_status(key, pressed);
    }

    pub fn load_rom<P: AsRef<Path>>(&mut self, path: P) {
        let game_code = fs::read(path).expect("Should have been able to read the game");

        self.load_rom_bytes(&game_code);
    }

    pub fn load_rom_bytes(&mut self, game_code: &[u8]) {
        let rom = Rom::new(game_code).unwrap();

        let rom_rc = Arc::new(Mutex::new(rom));
        self.cpu.bus.load_rom(rom_rc.clone());
        self.rom = Some(rom_rc);
        self.start_emulation();
    }

    pub fn start_emulation(&mut self) {
        self.cpu.reset();
        self.status = EmulationStatus::Running;
    }

    pub fn pause_emulation(&mut self) {
        if self.status == EmulationStatus::Running {
            self.status = EmulationStatus::Paused;
        }
    }
    pub fn continue_emulation(&mut self) {
        if self.status == EmulationStatus::Paused {
            self.status = EmulationStatus::Running;
        }
    }

    pub fn is_running(&self) -> bool {
        self.status == EmulationStatus::Running
    }

    fn pattern_table(&self, bank_index: usize) -> Frame {
        let mut pattern_table = Frame::new(128, 128);

        let palette = &self.cpu.bus.ppu.palette_table[0..4];

        let rom = self.rom.as_ref().unwrap().lock().unwrap();
        let chr_rom = &rom.chr_rom;

        let offset = bank_index * 128;
        let mut tile_y = 0;
        let mut tile_x = offset;

        let bank = bank_index * 0x1000;

        for tile_n in 0..256 {
            if tile_n != 0 && tile_n % 16 == 0 {
                tile_y += 8;
                tile_x = offset;
            }
            let tile = &chr_rom[(bank + tile_n * 16)..(bank + tile_n * 16 + 16)];

            for y in 0..8 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];

                for x in (0..8).rev() {
                    let value = (1 & upper) << 1 | (1 & lower);
                    upper >>= 1;
                    lower >>= 1;

                    let rgb = match value {
                        0..=3 => palette::SYSTEM_PALETTE[palette[value as usize] as usize],
                        _ => panic!("can't be"),
                    };

                    pattern_table.set_pixel(tile_x + x, tile_y + y, rgb)
                }
            }

            tile_x += 8;
        }

        pattern_table
    }

    pub fn ppu_viewer(&self) -> (Frame, Frame) {
        (self.pattern_table(0), self.pattern_table(1))
    }

    pub fn palette_viewer(&self) -> Frame {
        let mut frame = Frame::new(256, 8);

        let mut tile_x = 0;
        for color in self.cpu.bus.ppu.palette_table {
            let rgb = palette::SYSTEM_PALETTE[color as usize];
            for y in 0..8 {
                for x in 0..8 {
                    frame.set_pixel(tile_x + x, y, rgb);
                }
            }
            tile_x += 8;
        }

        frame
    }

    fn bg_pallette(
        &self,
        palette_table: &[u8],
        attribute_table: &[u8],
        tile_x: usize,
        tile_y: usize,
    ) -> [u8; 4] {
        let group = tile_y / 4 * 8 + tile_x / 4;
        let attribute_byte = attribute_table[group];

        let shift = ((tile_y & 0x02) << 1) | (tile_x & 0x02);
        let palette_idx = (attribute_byte >> shift) & 0x03;

        let palette_base = (palette_idx as usize) * 4;

        [
            palette_table[0],
            palette_table[palette_base + 1],
            palette_table[palette_base + 2],
            palette_table[palette_base + 3],
        ]
    }

    pub fn nametable_viewer(&self) -> Frame {
        let mut frame = Frame::new(512, 480);
        let mut x_offset = 0;
        let mut y_offset = 0;

        let rom = self.rom.as_ref().unwrap().lock().unwrap();
        let chr_rom = &rom.chr_rom;
        let ppu_ctrl_bank = self.cpu.bus.ppu.ctrl.bknd_pattern_addr() as usize;

        for nametable in self.cpu.bus.ppu.vram.chunks(0x400) {
            let attribute_table = &nametable[0x3c0..0x400];

            for i in 0..0x3c0 {
                let tile_index = nametable[i] as usize;
                let tile_block = &chr_rom
                    [(ppu_ctrl_bank + tile_index * 16)..(ppu_ctrl_bank + tile_index * 16 + 16)];

                let tile_x = i % 32;
                let tile_y = i / 32;

                let palette = self.bg_pallette(
                    &self.cpu.bus.ppu.palette_table,
                    attribute_table,
                    tile_x,
                    tile_y,
                );

                for y in 0..8 {
                    let lower = tile_block[y];
                    let upper = tile_block[y + 8];

                    for x in 0..8 {
                        let value = ((lower >> x) & 0x01) | (((upper >> x) & 0x01) << 1);

                        let rgb = match value {
                            0..=3 => palette::SYSTEM_PALETTE[palette[value as usize] as usize],
                            _ => panic!("can't be"),
                        };

                        let x = tile_x * 8 + (7 - x);
                        let y = tile_y * 8 + y;

                        frame.set_pixel(x_offset + x, y_offset + y, rgb);

                        if rom.mirroring == Mirroring::Vertical {
                            frame.set_pixel(x_offset + x, y_offset + y + 240, rgb);
                        } else {
                            frame.set_pixel(x_offset + x + 256, y_offset + y, rgb);
                        }
                    }
                }
            }

            if rom.mirroring == Mirroring::Vertical {
                x_offset = 256;
            } else {
                y_offset = 240;
            }
        }

        frame
    }
}

impl Default for NES {
    fn default() -> Self {
        Self::new()
    }
}
