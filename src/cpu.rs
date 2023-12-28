use crate::bus::Bus;
use crate::opcodes::{Mnemonic, OPCODES_MAP};

const CARRY_FLAG: u8 = 1 << 0;
const ZERO_FLAG: u8 = 1 << 1;
const IRQ_FLAG: u8 = 1 << 2;
const DECIMAL_FLAG: u8 = 1 << 3;
const BREAK_FLAG: u8 = 1 << 4;
const OVERFLOW_FLAG: u8 = 1 << 6;
const NEGATIVE_FLAG: u8 = 1 << 7;

// const BRK_VECTOR: u16 = 0xfffe;

const STACK_RESET: u8 = 0xFD;

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub processor_status: u8,
    pub stack_pointer: u8,
    pub program_counter: u16,
    pub bus: Bus,
}

#[derive(Debug)]
pub enum AddressingMode {
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
    NoneAddressing,
}

fn page_cross(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xFF00 != addr2 & 0xFF00
}

impl CPU {
    pub fn new(mut bus: Bus) -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            processor_status: 0b100100,
            stack_pointer: STACK_RESET,
            program_counter: bus.mem_read_u16(0xFFFC),
            bus: bus,
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
            AddressingMode::Immediate => (address, false),

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
                let ptr = (base as u8).wrapping_add(self.register_x) & 0xFF;

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
                let hi = self.bus.mem_read((base as u8).wrapping_add(1) as u16);
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
            self.bus.tick(1);
        }
    }

    fn and(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);
        self.register_a &= value;

        self.set_zero_and_negative_flags(self.register_a);

        if page_cross {
            self.bus.tick(1);
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
                let (addr, _) = self.get_operand_address(&mode);
                let value = self.bus.mem_read(addr);
                let result = value << 1;

                self.bus.mem_write(addr, result);

                self.set_flag(CARRY_FLAG, (value & 0x80) != 0);
                self.set_zero_and_negative_flags(result);
            }
        }
    }

    fn branch(&mut self) {
        let offset = self.bus.mem_read(self.program_counter) as i8;
        let jump_addr = self
            .program_counter
            .wrapping_add(1)
            .wrapping_add(offset as u16);

        let initial_high_byte = (self.program_counter.wrapping_add(1) >> 8) as u8;
        let target_high_byte = (jump_addr >> 8) as u8;

        self.program_counter = jump_addr;

        if initial_high_byte != target_high_byte {
            self.bus.tick(2);
        } else {
            self.bus.tick(1);
        }
    }

    fn bcc(&mut self) {
        if !self.get_flag(CARRY_FLAG) {
            self.branch()
        }
    }

    fn bcs(&mut self) {
        if self.get_flag(CARRY_FLAG) {
            self.branch()
        }
    }

    fn beq(&mut self) {
        if self.get_flag(ZERO_FLAG) {
            self.branch()
        }
    }

    fn bne(&mut self) {
        if !self.get_flag(ZERO_FLAG) {
            self.branch()
        }
    }

    fn bmi(&mut self) {
        if self.get_flag(NEGATIVE_FLAG) {
            self.branch()
        }
    }

    fn bpl(&mut self) {
        if !self.get_flag(NEGATIVE_FLAG) {
            self.branch()
        }
    }

    fn bvc(&mut self) {
        if !self.get_flag(OVERFLOW_FLAG) {
            self.branch()
        }
    }

    fn bvs(&mut self) {
        if self.get_flag(OVERFLOW_FLAG) {
            self.branch()
        }
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);

        self.set_flag(ZERO_FLAG, (self.register_a & value) == 0);
        self.set_flag(NEGATIVE_FLAG, (value & 0x80) != 0);
        self.set_flag(OVERFLOW_FLAG, (value & 0x40) != 0);
    }

    // fn brk(&mut self) {
    //     self.push_stack16(self.program_counter);
    //     self.push_stack(self.stack_pointer);

    //     self.program_counter = self.bus.mem_read_u16(BRK_VECTOR);

    //     self.set_flag(BREAK_FLAG, true);
    // }

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
        let (addr, page_cross) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);
        let result = reg.wrapping_sub(value);

        self.set_flag(CARRY_FLAG, reg >= value);
        self.set_flag(ZERO_FLAG, reg == value);
        self.set_flag(NEGATIVE_FLAG, (result & 0x80) != 0);

        if page_cross {
            self.bus.tick(1);
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
        let (addr, _) = self.get_operand_address(&mode);
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
        let (addr, page_cross) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);

        self.register_a ^= value;

        self.set_zero_and_negative_flags(self.register_a);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
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
        let (addr, _) = self.get_operand_address(&mode);
        self.program_counter = addr;
    }

    fn jsr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
        self.push_stack16(self.program_counter + 2 - 1);

        self.program_counter = addr;
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);

        self.set_zero_and_negative_flags(value);

        self.register_a = value;

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);

        self.set_zero_and_negative_flags(value);

        self.register_x = value;

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);

        self.set_zero_and_negative_flags(value);

        self.register_y = value;

        if page_cross {
            self.bus.tick(1);
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
                let (addr, _) = self.get_operand_address(&mode);
                let value = self.bus.mem_read(addr);
                let result = value >> 1;

                self.bus.mem_write(addr, result);

                self.set_flag(CARRY_FLAG, (value & 1) == 1);
                self.set_zero_and_negative_flags(result);
            }
        }
    }

    fn nop(&mut self) {
        // do nothing
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);

        self.register_a |= value;

        self.set_zero_and_negative_flags(self.register_a);

        if page_cross {
            self.bus.tick(1);
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
                let (addr, _) = self.get_operand_address(&mode);
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
                let (addr, _) = self.get_operand_address(&mode);
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
        let value = self.pop_stack();
        self.set_flags(value);

        self.program_counter = self.pop_stack16();
    }

    fn rts(&mut self) {
        self.program_counter = self.pop_stack16().wrapping_add(1);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.bus.mem_read(addr);

        let accumulator = self.register_a;

        let carry_flag = self.get_flag(CARRY_FLAG) as u8;

        let result = accumulator.wrapping_sub(value).wrapping_sub(1 - carry_flag);

        // Update the Carry flag (set if there was no borrow)
        self.set_flag(CARRY_FLAG, result <= accumulator);

        let overflow =
            ((accumulator ^ value) as u8) & 0x80 != 0 && ((accumulator ^ result) as u8) & 0x80 != 0;

        self.set_flag(OVERFLOW_FLAG, overflow);

        self.register_a = result;

        self.set_zero_and_negative_flags(self.register_a);

        if page_cross {
            self.bus.tick(1);
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
    fn lax(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);

        self.register_a = value;
        self.register_x = value;

        self.set_zero_and_negative_flags(value);
    }

    fn sax(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
        self.bus.mem_write(addr, self.register_a & self.register_x);
    }

    fn dcp(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);
        let decremented_value = value.wrapping_sub(1);

        self.bus.mem_write(addr, decremented_value);

        self.set_flag(CARRY_FLAG, self.register_a >= value);
        self.set_zero_and_negative_flags(self.register_a.wrapping_sub(decremented_value));
    }

    fn isb(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);
        let incremented_value = value.wrapping_add(1);

        self.bus.mem_write(addr, incremented_value);

        let borrow = if self.get_flag(CARRY_FLAG) { 0 } else { 1 };

        let result = self
            .register_a
            .wrapping_sub(incremented_value)
            .wrapping_sub(borrow);

        self.set_flag(CARRY_FLAG, self.register_a >= result);

        let overflow = ((self.register_a ^ value) as u8) & 0x80 != 0
            && ((self.register_a ^ result) as u8) & 0x80 != 0;

        self.set_flag(OVERFLOW_FLAG, overflow);

        self.register_a = result;

        self.set_zero_and_negative_flags(self.register_a);
    }

    fn slo(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);
        let result = value << 1;

        self.bus.mem_write(addr, result);

        self.set_flag(CARRY_FLAG, (value & 0x80) != 0);
        self.set_zero_and_negative_flags(result);

        self.register_a |= result;

        self.set_zero_and_negative_flags(self.register_a);
    }

    fn rla(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
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

    fn sre(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
        let value = self.bus.mem_read(addr);

        let result = value >> 1;

        self.bus.mem_write(addr, result);

        self.set_flag(CARRY_FLAG, (value & 1) == 1);
        self.set_zero_and_negative_flags(result);

        self.register_a ^= result;

        self.set_zero_and_negative_flags(self.register_a);
    }

    fn rra(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(&mode);
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

    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.bus.mem_write(0x8600 + i, program[i as usize]);
        }

        self.bus.mem_write_u16(0xFFFC, 0x8600);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.processor_status = 0b100100;
        self.stack_pointer = STACK_RESET;

        self.program_counter = self.bus.mem_read_u16(0xFFFC);
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        loop {
            let code = self.bus.mem_read(self.program_counter);
            self.program_counter = self.program_counter.wrapping_add(1);
            let program_counter_state = self.program_counter;

            let opcode = OPCODES_MAP
                .get(&code)
                .expect(&format!("OpCode {:x} is not recognized", code));

            match opcode.mnemonic {
                Mnemonic::ADC => self.adc(&opcode.mode),
                Mnemonic::AND => self.and(&opcode.mode),
                Mnemonic::ASL => self.asl(&opcode.mode),
                Mnemonic::BCC => self.bcc(),
                Mnemonic::BCS => self.bcs(),
                Mnemonic::BEQ => self.beq(),
                Mnemonic::BNE => self.bne(),
                Mnemonic::BMI => self.bmi(),
                Mnemonic::BPL => self.bpl(),
                Mnemonic::BVC => self.bvc(),
                Mnemonic::BVS => self.bvs(),
                Mnemonic::BIT => self.bit(&opcode.mode),
                Mnemonic::BRK => return,
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
                Mnemonic::NOP => self.nop(),
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
                Mnemonic::LAX => self.lax(&opcode.mode),
                Mnemonic::SAX => self.sax(&opcode.mode),
                Mnemonic::DCP => self.dcp(&opcode.mode),
                Mnemonic::ISB => self.isb(&opcode.mode),
                Mnemonic::SLO => self.slo(&opcode.mode),
                Mnemonic::RLA => self.rla(&opcode.mode),
                Mnemonic::SRE => self.sre(&opcode.mode),
                Mnemonic::RRA => self.rra(&opcode.mode),
                _ => todo!("{:?}", opcode.mnemonic),
            }
            self.bus.tick(opcode.cycles);

            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }

            callback(self);
        }
    }
}