#![allow(dead_code)]

pub const HLT: u8 = 0x00;
pub const NOP: u8 = 0x01;
pub const PUSH: u8 = 0x02;
pub const POP: u8 = 0x04;
pub const DUP: u8 = 0x05;
pub const SWAP: u8 = 0x06;

pub const ADD: u8 = 0x10;
pub const SUB: u8 = 0x11;
pub const MUL: u8 = 0x12;
pub const DIV: u8 = 0x13;
pub const MOD: u8 = 0x14;

pub const CMP: u8 = 0x20;
pub const EQ: u8 = 0x21;
pub const NEQ: u8 = 0x22;
pub const GT: u8 = 0x23;
pub const LT: u8 = 0x24;

pub const JMP: u8 = 0x30;
pub const JZ: u8 = 0x31;
pub const JNZ: u8 = 0x32;
pub const CALL: u8 = 0x33;
pub const RET: u8 = 0x34;

pub const ENTER: u8 = 0x35;
pub const LEAVE: u8 = 0x36;
pub const LOAD_ARG: u8 = 0x37;
pub const STORE_ARG: u8 = 0x38;

pub const LOAD: u8 = 0x40;
pub const STORE: u8 = 0x41;
pub const LOAD_REL: u8 = 0x42;
pub const STORE_REL: u8 = 0x43;
pub const LOAD_ABS: u8 = 0x44;
pub const STORE_ABS: u8 = 0x45;

pub const SYSCALL: u8 = 0x50;
pub const BREAK: u8 = 0x51;
