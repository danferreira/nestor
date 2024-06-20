use crate::mapper::Mapper;

pub struct NROM {
    chr_rom: Vec<u8>,
    prg_rom: Vec<u8>,
}

impl NROM {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        Self { prg_rom, chr_rom }
    }
}

impl Mapper for NROM {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => {
                // CHR-ROM: This should be fine if you're just mirroring the address.
                let len = self.chr_rom.len();
                self.chr_rom[address as usize % len]
            }
            0x8000..=0xFFFF => {
                // PRG-ROM: Ensure mirroring if there's only one bank.
                let bank = if self.prg_rom.len() > 0x4000 {
                    address as usize & 0x7FFF
                } else {
                    address as usize & 0x3FFF
                };
                self.prg_rom[bank]
            }
            _ => 0,
        }
    }

    fn write(&mut self, address: u16, val: u8) {
        match address {
            0x0000..=0x1fff => {
                let len = self.chr_rom.len();
                self.chr_rom[address as usize % len] = val;
            }
            0x6000..=0x7fff => {
                self.prg_rom[address as usize - 0x6000] = val;
            }
            _ => {}
        }
    }
}
