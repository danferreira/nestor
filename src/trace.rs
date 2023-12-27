use crate::{
    cpu::{AddressingMode, CPU},
    opcodes::{Mnemonic, OPCODES_MAP},
};

pub fn trace(cpu: &mut CPU) -> String {
    let program_counter = cpu.program_counter;
    let code = cpu.bus.mem_read(program_counter);
    let opcode = OPCODES_MAP.get(&code).unwrap();

    let mut hex_dump = vec![];
    hex_dump.push(code);

    let (mem_addr, stored_value) = match opcode.mode {
        AddressingMode::Immediate
        | AddressingMode::NoneAddressing
        | AddressingMode::Accumulator => (0, 0),
        _ => {
            let addr = cpu
                .get_address_by_addressing_mode(&opcode.mode, cpu.program_counter.wrapping_add(1));
            (addr, cpu.bus.mem_read(addr))
        }
    };

    let tmp = match opcode.len {
        1 => match opcode.mode {
            AddressingMode::Accumulator => String::from("A "),
            _ => String::from(""),
        },
        2 => {
            let address: u8 = cpu.bus.mem_read(program_counter + 1);
            hex_dump.push(address);

            match opcode.mode {
                AddressingMode::Immediate => format!("#${:02X}", address),
                AddressingMode::ZeroPage => format!("${:02X} = {:02X}", mem_addr, stored_value),
                AddressingMode::ZeroPageX => {
                    format!(
                        "${:02X},X @ {:02X} = {:02X}",
                        address, mem_addr, stored_value
                    )
                }
                AddressingMode::ZeroPageY => format!(
                    "${:02X},Y @ {:02X} = {:02X}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::IndirectX => format!(
                    "(${:02X},X) @ {:02X} = {:04X} = {:02X}",
                    address,
                    (address.wrapping_add(cpu.register_x)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                    address,
                    (mem_addr.wrapping_sub(cpu.register_y as u16)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::NoneAddressing => {
                    // assuming local jumps: BNE, BVS, etc....
                    let address: usize =
                        (program_counter as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04X}", address)
                }
                _ => panic!("Wrong mode for 2 bytes {:?}", opcode.mode),
            }
        }
        3 => {
            let address_lo = cpu.bus.mem_read(program_counter.wrapping_add(1));
            let address_hi = cpu.bus.mem_read(program_counter.wrapping_add(2));
            hex_dump.push(address_lo);
            hex_dump.push(address_hi);

            let address = cpu.bus.mem_read_u16(program_counter + 1);

            match opcode.mode {
                AddressingMode::Absolute => match opcode.mnemonic {
                    Mnemonic::JMP | Mnemonic::JSR => {
                        format!("${:04X}", mem_addr)
                    }
                    _ => format!("${:04X} = {:02X}", mem_addr, stored_value),
                },
                AddressingMode::AbsoluteX => format!(
                    "${:04X},X @ {:04X} = {:02X}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::AbsoluteY => format!(
                    "${:04X},Y @ {:04X} = {:02X}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::Indirect => match opcode.mnemonic {
                    Mnemonic::JMP => {
                        format!("(${:04X}) = {:04X}", address, mem_addr)
                    }
                    _ => format!("(${:04X})", address),
                },
                _ => panic!("Wrong mode for 3 bytes {:?}", opcode.mode),
            }
        }
        _ => panic!("Unknown length"),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02X}", z))
        .collect::<Vec<String>>()
        .join(" ");

    let asm_str = format!(
        "{:04X}  {:8} {: >4} {}",
        program_counter, hex_str, opcode.mnemonic_name, tmp
    );

    format!(
        "{:47} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
        asm_str,
        cpu.register_a,
        cpu.register_x,
        cpu.register_y,
        cpu.processor_status,
        cpu.stack_pointer
    )
}
