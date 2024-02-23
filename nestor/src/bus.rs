use std::sync::{Arc, Mutex};

use crate::{
    cartridge::Rom,
    joypad::Joypad,
    ppu::{frame::Frame, PPU},
};

pub struct Bus {
    cpu_vram: [u8; 2048],
    rom: Arc<Mutex<Rom>>,
    pub ppu: PPU,
    pub joypad1: Joypad,
}

impl Bus {
    pub fn new(rom: Rom) -> Bus {
        let rom_rc = Arc::new(Mutex::new(rom));
        let ppu = PPU::new(Arc::clone(&rom_rc));

        Bus {
            cpu_vram: [0; 2048],
            rom: rom_rc,
            ppu,
            joypad1: Joypad::new(),
        }
    }

    pub fn tick(&mut self, cycles: u8) -> Option<&Frame> {
        let mut frame_complete = false;

        for _ in 0..(cycles * 3) {
            if self.ppu.tick() {
                frame_complete = true;
                break;
            }
        }

        if frame_complete {
            Some(&self.ppu.frame)
        } else {
            None
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.poll_nmi_interrupt()
    }

    pub fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.cpu_vram[mirror_down_addr as usize]
            }
            0x2000..=0x3FFF => self.ppu.cpu_read(addr),
            0x4000..=0x4015 => {
                //ignore APU
                0
            }

            0x4016 => self.joypad1.read(),

            0x4017 => {
                // ignore joypad 2
                0
            }

            // SRAM
            0x6000..=0x7fff => self.rom.lock().unwrap().mapper.read(addr),
            // 0x8000..=0xFFFF => self.read_prg_rom(addr),
            0x8000..=0xFFFF => self.rom.lock().unwrap().mapper.read(addr),

            _ => {
                println!("Ignoring mem access at {:04X}", addr);
                0
            }
        }
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                let mirror_down_addr = addr & 0b11111111111;
                self.cpu_vram[mirror_down_addr as usize] = data;
            }
            0x2000..=0x3FFF => self.ppu.cpu_write(addr, data),
            0x4000..=0x4013 | 0x4015 => {
                //ignore APU
            }

            0x4016 => {
                self.joypad1.write(data);
            }

            0x4017 => {
                // ignore joypad 2
            }
            0x4014 => {
                let hi: u16 = (data as u16) << 8;
                for i in 0..256u16 {
                    let value = self.mem_read(hi + i);
                    self.ppu.oam_data[self.ppu.oam_addr as usize] = value;
                    self.ppu.oam_addr = self.ppu.oam_addr.wrapping_add(1);
                }
            }
            // SRAM
            0x6000..=0x7fff => self.rom.lock().unwrap().mapper.write(addr, data),
            // PRG-ROM
            0x8000..=0xFFFF => self.rom.lock().unwrap().mapper.write(addr, data),
            _ => {
                panic!("Ignoring mem write-access at {:04X}", addr);
            }
        }
    }

    pub fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read(pos);
        let hi = self.mem_read(pos.wrapping_add(1) as u16);
        (hi as u16) << 8 | (lo as u16)
    }

    pub fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
}