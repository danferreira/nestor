use core::panic;

pub trait Mapper {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

//
// NROM (mapper 0)
//
pub struct Mapper0 {
    chr_rom: Vec<u8>,
    prg_rom: Vec<u8>,
}

impl Mapper0 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        Self { prg_rom, chr_rom }
    }
}

impl Mapper for Mapper0 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1fff => {
                let len = self.chr_rom.len();
                self.chr_rom[address as usize % len]
            }
            0x6000..=0x7fff => self.prg_rom[address as usize - 0x6000],
            0x8000..=0xffff => self.prg_rom[address as usize % self.prg_rom.len()],
            _ => 0,
        }
    }

    fn write(&mut self, _address: u16, _val: u8) {
        // Nothing
        panic!("Writting not available for NROM");
    }
}
