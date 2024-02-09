use crate::cpu::AddressingMode;
use crate::cpu::CPU;
use crate::opcodes;
use crate::opcodes::Mnemonic;
use std::borrow::Borrow;
use std::collections::HashMap;

lazy_static! {
    pub static ref NON_READABLE_ADDR: Vec<u16> =
        vec!(0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x4016, 0x4017);
}

pub fn trace(cpu: &mut CPU) -> String {
    let ref opscodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

    let ref non_readable_addr = *NON_READABLE_ADDR;

    let code = cpu.bus.mem_read(cpu.program_counter);
    let ops = opscodes.get(&code).unwrap();

    let begin = cpu.program_counter;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let (mem_addr, stored_value) = match ops.mode {
        AddressingMode::Immediate
        | AddressingMode::NoneAddressing
        | AddressingMode::Accumulator => (0, 0),
        _ => {
            let (addr, _) = cpu.get_address_by_addressing_mode(&ops.mode, begin + 1);

            if !non_readable_addr.contains(&addr) {
                (addr, cpu.bus.mem_read(addr))
            } else {
                (addr, 0)
            }
        }
    };

    let tmp = match ops.len {
        1 => match ops.code {
            0x0a | 0x4a | 0x2a | 0x6a => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let address: u8 = cpu.bus.mem_read(begin + 1);
            // let value = cpu.mem_read(address));
            hex_dump.push(address);

            match ops.mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPageX => format!(
                    "${:02x},X @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::ZeroPageY => format!(
                    "${:02x},Y @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::IndirectX => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    address,
                    (address.wrapping_add(cpu.register_x)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    address,
                    (mem_addr.wrapping_sub(cpu.register_y as u16)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::NoneAddressing => {
                    // assuming local jumps: BNE, BVS, etc....
                    let address: usize =
                        (begin as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04x}", address)
                }

                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 2. code {:02x}",
                    ops.mode, ops.code
                ),
            }
        }
        3 => {
            let address_lo = cpu.bus.mem_read(begin + 1);
            let address_hi = cpu.bus.mem_read(begin + 2);
            hex_dump.push(address_lo);
            hex_dump.push(address_hi);

            let address = cpu.bus.mem_read_u16(begin + 1);

            match ops.mode {
                AddressingMode::NoneAddressing => {
                    if ops.code == 0x6c {
                        //jmp indirect
                        let jmp_addr = if address & 0x00FF == 0x00FF {
                            let lo = cpu.bus.mem_read(address);
                            let hi = cpu.bus.mem_read(address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.bus.mem_read_u16(address)
                        };

                        // let jmp_addr = cpu.mem_read_u16(address);
                        format!("(${:04x}) = {:04x}", address, jmp_addr)
                    } else {
                        format!("${:04x}", address)
                    }
                }
                AddressingMode::Absolute => match ops.mnemonic {
                    Mnemonic::JMP | Mnemonic::JSR => {
                        format!("${:04X}", address)
                    }
                    _ => format!("${:04x} = {:02x}", mem_addr, stored_value),
                },
                AddressingMode::AbsoluteX => format!(
                    "${:04x},X @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::AbsoluteY => format!(
                    "${:04x},Y @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::Indirect => match ops.mnemonic {
                    Mnemonic::JMP => {
                        format!("(${:04X}) = {:04X}", address, mem_addr)
                    }
                    _ => format!("(${:04X})", address),
                },
                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 3. code {:02x}",
                    ops.mode, ops.code
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!(
        "{:04x}  {:8} {: >4} {}",
        begin, hex_str, ops.mnemonic_name, tmp
    )
    .trim()
    .to_string();

    let ppu = cpu.bus.ppu.borrow();
    format!(
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x} PPU:{: >3},{: >3} CYC:{}",
        asm_str,
        cpu.register_a,
        cpu.register_x,
        cpu.register_y,
        cpu.processor_status,
        cpu.stack_pointer,
        ppu.scanline,
        ppu.cycles,
        cpu.cycles
    )
    .to_ascii_uppercase()
}
