use std::fs;

use crate::{bus::Bus, cartridge::Rom, cpu::CPU, ppu::frame::Frame, JoypadButton};

pub struct NES {
    pub cpu: CPU,
}

impl NES {
    pub fn new(path: String) -> Self {
        let game_code = fs::read(path).expect("Should have been able to read the game");
        let rom = Rom::new(&game_code).unwrap();
        let bus = Bus::new(rom);

        let mut cpu = CPU::new(bus);
        cpu.reset();

        Self { cpu }
    }

    pub fn emulate_frame(&mut self) -> Option<&Frame> {
        let cycles = self.cpu.run();

        self.cpu.bus.tick(cycles as u8)
    }

    pub fn button_pressed(&mut self, key: JoypadButton, pressed: bool) {
        self.cpu.bus.joypad1.set_button_pressed_status(key, pressed);
    }
}
