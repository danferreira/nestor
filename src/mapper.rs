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

//
// CNROM (mapper 3)
//
pub struct Mapper3 {
    chr_rom: Vec<u8>,
    prg_rom: Vec<u8>,
    chr_bank: usize,
}

impl Mapper3 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        Self {
            prg_rom,
            chr_rom,
            chr_bank: 0,
        }
    }
}

impl Mapper for Mapper3 {
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
                if self.prg_rom.len() == 16384 {
                    self.prg_rom[address as usize - 0x8000 - 16384]
                } else {
                    self.prg_rom[address as usize - 0x8000]
                }
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
