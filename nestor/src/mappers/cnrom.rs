use crate::mapper::Mapper;

pub struct CNROM {
    chr_rom: Vec<u8>,
    prg_rom: Vec<u8>,
    chr_bank: usize,
}

impl CNROM {
    pub fn new(prg_rom: &[u8], chr_rom: &[u8]) -> Self {
        Self {
            prg_rom: prg_rom.to_vec(),
            chr_rom: chr_rom.to_vec(),
            chr_bank: 0,
        }
    }
}

impl Mapper for CNROM {
    fn read(&self, address: u16) -> u8 {
        match address {
            // CHR-ROM
            0x0000..=0x1fff => {
                let chr_bank_size = 8192;
                let bank_offset = self.chr_bank * chr_bank_size;
                let index = bank_offset | address as usize & 0x1fff;
                self.chr_rom[index]
            }

            // PRG-ROM
            0x8000..=0xffff => {
                let mut bank = address as usize - 0x8000;
                if self.prg_rom.len() == 16384 {
                    bank %= 16384;
                }
                self.prg_rom[bank]
            }

            _ => 0,
        }
    }

    fn write(&mut self, address: u16, val: u8) {
        match address {
            // CHR-ROM
            0x0000..=0x1fff => {
                let chr_bank_size = 8192;
                let bank_offset = self.chr_bank * chr_bank_size;
                let index = bank_offset | address as usize & 0x1fff;
                self.chr_rom[index] = val;
            }

            // PRG-ROM
            0x8000..=0xffff => {
                // CNROM only uses the first 2 bits, but other boards may use
                // the rest, apparently.
                self.chr_bank = (val & 0x03) as usize;
            }
            _ => {}
        }
    }
}
