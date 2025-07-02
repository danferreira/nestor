use std::fs;
use std::path::Path;
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

pub struct ROM {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper: Arc<Mutex<Box<dyn Mapper + Send>>>,
    pub mirroring: Mirroring,
}

fn parse_ines_header(raw: &[u8]) -> Result<(usize, usize, Mirroring, u8), String> {
    if raw[0..4] != NES_TAG {
        return Err("File is not in iNES file format".to_string());
    }

    let ines_ver = (raw[7] >> 2) & 0b11;
    if ines_ver != 0 {
        return Err("NES2.0 format is not supported".to_string());
    }

    let prg_rom_size = raw[4] as usize * PRG_ROM_PAGE_SIZE;
    let chr_rom_size = raw[5] as usize * CHR_ROM_PAGE_SIZE;

    let four_screen = raw[6] & 0b1000 != 0;
    let vertical_mirroring = raw[6] & 0b1 != 0;
    let mirroring = match (four_screen, vertical_mirroring) {
        (true, _) => Mirroring::FourScreen,
        (false, true) => Mirroring::Vertical,
        (false, false) => Mirroring::Horizontal,
    };

    let mapper_idx = (raw[7] & 0b1111_0000) | (raw[6] >> 4);

    Ok((prg_rom_size, chr_rom_size, mirroring, mapper_idx))
}

fn create_mapper(mapper_idx: u8, prg_rom: &[u8], chr_rom: &[u8]) -> Result<Arc<Mutex<Box<dyn Mapper + Send>>>, String> {
    let mapper: Mutex<Box<dyn Mapper + Send>> = match mapper_idx {
        0 => Mutex::new(Box::new(NROM::new(prg_rom, chr_rom))),
        3 => Mutex::new(Box::new(CNROM::new(prg_rom, chr_rom))),
        _ => return Err(format!("Mapper not implement yet {mapper_idx}")),
    };

    Ok(Arc::new(mapper))
}

impl ROM {
    pub fn from_bytes(raw: &[u8]) -> Result<ROM, String> {
        let (prg_rom_size, chr_rom_size, mirroring, mapper_idx) = parse_ines_header(raw)?;

        let skip_trainer = raw[6] & 0b100 != 0;

        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };
        let chr_rom_start = prg_rom_start + prg_rom_size;

        let prg_rom = raw[prg_rom_start..(prg_rom_start + prg_rom_size)].to_vec();
        let mut chr_rom = raw[chr_rom_start..(chr_rom_start + chr_rom_size)].to_vec();

        if chr_rom_size == 0 {
            chr_rom = vec![0; 8192];
        }

        let mapper = create_mapper(mapper_idx, &prg_rom, &chr_rom)?;

        Ok(ROM {
            prg_rom,
            chr_rom,
            mapper,
            mirroring,
        })
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<ROM, String> {
        let game_code = fs::read(path).expect("Should have been able to read the game");

        ROM::from_bytes(&game_code)
    }
}
