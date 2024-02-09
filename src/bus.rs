use std::{cell::RefCell, rc::Rc};

use crate::{cartridge::Rom, joypad::Joypad, ppu::NesPPU};

pub struct Bus<'call> {
    cpu_vram: [u8; 2048],
    rom: Rc<RefCell<Rom>>,
    pub ppu: NesPPU,
    joypad1: Joypad,
    gameloop_callback: Box<dyn FnMut(&NesPPU, &mut Joypad) + 'call>,
}

impl<'call> Bus<'call> {
    pub fn new<F>(rom: Rom, gameloop_callback: F) -> Bus<'call>
    where
        F: FnMut(&NesPPU, &mut Joypad) + 'call,
    {
        let rom_rc = Rc::new(RefCell::new(rom));
        let ppu = NesPPU::new(Rc::clone(&rom_rc));

        Bus {
            cpu_vram: [0; 2048],
            rom: rom_rc,
            ppu,
            joypad1: Joypad::new(),
            gameloop_callback: Box::from(gameloop_callback),
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        let mut frame_complete = false;

        for _ in 0..(cycles * 3) {
            if self.ppu.tick() {
                frame_complete = true;
                break;
            }
        }

        if frame_complete {
            (self.gameloop_callback)(&self.ppu, &mut self.joypad1);
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
            0x6000..=0x7fff => self.rom.borrow_mut().mapper.read(addr),
            // 0x8000..=0xFFFF => self.read_prg_rom(addr),
            0x8000..=0xFFFF => self.rom.borrow_mut().mapper.read(addr),

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
            0x4014 => self.ppu.cpu_write(addr, data),
            // SRAM
            0x6000..=0x7fff => self.rom.borrow_mut().mapper.write(addr, data),
            // PRG-ROM
            0x8000..=0xFFFF => self.rom.borrow_mut().mapper.write(addr, data),
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
