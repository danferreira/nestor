use std::{fs, path::PathBuf};

use crate::{bus::Bus, cartridge::Rom, cpu::CPU, ppu::frame::Frame, JoypadButton};

pub struct NES {
    pub cpu: CPU,
    pub is_running: bool,
}

impl NES {
    pub fn new() -> Self {
        let bus = Bus::new();
        let cpu = CPU::new(bus);

        Self {
            cpu,
            is_running: false,
        }
    }

    pub fn emulate_frame(&mut self) -> Option<&Frame> {
        let cycles = self.cpu.run();

        self.cpu.bus.tick(cycles as u8)
    }

    pub fn button_pressed(&mut self, key: JoypadButton, pressed: bool) {
        self.cpu.bus.joypad1.set_button_pressed_status(key, pressed);
    }

    pub fn load_rom(&mut self, path: PathBuf) {
        let game_code = fs::read(path).expect("Should have been able to read the game");
        let rom = Rom::new(&game_code).unwrap();
        self.cpu.bus.load_rom(rom);
    }

    pub fn start_emulation(&mut self) {
        self.cpu.reset();
        self.is_running = true;
    }

    pub fn pause_emulation(&mut self) {
        self.is_running = false;
    }
    pub fn continue_emulation(&mut self) {
        self.is_running = true;
    }
}
