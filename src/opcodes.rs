use crate::cpu::AddressingMode;
use std::collections::HashMap;

pub struct OpCode {
    pub code: u8,
    pub mnemonic: Mnemonic,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

#[derive(Debug)]
pub enum Mnemonic {
    INV,
    ADC,
    AHX,
    ALR,
    ANC,
    AND,
    ARR,
    ASL,
    AXS,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DCP,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    ISB,
    JMP,
    JSR,
    LAX,
    LAS,
    LDA,
    LDX,
    LDY,
    LSR,
    LXA,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    RLA,
    ROL,
    ROR,
    RRA,
    RTI,
    RTS,
    SAX,
    SBC,
    SEC,
    SED,
    SEI,
    SHX,
    SHY,
    SLO,
    SRE,
    STA,
    STX,
    STY,
    TAS,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
    XAA,
}

impl OpCode {
    fn new(code: u8, mnemonic: Mnemonic, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        OpCode {
            code,
            mnemonic,
            len,
            cycles,
            mode,
        }
    }
}

lazy_static! {
    pub static ref CPU_OPS_CODES: Vec<OpCode> = vec![
        OpCode::new(0x00, Mnemonic::BRK, 1, 7, AddressingMode::NoneAddressing),
        OpCode::new(0xea, Mnemonic::NOP, 1, 2, AddressingMode::NoneAddressing),

        /* Arithmetic */
        OpCode::new(0x69, Mnemonic::ADC, 2, 2, AddressingMode::Immediate),
        OpCode::new(0x65, Mnemonic::ADC, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x75, Mnemonic::ADC, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x6d, Mnemonic::ADC, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x7d, Mnemonic::ADC, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        OpCode::new(0x79, Mnemonic::ADC, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        OpCode::new(0x61, Mnemonic::ADC, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x71, Mnemonic::ADC, 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        OpCode::new(0xe9, Mnemonic::SBC, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xe5, Mnemonic::SBC, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xf5, Mnemonic::SBC, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xed, Mnemonic::SBC, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xfd, Mnemonic::SBC, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        OpCode::new(0xf9, Mnemonic::SBC, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        OpCode::new(0xe1, Mnemonic::SBC, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xf1, Mnemonic::SBC, 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        OpCode::new(0x29, Mnemonic::AND, 2, 2, AddressingMode::Immediate),
        OpCode::new(0x25, Mnemonic::AND, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x35, Mnemonic::AND, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x2d, Mnemonic::AND, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x3d, Mnemonic::AND, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        OpCode::new(0x39, Mnemonic::AND, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        OpCode::new(0x21, Mnemonic::AND, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x31, Mnemonic::AND, 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        OpCode::new(0x49, Mnemonic::EOR, 2, 2, AddressingMode::Immediate),
        OpCode::new(0x45, Mnemonic::EOR, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x55, Mnemonic::EOR, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x4d, Mnemonic::EOR, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x5d, Mnemonic::EOR, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        OpCode::new(0x59, Mnemonic::EOR, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        OpCode::new(0x41, Mnemonic::EOR, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x51, Mnemonic::EOR, 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        OpCode::new(0x09, Mnemonic::ORA, 2, 2, AddressingMode::Immediate),
        OpCode::new(0x05, Mnemonic::ORA, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x15, Mnemonic::ORA, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x0d, Mnemonic::ORA, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x1d, Mnemonic::ORA, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        OpCode::new(0x19, Mnemonic::ORA, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        OpCode::new(0x01, Mnemonic::ORA, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x11, Mnemonic::ORA, 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        /* Shifts */
        OpCode::new(0x0a, Mnemonic::ASL, 1, 2, AddressingMode::Accumulator),
        OpCode::new(0x06, Mnemonic::ASL, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x16, Mnemonic::ASL, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x0e, Mnemonic::ASL, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x1e, Mnemonic::ASL, 3, 7, AddressingMode::AbsoluteX),

        OpCode::new(0x4a, Mnemonic::LSR, 1, 2, AddressingMode::Accumulator),
        OpCode::new(0x46, Mnemonic::LSR, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x56, Mnemonic::LSR, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x4e, Mnemonic::LSR, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x5e, Mnemonic::LSR, 3, 7, AddressingMode::AbsoluteX),

        OpCode::new(0x2a, Mnemonic::ROL, 1, 2, AddressingMode::Accumulator),
        OpCode::new(0x26, Mnemonic::ROL, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x36, Mnemonic::ROL, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x2e, Mnemonic::ROL, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x3e, Mnemonic::ROL, 3, 7, AddressingMode::AbsoluteX),

        OpCode::new(0x6a, Mnemonic::ROR, 1, 2, AddressingMode::Accumulator),
        OpCode::new(0x66, Mnemonic::ROR, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x76, Mnemonic::ROR, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x6e, Mnemonic::ROR, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x7e, Mnemonic::ROR, 3, 7, AddressingMode::AbsoluteX),

        OpCode::new(0xe6, Mnemonic::INC, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xf6, Mnemonic::INC, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0xee, Mnemonic::INC, 3, 6, AddressingMode::Absolute),
        OpCode::new(0xfe, Mnemonic::INC, 3, 7, AddressingMode::AbsoluteX),

        OpCode::new(0xe8, Mnemonic::INX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xc8, Mnemonic::INY, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0xc6, Mnemonic::DEC, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xd6, Mnemonic::DEC, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0xce, Mnemonic::DEC, 3, 6, AddressingMode::Absolute),
        OpCode::new(0xde, Mnemonic::DEC, 3, 7, AddressingMode::AbsoluteX),

        OpCode::new(0xca, Mnemonic::DEX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x88, Mnemonic::DEY, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0xc9, Mnemonic::CMP, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xc5, Mnemonic::CMP, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xd5, Mnemonic::CMP, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xcd, Mnemonic::CMP, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xdd, Mnemonic::CMP, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        OpCode::new(0xd9, Mnemonic::CMP, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        OpCode::new(0xc1, Mnemonic::CMP, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xd1, Mnemonic::CMP, 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        OpCode::new(0xc0, Mnemonic::CPY, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xc4, Mnemonic::CPY, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xcc, Mnemonic::CPY, 3, 4, AddressingMode::Absolute),

        OpCode::new(0xe0, Mnemonic::CPX, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xe4, Mnemonic::CPX, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xec, Mnemonic::CPX, 3, 4, AddressingMode::Absolute),


        /* Branching */

        OpCode::new(0x4c, Mnemonic::JMP, 3, 3, AddressingMode::Absolute), //AddressingMode that acts as Immidiate
        OpCode::new(0x6c, Mnemonic::JMP, 3, 5, AddressingMode::Indirect), //AddressingMode:Indirect with 6502 bug

        OpCode::new(0x20, Mnemonic::JSR, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x60, Mnemonic::RTS, 1, 6, AddressingMode::NoneAddressing),

        OpCode::new(0x40, Mnemonic::RTI, 1, 6, AddressingMode::NoneAddressing),

        OpCode::new(0xd0, Mnemonic::BNE, 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x70, Mnemonic::BVS, 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x50, Mnemonic::BVC, 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x30, Mnemonic::BMI, 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0xf0, Mnemonic::BEQ, 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0xb0, Mnemonic::BCS, 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x90, Mnemonic::BCC, 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x10, Mnemonic::BPL, 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),

        OpCode::new(0x24, Mnemonic::BIT, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x2c, Mnemonic::BIT, 3, 4, AddressingMode::Absolute),


        /* Stores, Loads */
        OpCode::new(0xa9, Mnemonic::LDA, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xa5, Mnemonic::LDA, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xb5, Mnemonic::LDA, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xad, Mnemonic::LDA, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xbd, Mnemonic::LDA, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        OpCode::new(0xb9, Mnemonic::LDA, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        OpCode::new(0xa1, Mnemonic::LDA, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xb1, Mnemonic::LDA, 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        OpCode::new(0xa2, Mnemonic::LDX, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xa6, Mnemonic::LDX, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xb6, Mnemonic::LDX, 2, 4, AddressingMode::ZeroPageY),
        OpCode::new(0xae, Mnemonic::LDX, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xbe, Mnemonic::LDX, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),

        OpCode::new(0xa0, Mnemonic::LDY, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xa4, Mnemonic::LDY, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xb4, Mnemonic::LDY, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xac, Mnemonic::LDY, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xbc, Mnemonic::LDY, 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),


        OpCode::new(0x85, Mnemonic::STA, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x95, Mnemonic::STA, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x8d, Mnemonic::STA, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x9d, Mnemonic::STA, 3, 5, AddressingMode::AbsoluteX),
        OpCode::new(0x99, Mnemonic::STA, 3, 5, AddressingMode::AbsoluteY),
        OpCode::new(0x81, Mnemonic::STA, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x91, Mnemonic::STA, 2, 6, AddressingMode::IndirectY),

        OpCode::new(0x86, Mnemonic::STX, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x96, Mnemonic::STX, 2, 4, AddressingMode::ZeroPageY),
        OpCode::new(0x8e, Mnemonic::STX, 3, 4, AddressingMode::Absolute),

        OpCode::new(0x84, Mnemonic::STY, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x94, Mnemonic::STY, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x8c, Mnemonic::STY, 3, 4, AddressingMode::Absolute),


        /* Flags clear */

        OpCode::new(0xD8, Mnemonic::CLD, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x58, Mnemonic::CLI, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xb8, Mnemonic::CLV, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x18, Mnemonic::CLC, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x38, Mnemonic::SEC, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x78, Mnemonic::SEI, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xf8, Mnemonic::SED, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0xaa, Mnemonic::TAX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xa8, Mnemonic::TAY, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xba, Mnemonic::TSX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x8a, Mnemonic::TXA, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x9a, Mnemonic::TXS, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x98, Mnemonic::TYA, 1, 2, AddressingMode::NoneAddressing),

        /* Stack */
        OpCode::new(0x48, Mnemonic::PHA, 1, 3, AddressingMode::NoneAddressing),
        OpCode::new(0x68, Mnemonic::PLA, 1, 4, AddressingMode::NoneAddressing),
        OpCode::new(0x08, Mnemonic::PHP, 1, 3, AddressingMode::NoneAddressing),
        OpCode::new(0x28, Mnemonic::PLP, 1, 4, AddressingMode::NoneAddressing),


        /* unofficial */

        OpCode::new(0xc7, Mnemonic::DCP, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xd7, Mnemonic::DCP, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0xCF, Mnemonic::DCP, 3, 6, AddressingMode::Absolute),
        OpCode::new(0xdF, Mnemonic::DCP, 3, 7, AddressingMode::AbsoluteX),
        OpCode::new(0xdb, Mnemonic::DCP, 3, 7, AddressingMode::AbsoluteY),
        OpCode::new(0xd3, Mnemonic::DCP, 2, 8, AddressingMode::IndirectY),
        OpCode::new(0xc3, Mnemonic::DCP, 2, 8, AddressingMode::IndirectX),


        OpCode::new(0x27, Mnemonic::RLA, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x37, Mnemonic::RLA, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x2F, Mnemonic::RLA, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x3F, Mnemonic::RLA, 3, 7, AddressingMode::AbsoluteX),
        OpCode::new(0x3b, Mnemonic::RLA, 3, 7, AddressingMode::AbsoluteY),
        OpCode::new(0x33, Mnemonic::RLA, 2, 8, AddressingMode::IndirectY),
        OpCode::new(0x23, Mnemonic::RLA, 2, 8, AddressingMode::IndirectX),

        OpCode::new(0x07, Mnemonic::SLO, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x17, Mnemonic::SLO, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x0F, Mnemonic::SLO, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x1f, Mnemonic::SLO, 3, 7, AddressingMode::AbsoluteX),
        OpCode::new(0x1b, Mnemonic::SLO, 3, 7, AddressingMode::AbsoluteY),
        OpCode::new(0x03, Mnemonic::SLO, 2, 8, AddressingMode::IndirectX),
        OpCode::new(0x13, Mnemonic::SLO, 2, 8, AddressingMode::IndirectY),

        OpCode::new(0x47, Mnemonic::SRE, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x57, Mnemonic::SRE, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x4F, Mnemonic::SRE, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x5f, Mnemonic::SRE, 3, 7, AddressingMode::AbsoluteX),
        OpCode::new(0x5b, Mnemonic::SRE, 3, 7, AddressingMode::AbsoluteY),
        OpCode::new(0x43, Mnemonic::SRE, 2, 8, AddressingMode::IndirectX),
        OpCode::new(0x53, Mnemonic::SRE, 2, 8, AddressingMode::IndirectY),


        OpCode::new(0x80, Mnemonic::NOP, 2,2, AddressingMode::Immediate),
        OpCode::new(0x82, Mnemonic::NOP, 2,2, AddressingMode::Immediate),
        OpCode::new(0x89, Mnemonic::NOP, 2,2, AddressingMode::Immediate),
        OpCode::new(0xc2, Mnemonic::NOP, 2,2, AddressingMode::Immediate),
        OpCode::new(0xe2, Mnemonic::NOP, 2,2, AddressingMode::Immediate),


        OpCode::new(0xCB, Mnemonic::AXS, 2,2, AddressingMode::Immediate),

        OpCode::new(0x6B, Mnemonic::ARR, 2,2, AddressingMode::Immediate),

        OpCode::new(0xeb, Mnemonic::SBC, 2,2, AddressingMode::Immediate),

        OpCode::new(0x0b, Mnemonic::ANC, 2,2, AddressingMode::Immediate),
        OpCode::new(0x2b, Mnemonic::ANC, 2,2, AddressingMode::Immediate),

        OpCode::new(0x4b, Mnemonic::ALR, 2,2, AddressingMode::Immediate),
        // OpCode::new(0xCB, Mnemonic::IGN, 3,4 /* or 5*/, AddressingMode::AbsoluteX),

        OpCode::new(0x04, Mnemonic::NOP, 2,3, AddressingMode::ZeroPage),
        OpCode::new(0x44, Mnemonic::NOP, 2,3, AddressingMode::ZeroPage),
        OpCode::new(0x64, Mnemonic::NOP, 2,3, AddressingMode::ZeroPage),
        OpCode::new(0x14, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x34, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x54, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x74, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xd4, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xf4, Mnemonic::NOP, 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x0c, Mnemonic::NOP, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x1c, Mnemonic::NOP, 3, 4 /*or 5*/, AddressingMode::AbsoluteX),
        OpCode::new(0x3c, Mnemonic::NOP, 3, 4 /*or 5*/, AddressingMode::AbsoluteX),
        OpCode::new(0x5c, Mnemonic::NOP, 3, 4 /*or 5*/, AddressingMode::AbsoluteX),
        OpCode::new(0x7c, Mnemonic::NOP, 3, 4 /*or 5*/, AddressingMode::AbsoluteX),
        OpCode::new(0xdc, Mnemonic::NOP, 3, 4 /* or 5*/, AddressingMode::AbsoluteX),
        OpCode::new(0xfc, Mnemonic::NOP, 3, 4 /* or 5*/, AddressingMode::AbsoluteX),

        OpCode::new(0x67, Mnemonic::RRA, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x77, Mnemonic::RRA, 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x6f, Mnemonic::RRA, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x7f, Mnemonic::RRA, 3, 7, AddressingMode::AbsoluteX),
        OpCode::new(0x7b, Mnemonic::RRA, 3, 7, AddressingMode::AbsoluteY),
        OpCode::new(0x63, Mnemonic::RRA, 2, 8, AddressingMode::IndirectX),
        OpCode::new(0x73, Mnemonic::RRA, 2, 8, AddressingMode::IndirectY),


        OpCode::new(0xe7, Mnemonic::ISB, 2,5, AddressingMode::ZeroPage),
        OpCode::new(0xf7, Mnemonic::ISB, 2,6, AddressingMode::ZeroPageX),
        OpCode::new(0xef, Mnemonic::ISB, 3,6, AddressingMode::Absolute),
        OpCode::new(0xff, Mnemonic::ISB, 3,7, AddressingMode::AbsoluteX),
        OpCode::new(0xfb, Mnemonic::ISB, 3,7, AddressingMode::AbsoluteY),
        OpCode::new(0xe3, Mnemonic::ISB, 2,8, AddressingMode::IndirectX),
        OpCode::new(0xf3, Mnemonic::ISB, 2,8, AddressingMode::IndirectY),

        OpCode::new(0x02, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x12, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x22, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x32, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x42, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x52, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x62, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x72, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x92, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0xb2, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0xd2, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0xf2, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),

        OpCode::new(0x1a, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x3a, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x5a, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0x7a, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0xda, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        // OpCode::new(0xea, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),
        OpCode::new(0xfa, Mnemonic::NOP, 1,2, AddressingMode::NoneAddressing),

        OpCode::new(0xab, Mnemonic::LXA, 2, 3, AddressingMode::Immediate), //todo: highly unstable and not used
        //http://visual6502.org/wiki/index.php?title=6502_Opcode_8B_%28XAA,_ANE%29
        OpCode::new(0x8b, Mnemonic::XAA, 2, 3, AddressingMode::Immediate), //todo: highly unstable and not used
        OpCode::new(0xbb, Mnemonic::LAS, 3, 2, AddressingMode::AbsoluteY), //todo: highly unstable and not used
        OpCode::new(0x9b, Mnemonic::TAS, 3, 2, AddressingMode::AbsoluteY), //todo: highly unstable and not used
        OpCode::new(0x93, Mnemonic::AHX, 2, /* guess */ 8, AddressingMode::IndirectY), //todo: highly unstable and not used
        OpCode::new(0x9f, Mnemonic::AHX, 3, /* guess */ 4/* or 5*/, AddressingMode::AbsoluteY), //todo: highly unstable and not used
        OpCode::new(0x9e, Mnemonic::SHX, 3, /* guess */ 4/* or 5*/, AddressingMode::AbsoluteY), //todo: highly unstable and not used
        OpCode::new(0x9c, Mnemonic::SHY, 3, /* guess */ 4/* or 5*/, AddressingMode::AbsoluteX), //todo: highly unstable and not used

        OpCode::new(0xa7, Mnemonic::LAX, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xb7, Mnemonic::LAX, 2, 4, AddressingMode::ZeroPageY),
        OpCode::new(0xaf, Mnemonic::LAX, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xbf, Mnemonic::LAX, 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0xa3, Mnemonic::LAX, 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xb3, Mnemonic::LAX, 2, 5, AddressingMode::IndirectY),

        OpCode::new(0x87, Mnemonic::SAX, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x97, Mnemonic::SAX, 2, 4, AddressingMode::ZeroPageY),
        OpCode::new(0x8f, Mnemonic::SAX, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x83, Mnemonic::SAX, 2, 6, AddressingMode::IndirectX),

    ];


    pub static ref OPCODES_MAP: HashMap<u8, &'static OpCode> = {
        let mut map = HashMap::new();
        for cpuop in &*CPU_OPS_CODES {
            map.insert(cpuop.code, cpuop);
        }
        map
    };
}
