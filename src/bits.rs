use num_derive::FromPrimitive;

use enum_map::EnumMap;

pub fn sign_extend(mut x: u16, bit_count: usize) -> u16 {
    if ((x >> (bit_count - 1)) & 1) != 0 {
        x |= 0xFFFF << bit_count;
    }
    x
}

#[derive(Debug, Default, Clone)]
pub struct DiagnosticStatus {
    pub registers: EnumMap<Register, u16>,
    pub memory_view_range: (usize, usize),
    pub memory_view: Vec<u16>,
}

#[derive(Debug, Enum)]
pub enum Register {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    PC,
    COND,
    PSR,
}

impl Register {
    pub fn from_u16(value: u16) -> Self {
        use enum_map::Enum;
        Enum::<Self>::from_usize(value as usize)
    }
}

#[derive(Debug, FromPrimitive)]
pub enum Opcode {
    BR = 0b0000,   // branch
    ADD = 0b0001,  // add
    LD = 0b0010,   // load
    ST = 0b0011,   // store
    JSR = 0b0100,  // jump register
    AND = 0b0101,  // bitwise and
    LDR = 0b0110,  // load register
    STR = 0b0111,  // store register
    RTI = 0b1000,  // unused
    NOT = 0b1001,  // bitwise not
    LDI = 0b1010,  // load indirect
    STI = 0b1011,  // store indirect
    JMP = 0b1100,  // jump
    RES = 0b1101,  // reserved (unused)
    LEA = 0b1110,  // load effective address
    TRAP = 0b1111, // execute trap
}

pub enum ConditionFlags {
    POS = 1 << 0, // P
    ZRO = 1 << 1, // Z
    NEG = 1 << 2, // N
}

pub enum MemoryMappedRegister {
    KBSR = 0xFE00,
    KBDR = 0xFE02,
}

#[derive(FromPrimitive)]
pub enum TrapCode {
    GetC = 0x20,  // get character from keyboard
    Out = 0x21,   // output a character
    Puts = 0x22,  // output a word string
    In = 0x23,    // input a string
    PutSp = 0x24, // output a byte string
    Halt = 0x25,  // halt the program
}
