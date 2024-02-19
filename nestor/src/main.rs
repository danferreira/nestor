pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod joypad;
pub mod mapper;
pub mod opcodes;
pub mod ppu;
pub mod trace;

use std::fs;

use crate::ppu::frame::Frame;

use bus::Bus;
use cartridge::{Mirroring, Rom};
use cpu::CPU;
use joypad::JoypadButton;
use ppu::{palette, PPU};

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

fn main() {
    let path = std::env::args().nth(1).expect("no path given");
}
