use std::sync::Arc;
use std::sync::Mutex;

use crate::{
    joypad::Joypad,
    mapper::Mapper,
    ppu::{frame::Frame, PPU},
    rom::ROM,
};

pub trait Memory {
    fn mem_read(&mut self, addr: u16) -> u8;
    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read(pos);
        let hi = self.mem_read(pos.wrapping_add(1));
        (hi as u16) << 8 | (lo as u16)
    }
    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
}

pub trait CpuBus {
    fn poll_nmi_status(&mut self) -> Option<u8>;
}

pub struct Bus {
    cpu_vram: [u8; 2048],
    pub ppu: PPU,
    pub joypad1: Joypad,
    pub joypad2: Joypad,
    mapper: Option<Arc<Mutex<Box<dyn Mapper + Send>>>>,
}

impl Bus {
    pub fn new() -> Bus {
        let ppu = PPU::new();
        Bus {
            cpu_vram: [0; 2048],
            ppu,
            joypad1: Joypad::new(),
            joypad2: Joypad::new(),
            mapper: None,
        }
    }

    pub fn load_rom(&mut self, rom: &ROM) {
        self.ppu.load_rom(rom);
        self.mapper = Some(Arc::clone(&rom.mapper));
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

    fn dma_transfer(&mut self, data: u8) {
        let hi: u16 = (data as u16) << 8;
        for i in 0..256u16 {
            let value = self.mem_read(hi + i);
            self.ppu.oam_data[self.ppu.oam_addr as usize] = value;
            self.ppu.oam_addr = self.ppu.oam_addr.wrapping_add(1);
        }
    }
}

impl Memory for Bus {
    fn mem_read(&mut self, addr: u16) -> u8 {
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
            0x4017 => self.joypad2.read(),

            // SRAM
            0x6000..=0x7fff => self.mapper.as_ref().unwrap().lock().unwrap().read(addr),
            0x8000..=0xFFFF => self.mapper.as_ref().unwrap().lock().unwrap().read(addr),

            _ => {
                println!("Ignoring mem access at {:04X}", addr);
                0
            }
        }
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
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
                self.joypad2.write(data);
            }
            0x4017 => {
                //ignore for now
            }
            0x4014 => self.dma_transfer(data),
            // SRAM
            0x6000..=0x7fff => {
                self.mapper
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .write(addr, data);
            }
            // PRG-ROM
            0x8000..=0xFFFF => self
                .mapper
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .write(addr, data),
            _ => {
                panic!("Ignoring mem write-access at {:04X}", addr);
            }
        }
    }

    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read(pos);
        let hi = self.mem_read(pos.wrapping_add(1));
        (hi as u16) << 8 | (lo as u16)
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
}

impl CpuBus for Bus {
    fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.poll_nmi_interrupt()
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}
