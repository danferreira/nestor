use std::fmt::Debug;

use crate::{
    bus::{CpuBus, Memory},
    opcodes::{Mnemonic, OpCode, OPCODES_MAP},
};

const CARRY_FLAG: u8 = 1 << 0;
const ZERO_FLAG: u8 = 1 << 1;
const IRQ_FLAG: u8 = 1 << 2;
const DECIMAL_FLAG: u8 = 1 << 3;
const BREAK_FLAG: u8 = 1 << 4;
const OVERFLOW_FLAG: u8 = 1 << 6;
const NEGATIVE_FLAG: u8 = 1 << 7;

// const BRK_VECTOR: u16 = 0xfffe;

const STACK_RESET: u8 = 0xFD;

pub struct CPU<B: Memory + CpuBus> {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub processor_status: u8,
    pub stack_pointer: u8,
    pub program_counter: u16,
    pub bus: B,
    pub cycles: u64,
}

#[derive(Debug)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    Relative,
    NoneAddressing,
}

fn page_cross(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xFF00 != addr2 & 0xFF00
}

impl<B: Memory + CpuBus> CPU<B> {
    pub fn new(bus: B) -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            processor_status: 0x24,
            stack_pointer: STACK_RESET,
            program_counter: 0,
            cycles: 0,
            bus,
        }
    }

    pub fn get_operand_address(&mut self, mode: &AddressingMode) -> (u16, bool) {
        self.get_address_by_addressing_mode(mode, self.program_counter)
    }

    pub fn get_address_by_addressing_mode(
        &mut self,
        mode: &AddressingMode,
        address: u16,
    ) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate | AddressingMode::Implied => (address, false),

            AddressingMode::ZeroPage => (self.bus.mem_read(address) as u16, false),

            AddressingMode::Absolute => (self.bus.mem_read_u16(address), false),

            AddressingMode::ZeroPageX => {
                let base = self.bus.mem_read(address);
                let addr = base.wrapping_add(self.register_x) as u16;
                (addr, false)
            }
            AddressingMode::ZeroPageY => {
                let base = self.bus.mem_read(address);
                let addr = base.wrapping_add(self.register_y) as u16;
                (addr, false)
            }

            AddressingMode::AbsoluteX => {
                let base = self.bus.mem_read_u16(address);
                let addr = base.wrapping_add(self.register_x as u16);
                (addr, page_cross(base, addr))
            }
            AddressingMode::AbsoluteY => {
                let base = self.bus.mem_read_u16(address);
                let addr = base.wrapping_add(self.register_y as u16);

                (addr, page_cross(base, addr))
            }

            AddressingMode::Indirect => {
                let indirect_address = self.bus.mem_read_u16(address);

                if indirect_address & 0x00FF == 0x00FF {
                    let lo = self.bus.mem_read(indirect_address);
                    let hi = self.bus.mem_read(indirect_address & 0xFF00);
                    ((hi as u16) << 8 | (lo as u16), false)
                } else {
                    (self.bus.mem_read_u16(indirect_address), false)
                }
            }
            AddressingMode::IndirectX => {
                let base = self.bus.mem_read(address);
                let ptr = base.wrapping_add(self.register_x);

                let lo = self.bus.mem_read(ptr as u16);
                let hi = self.bus.mem_read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8 | (lo as u16), false)

                // self.bus.mem_read_u16(ptr as u16)
            }
            AddressingMode::IndirectY => {
                let base = self.bus.mem_read(address);
                // let ptr = self.bus.mem_read_u16(base as u16);
                // ptr.wrapping_add(self.register_y as u16)

                let lo = self.bus.mem_read(base as u16);
                let hi = self.bus.mem_read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                (deref, page_cross(deref, deref_base))
            }

            _ => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    fn pop_stack(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);

        self.bus.mem_read((self.stack_pointer as u16) + 0x0100)
    }

    fn pop_stack16(&mut self) -> u16 {
        let lsb = self.pop_stack() as u16;
        let msb = self.pop_stack() as u16;

        (msb << 8) | lsb
    }

    fn push_stack(&mut self, v: u8) {
        let addr = (self.stack_pointer as u16) + 0x0100;
        self.bus.mem_write(addr, v);

        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn push_stack16(&mut self, v: u16) {
        self.push_stack(((v >> 8) & 0xFF) as u8);
        self.push_stack((v & 0xFF) as u8);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let mut result = self.register_a as u16 + value as u16;

        if self.get_flag(CARRY_FLAG) {
            result += 1;
        }

        self.set_flag(CARRY_FLAG, result > 255);

        let result = result as u8;

        self.set_flag(
            OVERFLOW_FLAG,
            ((self.register_a ^ result) & (value ^ result) & 0x80) == 0x80,
        );
        self.set_zero_and_negative_flags(result);

        self.register_a = result;

        if page_cross {
            self.cycles += 1;
        }
    }

    fn and(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);
        self.register_a &= value;

        self.set_zero_and_negative_flags(self.register_a);

        if page_cross {
            self.cycles += 1;
        }
    }

    fn asl(&mut self, mode: &AddressingMode) {
        match mode {
            AddressingMode::Accumulator => {
                self.set_flag(CARRY_FLAG, (self.register_a & 0x80) != 0);

                self.register_a <<= 1;

                self.set_zero_and_negative_flags(self.register_a);
            }
            _ => {
                let (addr, _) = self.get_operand_address(mode);
                let value = self.bus.mem_read(addr);
                let result = value << 1;

                self.bus.mem_write(addr, result);

                self.set_flag(CARRY_FLAG, (value & 0x80) != 0);
                self.set_zero_and_negative_flags(result);
            }
        }
    }

    fn branch(&mut self, opcode: &OpCode) {
        let offset = self.bus.mem_read(self.program_counter) as i8;
        let jump_addr = self
            .program_counter
            .wrapping_add(1)
            .wrapping_add(offset as u16);

        let next_instruction = self.program_counter.wrapping_add((opcode.len - 1) as u16);

        self.cycles += 1;
        if page_cross(next_instruction, jump_addr) {
            self.cycles += 1;
        }

        self.program_counter = jump_addr;
    }

    fn bcc(&mut self, opcode: &OpCode) {
        if !self.get_flag(CARRY_FLAG) {
            self.branch(opcode)
        }
    }

    fn bcs(&mut self, opcode: &OpCode) {
        if self.get_flag(CARRY_FLAG) {
            self.branch(opcode)
        }
    }

    fn beq(&mut self, opcode: &OpCode) {
        if self.get_flag(ZERO_FLAG) {
            self.branch(opcode)
        }
    }

    fn bne(&mut self, opcode: &OpCode) {
        if !self.get_flag(ZERO_FLAG) {
            self.branch(opcode)
        }
    }

    fn bmi(&mut self, opcode: &OpCode) {
        if self.get_flag(NEGATIVE_FLAG) {
            self.branch(opcode)
        }
    }

    fn bpl(&mut self, opcode: &OpCode) {
        if !self.get_flag(NEGATIVE_FLAG) {
            self.branch(opcode)
        }
    }

    fn bvc(&mut self, opcode: &OpCode) {
        if !self.get_flag(OVERFLOW_FLAG) {
            self.branch(opcode)
        }
    }

    fn bvs(&mut self, opcode: &OpCode) {
        if self.get_flag(OVERFLOW_FLAG) {
            self.branch(opcode)
        }
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        self.set_flag(ZERO_FLAG, (self.register_a & value) == 0);
        self.set_flag(NEGATIVE_FLAG, (value & 0x80) != 0);
        self.set_flag(OVERFLOW_FLAG, (value & 0x40) != 0);
    }

    fn brk(&mut self) {
        // TODO: Dummy reads
        self.bus.mem_read(self.program_counter);
        self.push_stack16(self.program_counter.wrapping_add(1));

        let status = self.processor_status | 0x10;
        self.push_stack(status);

        self.set_flag(IRQ_FLAG, true);

        self.program_counter = self.bus.mem_read_u16(0xFFFE);
    }

    fn clc(&mut self) {
        self.set_flag(CARRY_FLAG, false);
    }

    fn cld(&mut self) {
        self.set_flag(DECIMAL_FLAG, false);
    }

    fn cli(&mut self) {
        self.set_flag(IRQ_FLAG, false);
    }

    fn clv(&mut self) {
        self.set_flag(OVERFLOW_FLAG, false);
    }

    fn cmp_register(&mut self, reg: u8, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);
        let result = reg.wrapping_sub(value);

        self.set_flag(CARRY_FLAG, reg >= value);
        self.set_flag(ZERO_FLAG, reg == value);
        self.set_flag(NEGATIVE_FLAG, (result & 0x80) != 0);

        if page_cross {
            self.cycles += 1;
        }
    }

    fn cmp(&mut self, mode: &AddressingMode) {
        self.cmp_register(self.register_a, mode);
    }

    fn cpx(&mut self, mode: &AddressingMode) {
        self.cmp_register(self.register_x, mode);
    }

    fn cpy(&mut self, mode: &AddressingMode) {
        self.cmp_register(self.register_y, mode);
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);
        let result = value.wrapping_sub(1);

        self.bus.mem_write(addr, result);

        self.set_zero_and_negative_flags(result);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);

        self.set_zero_and_negative_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);

        self.set_zero_and_negative_flags(self.register_y);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        self.register_a ^= value;

        self.set_zero_and_negative_flags(self.register_a);

        if page_cross {
            self.cycles += 1;
        }
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);
        let result = value.wrapping_add(1);

        self.bus.mem_write(addr, result);

        self.set_zero_and_negative_flags(result);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.set_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.set_zero_and_negative_flags(self.register_y);
    }

    fn jmp(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.program_counter = addr;
    }

    fn jsr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.push_stack16(self.program_counter + 2 - 1);

        self.program_counter = addr;
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        self.set_zero_and_negative_flags(value);

        self.register_a = value;

        if page_cross {
            self.cycles += 1;
        }
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        self.set_zero_and_negative_flags(value);

        self.register_x = value;

        if page_cross {
            self.cycles += 1;
        }
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        self.set_zero_and_negative_flags(value);

        self.register_y = value;

        if page_cross {
            self.cycles += 1;
        }
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        match mode {
            AddressingMode::Accumulator => {
                self.set_flag(CARRY_FLAG, (self.register_a & 1) == 1);

                self.register_a >>= 1;

                self.set_zero_and_negative_flags(self.register_a);
            }
            _ => {
                let (addr, _) = self.get_operand_address(mode);
                let value = self.bus.mem_read(addr);
                let result = value >> 1;

                self.bus.mem_write(addr, result);

                self.set_flag(CARRY_FLAG, (value & 1) == 1);
                self.set_zero_and_negative_flags(result);
            }
        }
    }

    fn nop(&mut self, mode: &AddressingMode) {
        let (_addr, page_cross) = self.get_operand_address(mode);

        if page_cross {
            self.cycles += 1;
        }
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        self.register_a |= value;

        self.set_zero_and_negative_flags(self.register_a);

        if page_cross {
            self.cycles += 1;
        }
    }

    fn pha(&mut self) {
        self.push_stack(self.register_a)
    }

    fn php(&mut self) {
        self.push_stack(self.processor_status | BREAK_FLAG);
    }

    fn pla(&mut self) {
        self.register_a = self.pop_stack();

        self.set_zero_and_negative_flags(self.register_a);
    }

    fn plp(&mut self) {
        let value = self.pop_stack();
        self.set_flags(value);
    }

    fn rol(&mut self, mode: &AddressingMode) {
        match mode {
            AddressingMode::Accumulator => {
                let current_carry_flag = self.get_flag(CARRY_FLAG) as u8;
                let new_carry_flag = ((self.register_a >> 7) & 1) == 1;

                self.register_a = (self.register_a << 1) | current_carry_flag;

                self.set_flag(CARRY_FLAG, new_carry_flag);
                self.set_zero_and_negative_flags(self.register_a);
            }
            _ => {
                let (addr, _) = self.get_operand_address(mode);
                let value = self.bus.mem_read(addr);

                let current_carry_flag = self.get_flag(CARRY_FLAG) as u8;
                let new_carry_flag = ((value >> 7) & 1) == 1;

                let result = (value << 1) | current_carry_flag;

                self.bus.mem_write(addr, result);

                self.set_flag(CARRY_FLAG, new_carry_flag);
                self.set_zero_and_negative_flags(result);
            }
        }
    }

    fn ror(&mut self, mode: &AddressingMode) {
        match mode {
            AddressingMode::Accumulator => {
                let current_carry_flag = self.get_flag(CARRY_FLAG);
                let new_carry_flag = (self.register_a & 1) == 1;

                self.register_a >>= 1;

                if current_carry_flag {
                    self.register_a |= 0x80;
                }

                self.set_flag(CARRY_FLAG, new_carry_flag);
                self.set_zero_and_negative_flags(self.register_a);
            }
            _ => {
                let (addr, _) = self.get_operand_address(mode);
                let value = self.bus.mem_read(addr);

                let current_carry_flag = self.get_flag(CARRY_FLAG);
                let new_carry_flag = (value & 1) == 1;

                let mut result = value >> 1;

                if current_carry_flag {
                    result |= 0x80;
                }

                self.bus.mem_write(addr, result);

                self.set_flag(CARRY_FLAG, new_carry_flag);
                self.set_zero_and_negative_flags(result);
            }
        }
    }

    fn rti(&mut self) {
        //TODO: dummy reads
        self.bus.mem_read(self.program_counter);
        let value = self.pop_stack();
        self.set_flags(value);

        self.program_counter = self.pop_stack16();
    }

    fn rts(&mut self) {
        self.bus.mem_read(self.program_counter);
        self.program_counter = self.pop_stack16().wrapping_add(1);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let accumulator = self.register_a;

        let carry_flag = self.get_flag(CARRY_FLAG) as u8;

        let (v1, o1) = accumulator.overflowing_sub(value);
        let (result, o2) = v1.overflowing_sub(1 - carry_flag);

        self.set_flag(CARRY_FLAG, !(o1 | o2));

        let overflow = ((accumulator ^ result) & 0x80) != 0 && ((accumulator ^ value) & 0x80) != 0;

        self.set_flag(OVERFLOW_FLAG, overflow);

        self.register_a = result;

        self.set_zero_and_negative_flags(self.register_a);

        if page_cross {
            self.cycles += 1;
        }
    }

    fn sec(&mut self) {
        self.set_flag(CARRY_FLAG, true);
    }

    fn sed(&mut self) {
        self.set_flag(DECIMAL_FLAG, true);
    }

    fn sei(&mut self) {
        self.set_flag(IRQ_FLAG, true);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.bus.mem_write(addr, self.register_a);
    }

    fn stx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.bus.mem_write(addr, self.register_x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.bus.mem_write(addr, self.register_y);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.set_zero_and_negative_flags(self.register_x);
    }

    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.set_zero_and_negative_flags(self.register_y);
    }

    fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
        self.set_zero_and_negative_flags(self.register_x);
    }

    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.set_zero_and_negative_flags(self.register_a);
    }

    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.set_zero_and_negative_flags(self.register_a);
    }

    // Unoficial
    pub fn alr(&mut self, mode: &AddressingMode) {
        // and + lsr
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let and_result = self.register_a & value;
        let shift_result = and_result >> 1;

        // Update flags
        self.set_flag(CARRY_FLAG, and_result & 0x01 != 0); // Carry is old bit 0
        self.set_zero_and_negative_flags(shift_result);

        // Store the result back in the accumulator
        self.register_a = shift_result;
    }

    pub fn anc(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        self.register_a &= value;
        self.set_flag(CARRY_FLAG, (self.register_a & 0x80) != 0);
        self.set_zero_and_negative_flags(self.register_a);
    }

    pub fn arr(&mut self, mode: &AddressingMode) {
        // and + ror
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let carry = self.get_flag(CARRY_FLAG) as u8;

        let and_result = self.register_a & value;
        let result = (and_result >> 1) | (carry << 7);

        // Store the result back in the accumulator
        self.register_a = result;

        // Update flags
        let bit_5 = (result >> 5) & 1;
        let bit_6 = (result >> 6) & 1;

        self.set_flag(CARRY_FLAG, bit_6 == 1);
        self.set_flag(OVERFLOW_FLAG, bit_5 ^ bit_6 == 1);
        self.set_zero_and_negative_flags(result);
    }

    pub fn axs(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let (result, overflow) = (self.register_a & self.register_x).overflowing_sub(value);

        self.register_x = result;

        self.set_flag(CARRY_FLAG, !overflow);
        self.set_zero_and_negative_flags(self.register_x);
    }

    fn las(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let result = self.stack_pointer & value;
        self.register_a = result;
        self.register_x = result;
        self.stack_pointer = result;

        self.set_zero_and_negative_flags(result);

        if page_cross {
            self.cycles += 1;
        }
    }

    fn lax(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        self.register_a = value;
        self.register_x = value;

        self.set_zero_and_negative_flags(value);

        if page_cross {
            self.cycles += 1;
        }
    }

    fn sax(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.bus.mem_write(addr, self.register_a & self.register_x);
    }

    fn dcp(&mut self, mode: &AddressingMode) {
        // dec
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let decremented_value = value.wrapping_sub(1);

        self.bus.mem_write(addr, decremented_value);

        // cmp
        self.set_flag(CARRY_FLAG, self.register_a >= decremented_value);
        self.set_zero_and_negative_flags(self.register_a.wrapping_sub(decremented_value));
    }

    fn isb(&mut self, mode: &AddressingMode) {
        // inc
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let incremented_value = value.wrapping_add(1);

        self.bus.mem_write(addr, incremented_value);

        // sbc
        let accumulator = self.register_a;

        let carry_flag = self.get_flag(CARRY_FLAG) as u8;

        let (v1, o1) = accumulator.overflowing_sub(incremented_value);
        let (result, o2) = v1.overflowing_sub(1 - carry_flag);

        self.set_flag(CARRY_FLAG, !(o1 | o2));

        let overflow =
            ((accumulator ^ result) & 0x80) != 0 && ((accumulator ^ incremented_value) & 0x80) != 0;

        self.set_flag(OVERFLOW_FLAG, overflow);

        self.register_a = result;

        self.set_zero_and_negative_flags(self.register_a);
    }

    fn slo(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);
        let result = value << 1;

        self.bus.mem_write(addr, result);

        self.set_flag(CARRY_FLAG, (value & 0x80) != 0);
        self.set_zero_and_negative_flags(result);

        self.register_a |= result;

        self.set_zero_and_negative_flags(self.register_a);
    }

    fn rla(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let current_carry_flag = self.get_flag(CARRY_FLAG) as u8;
        let new_carry_flag = ((value >> 7) & 1) == 1;

        let result = (value << 1) | current_carry_flag;

        self.bus.mem_write(addr, result);

        self.set_flag(CARRY_FLAG, new_carry_flag);
        self.set_zero_and_negative_flags(result);

        self.register_a &= result;

        self.set_zero_and_negative_flags(self.register_a);
    }

    fn shx(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);

        let hi = (addr >> 8) as u8;

        let result = self.register_x & hi.wrapping_add(!page_cross as u8);
        let high = if page_cross { result } else { hi };

        self.bus
            .mem_write(addr & 0x00FF | (high as u16) << 8, result);
    }

    fn shy(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);

        let hi = (addr >> 8) as u8;

        let result = self.register_y & hi.wrapping_add(!page_cross as u8);
        let high = if page_cross { result } else { hi };

        self.bus
            .mem_write(addr & 0x00FF | (high as u16) << 8, result);
    }

    fn sre(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let result = value >> 1;

        self.bus.mem_write(addr, result);

        self.set_flag(CARRY_FLAG, (value & 1) == 1);
        self.set_zero_and_negative_flags(result);

        self.register_a ^= result;

        self.set_zero_and_negative_flags(self.register_a);
    }

    fn rra(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let current_carry_flag = self.get_flag(CARRY_FLAG);
        let new_carry_flag = (value & 1) == 1;

        let mut rotate_result = value >> 1;

        if current_carry_flag {
            rotate_result |= 0x80;
        }

        self.bus.mem_write(addr, rotate_result);

        self.set_flag(CARRY_FLAG, new_carry_flag);
        self.set_zero_and_negative_flags(rotate_result);

        let mut result = self.register_a as u16 + rotate_result as u16;

        if self.get_flag(CARRY_FLAG) {
            result += 1;
        }

        self.set_flag(CARRY_FLAG, result > 255);

        let result = result as u8;

        self.set_flag(
            OVERFLOW_FLAG,
            ((self.register_a ^ result) & (rotate_result ^ result) & 0x80) == 0x80,
        );
        self.set_zero_and_negative_flags(result);

        self.register_a = result;
    }

    fn get_flag(&self, flag: u8) -> bool {
        (self.processor_status & flag) != 0
    }

    fn set_flag(&mut self, flag: u8, on: bool) {
        if on {
            self.processor_status |= flag;
        } else {
            self.processor_status &= !flag;
        }
    }

    fn set_flags(&mut self, value: u8) {
        // This make sure that the bit 5 is not set
        self.processor_status = (value | 0x30) - 0x10;
    }

    fn set_zero_and_negative_flags(&mut self, value: u8) {
        self.set_flag(ZERO_FLAG, value == 0);
        self.set_flag(NEGATIVE_FLAG, (value & 0x80) != 0);
    }

    fn interrupt_nmi(&mut self) {
        self.push_stack16(self.program_counter);
        self.php();

        self.cycles += 7;
        self.set_flag(IRQ_FLAG, true);

        self.program_counter = self.bus.mem_read_u16(0xFFFA);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.processor_status = 0x24;
        self.stack_pointer = STACK_RESET;
        // self.cycles = 7;
        // self.bus.tick(7);

        self.program_counter = self.bus.mem_read_u16(0xFFFC);
    }

    pub fn run(&mut self) -> u8 {
        if self.bus.poll_nmi_status().is_some() {
            self.interrupt_nmi();
        }

        let start_cycles = self.cycles;

        let code = self.bus.mem_read(self.program_counter);
        self.program_counter = self.program_counter.wrapping_add(1);
        let program_counter_state = self.program_counter;

        let opcode = OPCODES_MAP
            .get(&code)
            .unwrap_or_else(|| panic!("OpCode {:x} is not recognized", code));

        match opcode.mnemonic {
            Mnemonic::ADC => self.adc(&opcode.mode),
            Mnemonic::AND => self.and(&opcode.mode),
            Mnemonic::ASL => self.asl(&opcode.mode),
            Mnemonic::BCC => self.bcc(opcode),
            Mnemonic::BCS => self.bcs(opcode),
            Mnemonic::BEQ => self.beq(opcode),
            Mnemonic::BNE => self.bne(opcode),
            Mnemonic::BMI => self.bmi(opcode),
            Mnemonic::BPL => self.bpl(opcode),
            Mnemonic::BVC => self.bvc(opcode),
            Mnemonic::BVS => self.bvs(opcode),
            Mnemonic::BIT => self.bit(&opcode.mode),
            Mnemonic::BRK => self.brk(),
            Mnemonic::CLC => self.clc(),
            Mnemonic::CLD => self.cld(),
            Mnemonic::CLI => self.cli(),
            Mnemonic::CLV => self.clv(),
            Mnemonic::CMP => self.cmp(&opcode.mode),
            Mnemonic::CPX => self.cpx(&opcode.mode),
            Mnemonic::CPY => self.cpy(&opcode.mode),
            Mnemonic::DEC => self.dec(&opcode.mode),
            Mnemonic::DEX => self.dex(),
            Mnemonic::DEY => self.dey(),
            Mnemonic::EOR => self.eor(&opcode.mode),
            Mnemonic::INC => self.inc(&opcode.mode),
            Mnemonic::INX => self.inx(),
            Mnemonic::INY => self.iny(),
            Mnemonic::JMP => self.jmp(&opcode.mode),
            Mnemonic::JSR => self.jsr(&opcode.mode),
            Mnemonic::LDA => self.lda(&opcode.mode),
            Mnemonic::LDX => self.ldx(&opcode.mode),
            Mnemonic::LDY => self.ldy(&opcode.mode),
            Mnemonic::LSR => self.lsr(&opcode.mode),
            Mnemonic::NOP => self.nop(&opcode.mode),
            Mnemonic::ORA => self.ora(&opcode.mode),
            Mnemonic::PHA => self.pha(),
            Mnemonic::PHP => self.php(),
            Mnemonic::PLA => self.pla(),
            Mnemonic::PLP => self.plp(),
            Mnemonic::ROL => self.rol(&opcode.mode),
            Mnemonic::ROR => self.ror(&opcode.mode),
            Mnemonic::RTI => self.rti(),
            Mnemonic::RTS => self.rts(),
            Mnemonic::SBC => self.sbc(&opcode.mode),
            Mnemonic::SEC => self.sec(),
            Mnemonic::SED => self.sed(),
            Mnemonic::SEI => self.sei(),
            Mnemonic::STA => self.sta(&opcode.mode),
            Mnemonic::STX => self.stx(&opcode.mode),
            Mnemonic::STY => self.sty(&opcode.mode),
            Mnemonic::TAX => self.tax(),
            Mnemonic::TAY => self.tay(),
            Mnemonic::TSX => self.tsx(),
            Mnemonic::TXA => self.txa(),
            Mnemonic::TXS => self.txs(),
            Mnemonic::TYA => self.tya(),
            // Unoficial
            Mnemonic::ALR => self.alr(&opcode.mode),
            Mnemonic::ARR => self.arr(&opcode.mode),
            Mnemonic::ANC => self.anc(&opcode.mode),
            Mnemonic::AXS => self.axs(&opcode.mode),
            Mnemonic::LAS => self.las(&opcode.mode),
            Mnemonic::LAX => self.lax(&opcode.mode),
            Mnemonic::SAX => self.sax(&opcode.mode),
            Mnemonic::DCP => self.dcp(&opcode.mode),
            Mnemonic::ISB => self.isb(&opcode.mode),
            Mnemonic::SLO => self.slo(&opcode.mode),
            Mnemonic::RLA => self.rla(&opcode.mode),
            Mnemonic::SHX => self.shx(&opcode.mode),
            Mnemonic::SHY => self.shy(&opcode.mode),
            Mnemonic::SRE => self.sre(&opcode.mode),
            Mnemonic::RRA => self.rra(&opcode.mode),
            _ => todo!("{:?}", opcode.mnemonic),
        }

        if program_counter_state == self.program_counter {
            self.program_counter = self.program_counter.wrapping_add((opcode.len - 1) as u16);
        }

        self.cycles += opcode.cycles as u64;

        (self.cycles - start_cycles) as u8
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    struct MockBus {
        memory: [u8; 0x10000],
    }

    impl MockBus {
        pub fn new() -> Self {
            let mut bus = Self {
                memory: [0; 0x10000],
            };

            bus.mem_write_u16(0xFFFC, 0x8000);

            bus
        }

        pub fn load(&mut self, data: &[u8]) {
            self.memory[0x8000..(0x8000 + data.len())].copy_from_slice(data);
        }
    }

    impl Memory for MockBus {
        fn mem_read(&mut self, addr: u16) -> u8 {
            self.memory[addr as usize]
        }

        fn mem_write(&mut self, addr: u16, data: u8) {
            self.memory[addr as usize] = data;
        }
    }

    impl CpuBus for MockBus {
        fn poll_nmi_status(&mut self) -> Option<u8> {
            None
        }
    }

    #[test]
    fn test_adc_carry_zero_immediate() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x69, 0x38]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0xc8;

        cpu.run();

        assert_eq!(cpu.register_a, 0);
        assert!(cpu.get_flag(CARRY_FLAG));
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_and_immediate() {
        let mut mock_bus = MockBus::new();

        mock_bus.load(&[0x29, 0x12]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x01;

        cpu.run();

        assert_eq!(cpu.register_a, 0x0);
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_asl_immediate() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x0A]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0b11000001;

        cpu.run();

        assert_eq!(cpu.register_a, 0b10000010);
        assert!(cpu.get_flag(CARRY_FLAG));
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_bit_zero_flag() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x24, 0x10]);
        mock_bus.mem_write(0x0010, 0xFF);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x00;

        cpu.run();

        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(cpu.get_flag(NEGATIVE_FLAG));
        assert!(cpu.get_flag(OVERFLOW_FLAG));
    }

    #[test]
    fn test_brk() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x00]); // BRK
        mock_bus.mem_write_u16(0xFFFE, 0x9000); // Interrupt vector

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run();

        assert_eq!(cpu.program_counter, 0x9000);
    }

    #[test]
    fn test_clc() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x18]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.set_flag(CARRY_FLAG, true);

        cpu.run();

        assert!(!cpu.get_flag(CARRY_FLAG));
    }

    #[test]
    fn test_cld() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xD8]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.set_flag(DECIMAL_FLAG, true);

        cpu.run();

        assert!(!cpu.get_flag(DECIMAL_FLAG));
    }

    #[test]
    fn test_cli() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x58]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.set_flag(IRQ_FLAG, true);

        cpu.run();

        assert!(!cpu.get_flag(IRQ_FLAG));
    }

    #[test]
    fn test_clv() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xB8]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.set_flag(OVERFLOW_FLAG, true);

        cpu.run();

        assert!(!cpu.get_flag(OVERFLOW_FLAG));
    }

    #[test]
    fn test_lda_immediate() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xA9, 0x42]); // LDA #$42

        let mut cpu = CPU::new(mock_bus);
        cpu.program_counter = 0x8000;

        cpu.run();

        assert_eq!(cpu.register_a, 0x42);
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_lda_zero_flag() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xA9, 0x00]); // LDA #$00

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run();

        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_lda_negative_flag() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xA9, 0x80]); // LDA #$80

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run();

        assert_eq!(cpu.register_a, 0x80);
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_sta_absolute() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x8D, 0x00, 0x20]); // STA $2000

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x55;

        cpu.run();

        assert_eq!(cpu.bus.mem_read(0x2000), 0x55);
    }

    #[test]
    fn test_tax() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xAA]); // TAX

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x0F;

        cpu.run();

        assert_eq!(cpu.register_x, 0x0F);
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_inx_overflow() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xE8]); // INX

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_x = 0xFF;

        cpu.run();

        assert_eq!(cpu.register_x, 0x00);
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_jmp_absolute() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x4C, 0x05, 0x80]); // JMP $8005

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run();

        assert_eq!(cpu.program_counter, 0x8005);
    }

    #[test]
    fn test_jsr_rts() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[
            0x20, 0x05, 0x80, // JSR $8005
            0x00, // BRK (should not reach here)
            0x00, // Unspecified (won’t be reached)
            0xE8, // INX at $8005
            0x60, // RTS
        ]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_x = 0x00;

        cpu.run(); // Execute JSR

        assert_eq!(cpu.program_counter, 0x8005);
        assert_eq!(cpu.stack_pointer, STACK_RESET - 0x2);

        cpu.run(); // Execute INX

        assert_eq!(cpu.register_x, 0x01);

        cpu.run(); // Execute RTS

        assert_eq!(cpu.program_counter, 0x8003);
    }

    #[test]
    fn test_branching_bne() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[
            0xA9, 0x00, // LDA #$00
            0xD0, 0x02, // BNE +2 (should not branch)
            0xA9, 0x01, // LDA #$01 (should execute)
            0x00, // BRK
        ]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run(); // LDA #$00
        cpu.run(); // BNE (should not branch)
        cpu.run(); // LDA #$01

        assert_eq!(cpu.register_a, 0x01);
    }

    #[test]
    fn test_branching_beq() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[
            0xA9, 0x00, // LDA #$00
            0xF0, 0x02, // BEQ +2 (should branch)
            0xA9, 0x01, // LDA #$01 (should be skipped)
            0x00, // BRK
        ]);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run(); // LDA #$00
        cpu.run(); // BEQ (should branch)
        cpu.run(); // BRK

        assert_eq!(cpu.register_a, 0x00);
    }

    #[test]
    fn test_cmp_equal() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xC9, 0x42]); // CMP #$42

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x42;

        cpu.run();

        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(cpu.get_flag(CARRY_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_cmp_less() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xC9, 0x50]); // CMP #$50

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x40;

        cpu.run();

        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(CARRY_FLAG));
        assert!(cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_inc_zero_page() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xE6, 0x10]); // INC $10
        mock_bus.mem_write(0x0010, 0xFF);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run();

        assert_eq!(cpu.bus.mem_read(0x0010), 0x00);
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_dec_absolute() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xCE, 0x00, 0x20]); // DEC $2000
        mock_bus.mem_write(0x2000, 0x01);

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run();

        assert_eq!(cpu.bus.mem_read(0x2000), 0x00);
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_eor_immediate() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x49, 0xFF]); // EOR #$FF

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0xFF;

        cpu.run();

        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_ora_immediate() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x09, 0x0F]); // ORA #$0F

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0xF0;

        cpu.run();

        assert_eq!(cpu.register_a, 0xFF);
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_rol_accumulator() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x2A]); // ROL A

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0b10000000;
        cpu.set_flag(CARRY_FLAG, false);

        cpu.run();

        assert_eq!(cpu.register_a, 0b00000000);
        assert!(cpu.get_flag(CARRY_FLAG));
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_ror_accumulator() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x6A]); // ROR A

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0b00000001;
        cpu.set_flag(CARRY_FLAG, false);

        cpu.run();

        assert_eq!(cpu.register_a, 0b00000000);
        assert!(cpu.get_flag(CARRY_FLAG));
        assert!(cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_sec_clc() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x38, 0x18]); // SEC, CLC

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run(); // SEC
        assert!(cpu.get_flag(CARRY_FLAG));

        cpu.run(); // CLC
        assert!(!cpu.get_flag(CARRY_FLAG));
    }

    #[test]
    fn test_sed_cld() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xF8, 0xD8]); // SED, CLD

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run(); // SED
        assert!(cpu.get_flag(DECIMAL_FLAG));

        cpu.run(); // CLD
        assert!(!cpu.get_flag(DECIMAL_FLAG));
    }

    #[test]
    fn test_sei_cli() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x78, 0x58]); // SEI, CLI

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();

        cpu.run(); // SEI
        assert!(cpu.get_flag(IRQ_FLAG));

        cpu.run(); // CLI
        assert!(!cpu.get_flag(IRQ_FLAG));
    }

    #[test]
    fn test_transfer_tay() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xA8]); // TAY

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x10;

        cpu.run();

        assert_eq!(cpu.register_y, 0x10);
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }

    #[test]
    fn test_stack_pha_pla() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x48, 0xA9, 0x00, 0x68]); // PHA, LDA #$00, PLA

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x77;

        cpu.run(); // PHA
        cpu.run(); // LDA #$00
        cpu.run(); // PLA

        assert_eq!(cpu.register_a, 0x77);
    }

    #[test]
    fn test_rti() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0x40]); // RTI

        // Set up the stack to simulate an interrupt return
        mock_bus.mem_write(0x01FF, 0x80); // PCH
        mock_bus.mem_write(0x01FE, 0x20); // PCL
        mock_bus.mem_write(0x01FD, 0x00); // Status
        let mut cpu = CPU::new(mock_bus);

        cpu.reset();
        cpu.stack_pointer = 0xFC;

        cpu.run();

        assert_eq!(cpu.program_counter, 0x8020);
    }

    #[test]
    fn test_sbc_immediate() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xE9, 0x01]); // SBC #$01

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x03;
        cpu.set_flag(CARRY_FLAG, true);

        cpu.run();

        assert_eq!(cpu.register_a, 0x02);
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
        assert!(cpu.get_flag(CARRY_FLAG));
    }

    #[test]
    fn test_sbc_with_borrow() {
        let mut mock_bus = MockBus::new();
        mock_bus.load(&[0xE9, 0x01]); // SBC #$01

        let mut cpu = CPU::new(mock_bus);
        cpu.reset();
        cpu.register_a = 0x00;
        cpu.set_flag(CARRY_FLAG, false); // Borrow

        cpu.run();

        assert_eq!(cpu.register_a, 0xFE);
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(cpu.get_flag(NEGATIVE_FLAG));
        assert!(!cpu.get_flag(CARRY_FLAG));
    }
}
