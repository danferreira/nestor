use std::sync::{Arc, Mutex};

use crate::mapper::Mapper;
use crate::mappers::{CNROM, NROM};

const NES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const PRG_ROM_PAGE_SIZE: usize = 16384;
const CHR_ROM_PAGE_SIZE: usize = 8192;

#[derive(Clone, Debug, PartialEq)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
    None,
}

pub struct Rom {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper: Arc<Mutex<Box<dyn Mapper + Send>>>,
    pub mirroring: Mirroring,
}

impl Rom {
    pub fn new(raw: &[u8]) -> Result<Rom, String> {
        if raw[0..4] != NES_TAG {
            return Err("File is not in iNES file format".to_string());
        }

        let ines_ver = (raw[7] >> 2) & 0b11;
        if ines_ver != 0 {
            return Err("NES2.0 format is not supported".to_string());
        }

        let prg_rom_size = raw[4] as usize * PRG_ROM_PAGE_SIZE;
        let chr_rom_size = raw[5] as usize * CHR_ROM_PAGE_SIZE;

        // ROM Control Byte 1:
        // • Bit 0 - Indicates the type of mirroring used by the game
        // where 0 indicates horizontal mirroring, 1 indicates
        // vertical mirroring.
        // • Bit 1 - Indicates the presence of battery-backed RAM at
        // memory locations $6000-$7FFF.
        // • Bit 2 - Indicates the presence of a 512-byte trainer at
        // memory locations $7000-$71FF.
        // • Bit 3 - If this bit is set it overrides bit 0 to indicate fourscreen mirroring should be used.
        // • Bits 4-7 - Four lower bits of the mapper number.
        let skip_trainer = raw[6] & 0b100 != 0;

        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };
        let chr_rom_start = prg_rom_start + prg_rom_size;

        let prg_rom = raw[prg_rom_start..(prg_rom_start + prg_rom_size)].to_vec();
        let mut chr_rom = raw[chr_rom_start..(chr_rom_start + chr_rom_size)].to_vec();

        if chr_rom_size == 0 {
            chr_rom = vec![0; 8192];
        }

        let four_screen = raw[6] & 0b1000 != 0;
        let vertical_mirroring = raw[6] & 0b1 != 0;
        let mirroring = match (four_screen, vertical_mirroring) {
            (true, _) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        };

        let mapper_idx = (raw[7] & 0b1111_0000) | (raw[6] >> 4);

        let mapper: Mutex<Box<dyn Mapper + Send>> = match mapper_idx {
            0 => Mutex::new(Box::new(NROM::new(&prg_rom, &chr_rom))),
            3 => Mutex::new(Box::new(CNROM::new(&prg_rom, &chr_rom))),
            _ => panic!("Mapper not implement yet {mapper_idx}"),
        };

        Ok(Rom {
            prg_rom,
            chr_rom,
            mapper: Arc::new(mapper),
            mirroring,
        })
    }
}
