#![allow(unused)]

use crate::dwarf::encoding::{decode_sleb128, decode_uleb128};
use crate::registers::Registers;
use crate::utils::load;
use std::ops::{Index, IndexMut};

// These DW_* constants were taken from version 3 of the DWARF standard,
// which is Copyright (c) 2005 Free Standards Group, and
// Copyright (c) 1992, 1993 UNIX International, Inc.
//
// DWARF expressions.
const DW_OP_ADDR: u8 = 0x03; // pub constant address (size target specific)
const DW_OP_DEREF: u8 = 0x06;
const DW_OP_CONST1U: u8 = 0x08; // 1-byte pub constant
const DW_OP_CONST1S: u8 = 0x09; // 1-byte pub constant
const DW_OP_CONST2U: u8 = 0x0A; // 2-byte pub constant
const DW_OP_CONST2S: u8 = 0x0B; // 2-byte pub constant
const DW_OP_CONST4U: u8 = 0x0C; // 4-byte pub constant
const DW_OP_CONST4S: u8 = 0x0D; // 4-byte pub constant
const DW_OP_CONST8U: u8 = 0x0E; // 8-byte pub constant
const DW_OP_CONST8S: u8 = 0x0F; // 8-byte pub constant
const DW_OP_CONSTU: u8 = 0x10; // ULEB128 pub constant
const DW_OP_CONSTS: u8 = 0x11; // SLEB128 pub constant
const DW_OP_DUP: u8 = 0x12;
const DW_OP_DROP: u8 = 0x13;
const DW_OP_OVER: u8 = 0x14;
const DW_OP_PICK: u8 = 0x15; // 1-byte stack index
const DW_OP_SWAP: u8 = 0x16;
const DW_OP_ROT: u8 = 0x17;
const DW_OP_XDEREF: u8 = 0x18;
const DW_OP_ABS: u8 = 0x19;
const DW_OP_AND: u8 = 0x1A;
const DW_OP_DIV: u8 = 0x1B;
const DW_OP_MINUS: u8 = 0x1C;
const DW_OP_MOD: u8 = 0x1D;
const DW_OP_MUL: u8 = 0x1E;
const DW_OP_NEG: u8 = 0x1F;
const DW_OP_NOT: u8 = 0x20;
const DW_OP_OR: u8 = 0x21;
const DW_OP_PLUS: u8 = 0x22;
const DW_OP_PLUS_UCONST: u8 = 0x23; // ULEB128 addend
const DW_OP_SHL: u8 = 0x24;
const DW_OP_SHR: u8 = 0x25;
const DW_OP_SHRA: u8 = 0x26;
const DW_OP_XOR: u8 = 0x27;
const DW_OP_SKIP: u8 = 0x2F; // signed 2-byte pub constant
const DW_OP_BRA: u8 = 0x28; // signed 2-byte pub constant
const DW_OP_EQ: u8 = 0x29;
const DW_OP_GE: u8 = 0x2A;
const DW_OP_GT: u8 = 0x2B;
const DW_OP_LE: u8 = 0x2C;
const DW_OP_LT: u8 = 0x2D;
const DW_OP_NE: u8 = 0x2E;
const DW_OP_LIT0: u8 = 0x30; // Literal 0
const DW_OP_LIT1: u8 = 0x31; // Literal 1
const DW_OP_LIT2: u8 = 0x32; // Literal 2
const DW_OP_LIT3: u8 = 0x33; // Literal 3
const DW_OP_LIT4: u8 = 0x34; // Literal 4
const DW_OP_LIT5: u8 = 0x35; // Literal 5
const DW_OP_LIT6: u8 = 0x36; // Literal 6
const DW_OP_LIT7: u8 = 0x37; // Literal 7
const DW_OP_LIT8: u8 = 0x38; // Literal 8
const DW_OP_LIT9: u8 = 0x39; // Literal 9
const DW_OP_LIT10: u8 = 0x3A; // Literal 10
const DW_OP_LIT11: u8 = 0x3B; // Literal 11
const DW_OP_LIT12: u8 = 0x3C; // Literal 12
const DW_OP_LIT13: u8 = 0x3D; // Literal 13
const DW_OP_LIT14: u8 = 0x3E; // Literal 14
const DW_OP_LIT15: u8 = 0x3F; // Literal 15
const DW_OP_LIT16: u8 = 0x40; // Literal 16
const DW_OP_LIT17: u8 = 0x41; // Literal 17
const DW_OP_LIT18: u8 = 0x42; // Literal 18
const DW_OP_LIT19: u8 = 0x43; // Literal 19
const DW_OP_LIT20: u8 = 0x44; // Literal 20
const DW_OP_LIT21: u8 = 0x45; // Literal 21
const DW_OP_LIT22: u8 = 0x46; // Literal 22
const DW_OP_LIT23: u8 = 0x47; // Literal 23
const DW_OP_LIT24: u8 = 0x48; // Literal 24
const DW_OP_LIT25: u8 = 0x49; // Literal 25
const DW_OP_LIT26: u8 = 0x4A; // Literal 26
const DW_OP_LIT27: u8 = 0x4B; // Literal 27
const DW_OP_LIT28: u8 = 0x4C; // Literal 28
const DW_OP_LIT29: u8 = 0x4D; // Literal 29
const DW_OP_LIT30: u8 = 0x4E; // Literal 30
const DW_OP_LIT31: u8 = 0x4F; // Literal 31
const DW_OP_REG0: u8 = 0x50; // Contents of reg0
const DW_OP_REG1: u8 = 0x51; // Contents of reg1
const DW_OP_REG2: u8 = 0x52; // Contents of reg2
const DW_OP_REG3: u8 = 0x53; // Contents of reg3
const DW_OP_REG4: u8 = 0x54; // Contents of reg4
const DW_OP_REG5: u8 = 0x55; // Contents of reg5
const DW_OP_REG6: u8 = 0x56; // Contents of reg6
const DW_OP_REG7: u8 = 0x57; // Contents of reg7
const DW_OP_REG8: u8 = 0x58; // Contents of reg8
const DW_OP_REG9: u8 = 0x59; // Contents of reg9
const DW_OP_REG10: u8 = 0x5A; // Contents of reg10
const DW_OP_REG11: u8 = 0x5B; // Contents of reg11
const DW_OP_REG12: u8 = 0x5C; // Contents of reg12
const DW_OP_REG13: u8 = 0x5D; // Contents of reg13
const DW_OP_REG14: u8 = 0x5E; // Contents of reg14
const DW_OP_REG15: u8 = 0x5F; // Contents of reg15
const DW_OP_REG16: u8 = 0x60; // Contents of reg16
const DW_OP_REG17: u8 = 0x61; // Contents of reg17
const DW_OP_REG18: u8 = 0x62; // Contents of reg18
const DW_OP_REG19: u8 = 0x63; // Contents of reg19
const DW_OP_REG20: u8 = 0x64; // Contents of reg20
const DW_OP_REG21: u8 = 0x65; // Contents of reg21
const DW_OP_REG22: u8 = 0x66; // Contents of reg22
const DW_OP_REG23: u8 = 0x67; // Contents of reg23
const DW_OP_REG24: u8 = 0x68; // Contents of reg24
const DW_OP_REG25: u8 = 0x69; // Contents of reg25
const DW_OP_REG26: u8 = 0x6A; // Contents of reg26
const DW_OP_REG27: u8 = 0x6B; // Contents of reg27
const DW_OP_REG28: u8 = 0x6C; // Contents of reg28
const DW_OP_REG29: u8 = 0x6D; // Contents of reg29
const DW_OP_REG30: u8 = 0x6E; // Contents of reg30
const DW_OP_REG31: u8 = 0x6F; // Contents of reg31
const DW_OP_BREG0: u8 = 0x70; // base register 0 + SLEB128 offset
const DW_OP_BREG1: u8 = 0x71; // base register 1 + SLEB128 offset
const DW_OP_BREG2: u8 = 0x72; // base register 2 + SLEB128 offset
const DW_OP_BREG3: u8 = 0x73; // base register 3 + SLEB128 offset
const DW_OP_BREG4: u8 = 0x74; // base register 4 + SLEB128 offset
const DW_OP_BREG5: u8 = 0x75; // base register 5 + SLEB128 offset
const DW_OP_BREG6: u8 = 0x76; // base register 6 + SLEB128 offset
const DW_OP_BREG7: u8 = 0x77; // base register 7 + SLEB128 offset
const DW_OP_BREG8: u8 = 0x78; // base register 8 + SLEB128 offset
const DW_OP_BREG9: u8 = 0x79; // base register 9 + SLEB128 offset
const DW_OP_BREG10: u8 = 0x7A; // base register 10 + SLEB128 offset
const DW_OP_BREG11: u8 = 0x7B; // base register 11 + SLEB128 offset
const DW_OP_BREG12: u8 = 0x7C; // base register 12 + SLEB128 offset
const DW_OP_BREG13: u8 = 0x7D; // base register 13 + SLEB128 offset
const DW_OP_BREG14: u8 = 0x7E; // base register 14 + SLEB128 offset
const DW_OP_BREG15: u8 = 0x7F; // base register 15 + SLEB128 offset
const DW_OP_BREG16: u8 = 0x80; // base register 16 + SLEB128 offset
const DW_OP_BREG17: u8 = 0x81; // base register 17 + SLEB128 offset
const DW_OP_BREG18: u8 = 0x82; // base register 18 + SLEB128 offset
const DW_OP_BREG19: u8 = 0x83; // base register 19 + SLEB128 offset
const DW_OP_BREG20: u8 = 0x84; // base register 20 + SLEB128 offset
const DW_OP_BREG21: u8 = 0x85; // base register 21 + SLEB128 offset
const DW_OP_BREG22: u8 = 0x86; // base register 22 + SLEB128 offset
const DW_OP_BREG23: u8 = 0x87; // base register 23 + SLEB128 offset
const DW_OP_BREG24: u8 = 0x88; // base register 24 + SLEB128 offset
const DW_OP_BREG25: u8 = 0x89; // base register 25 + SLEB128 offset
const DW_OP_BREG26: u8 = 0x8A; // base register 26 + SLEB128 offset
const DW_OP_BREG27: u8 = 0x8B; // base register 27 + SLEB128 offset
const DW_OP_BREG28: u8 = 0x8C; // base register 28 + SLEB128 offset
const DW_OP_BREG29: u8 = 0x8D; // base register 29 + SLEB128 offset
const DW_OP_BREG30: u8 = 0x8E; // base register 30 + SLEB128 offset
const DW_OP_BREG31: u8 = 0x8F; // base register 31 + SLEB128 offset
const DW_OP_REGX: u8 = 0x90; // ULEB128 register
const DW_OP_FBREG: u8 = 0x91; // SLEB128 offset
const DW_OP_BREGX: u8 = 0x92; // ULEB128 register followed by SLEB128 offset
const DW_OP_PIECE: u8 = 0x93; // ULEB128 size of piece addressed
const DW_OP_DEREF_SIZE: u8 = 0x94; // 1-byte size of data retrieved
const DW_OP_XDEREF_SIZE: u8 = 0x95; // 1-byte size of data retrieved
const DW_OP_NOP: u8 = 0x96;
const DW_OP_PUSH_OBJECT_ADDRESS: u8 = 0x97;
const DW_OP_CALL2: u8 = 0x98; // 2-byte offset of DIE
const DW_OP_CALL4: u8 = 0x99; // 4-byte offset of DIE
const DW_OP_CALL_REF: u8 = 0x9A; // 4- or 8-byte offset of DIE
const DW_OP_LO_USER: u8 = 0xE0;
const DW_OP_APPLE_UNINIT: u8 = 0xF0;
const DW_OP_HI_USER: u8 = 0xFF;

pub fn evaluate(expression: u64, registers: &Registers, initial_stack: u64) -> u64 {
    let mut loc = expression;
    let end = expression + decode_uleb128(&mut loc, expression + 20); // 20 is a tmp guard.
    let mut stack = EvaluateStack::default();
    stack.push(initial_stack);
    while loc < end {
        let mut u1: u64;
        let mut s1: i64;
        let mut s2: i64;
        let mut reg: u32;
        let opcode = load::<u8>(loc);
        match opcode {
            DW_OP_ADDR => {
                // Push immediate address sized value.
                u1 = load::<u64>(loc);
                loc += 8;
                stack.push(u1);
            }
            DW_OP_DEREF => {
                // Pop stack, dereference, push result.
                u1 = stack.pop();
                stack.push(load::<u64>(u1));
            }
            DW_OP_CONST1U => {
                // Push immediate 1 byte value.
                u1 = load::<u8>(loc) as u64;
                loc += 1;
                stack.push(u1);
            }
            DW_OP_CONST1S => {
                // Push immediate 1 byte signed value.
                s1 = load::<i8>(loc) as i64;
                loc += 1;
                stack.push(s1 as u64);
            }
            DW_OP_CONST2U => {
                // Push immediate 2 byte value.
                u1 = load::<u16>(loc) as u64;
                loc += 2;
                stack.push(u1);
            }
            DW_OP_CONST2S => {
                // Push immediate 2 byte signed value.
                s1 = load::<i16>(loc) as i64;
                loc += 2;
                stack.push(s1 as u64);
            }
            DW_OP_CONST4U => {
                // Push immediate 4 byte value.
                u1 = load::<u32>(loc) as u64;
                loc += 4;
                stack.push(u1);
            }
            DW_OP_CONST4S => {
                // Push immediate 4 byte signed value.
                s1 = load::<i32>(loc) as i64;
                loc += 4;
                stack.push(s1 as u64);
            }
            DW_OP_CONST8U => {
                // Push immediate 8 byte value.
                u1 = load::<u64>(loc);
                loc += 8;
                stack.push(u1);
            }
            DW_OP_CONST8S => {
                // Push immediate 8 byte signed value.
                s1 = load::<i64>(loc);
                loc += 8;
                stack.push(s1 as u64);
            }
            DW_OP_CONSTU => {
                // Push immediate ULEB128 value.
                u1 = decode_uleb128(&mut loc, end);
                stack.push(u1);
            }
            DW_OP_CONSTS => {
                // Push immediate SLEB128 value.
                s1 = decode_sleb128(&mut loc, end);
                stack.push(s1 as u64);
            }
            DW_OP_DUP => {
                // Push top of stack.
                u1 = stack.pop();
                stack.push(u1);
            }
            DW_OP_DROP => {
                // Pop.
                stack.pop();
            }
            DW_OP_OVER => {
                // Dup second.
                u1 = stack.top(1);
                stack.push(u1);
            }
            DW_OP_PICK => {
                // Pick from.
                reg = load::<u8>(loc) as u32;
                loc += 1;
                u1 = stack.top(reg as usize);
                stack.push(u1);
            }
            DW_OP_SWAP => {
                // Swap top two.
                u1 = stack.top(0);
                *stack.top_mut(0) = stack.top(1);
                *stack.top_mut(1) = u1;
            }
            DW_OP_ROT => {
                // Rotate top three.
                u1 = stack.top(0);
                *stack.top_mut(0) = stack.top(1);
                *stack.top_mut(1) = stack.top(2);
                *stack.top_mut(2) = u1;
            }
            DW_OP_XDEREF => {
                // Pop stack, dereference, push result.
                u1 = stack.pop();
                *stack.top_mut(0) = load::<u64>(u1);
            }
            DW_OP_ABS => {
                s1 = stack.top(0) as i64;
                if s1 < 0 {
                    *stack.top_mut(0) = (-s1) as u64;
                }
            }
            DW_OP_AND => {
                u1 = stack.pop();
                *stack.top_mut(0) &= u1;
            }
            DW_OP_DIV => {
                s1 = stack.pop() as i64;
                s2 = stack.top(0) as i64;
                *stack.top_mut(0) = (s2 / s1) as u64;
            }
            DW_OP_MINUS => {
                u1 = stack.pop();
                *stack.top_mut(0) -= u1;
            }
            DW_OP_MOD => {
                s1 = stack.pop() as i64;
                s2 = stack.top(0) as i64;
                *stack.top_mut(0) = (s2 % s1) as u64;
            }
            DW_OP_MUL => {
                s1 = stack.pop() as i64;
                s2 = stack.top(0) as i64;
                *stack.top_mut(0) = (s2 * s1) as u64;
            }
            DW_OP_NEG => {
                *stack.top_mut(0) = 0 - stack.top(0);
            }
            DW_OP_NOT => {
                s1 = stack.top(0) as i64;
                *stack.top_mut(0) = (!s1) as u64
            }
            DW_OP_OR => {
                u1 = stack.pop();
                *stack.top_mut(0) |= u1;
            }
            DW_OP_PLUS => {
                u1 = stack.pop();
                *stack.top_mut(0) += u1;
            }
            DW_OP_PLUS_UCONST => {
                u1 = decode_uleb128(&mut loc, end);
                *stack.top_mut(0) += u1;
            }
            DW_OP_SHL => {
                u1 = stack.pop();
                *stack.top_mut(0) <<= u1;
            }
            DW_OP_SHR => {
                u1 = stack.pop();
                *stack.top_mut(0) >>= u1;
            }
            DW_OP_SHRA => {
                u1 = stack.pop();
                s1 = stack.top(0) as i64;
                *stack.top_mut(0) = (s1 >> u1) as u64;
            }
            DW_OP_XOR => {
                u1 = stack.pop();
                *stack.top_mut(0) ^= u1;
            }
            DW_OP_SKIP => {
                s1 = load::<i16>(loc) as i64;
                loc += 2;
                loc = ((loc as i64) + s1) as u64;
            }
            DW_OP_BRA => {
                s1 = load::<i16>(loc) as i64;
                loc += 2;
                if stack.pop() != 0 {
                    loc = ((loc as i64) + s1) as u64;
                }
            }
            DW_OP_EQ => {
                u1 = stack.pop();
                *stack.top_mut(0) = if stack.top(0) == u1 { 1 } else { 0 };
            }
            DW_OP_GE => {
                u1 = stack.pop();
                *stack.top_mut(0) = if stack.top(0) >= u1 { 1 } else { 0 };
            }
            DW_OP_GT => {
                u1 = stack.pop();
                *stack.top_mut(0) = if stack.top(0) > u1 { 1 } else { 0 };
            }
            DW_OP_LE => {
                u1 = stack.pop();
                *stack.top_mut(0) = if stack.top(0) <= u1 { 1 } else { 0 };
            }
            DW_OP_LT => {
                u1 = stack.pop();
                *stack.top_mut(0) = if stack.top(0) < u1 { 1 } else { 0 };
            }
            DW_OP_NE => {
                u1 = stack.pop();
                *stack.top_mut(0) = if stack.top(0) != u1 { 1 } else { 0 };
            }
            DW_OP_LIT0..=DW_OP_LIT31 => {
                u1 = (opcode - DW_OP_LIT0) as u64;
                stack.push(u1);
            }
            DW_OP_REG0..=DW_OP_REG31 => {
                reg = (opcode - DW_OP_REG0) as u32;
                stack.push(registers[reg as usize]);
            }
            DW_OP_REGX => {
                reg = decode_uleb128(&mut loc, end) as u32;
                stack.push(registers[reg as usize]);
            }
            DW_OP_BREG0..=DW_OP_BREG31 => {
                reg = (opcode - DW_OP_BREG0) as u32;
                s1 = decode_sleb128(&mut loc, end);
                s1 += registers[reg as usize] as i64;
                stack.push(s1 as u64);
            }
            DW_OP_BREGX => {
                reg = decode_uleb128(&mut loc, end) as u32;
                s1 = decode_sleb128(&mut loc, end);
                s1 += registers[reg as usize] as i64;
                stack.push(s1 as u64);
            }
            DW_OP_DEREF_SIZE => {
                u1 = stack.pop();
                match load::<u8>(loc) {
                    1 => u1 = load::<u8>(u1) as u64,
                    2 => u1 = load::<u16>(u1) as u64,
                    4 => u1 = load::<u32>(u1) as u64,
                    8 => u1 = load::<u64>(u1),
                    _ => unreachable!(),
                }
                loc += 1;
                stack.push(u1);
            }
            _ => unimplemented!(),
        }
    }
    stack.top(0)
}

struct EvaluateStack {
    len: usize,
    stack: [u64; 100],
}

impl Default for EvaluateStack {
    fn default() -> Self {
        Self {
            len: 0,
            stack: [0; 100],
        }
    }
}

impl Index<usize> for EvaluateStack {
    type Output = u64;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.stack[index]
    }
}

impl IndexMut<usize> for EvaluateStack {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.stack[index]
    }
}

impl EvaluateStack {
    #[inline]
    fn push(&mut self, v: u64) {
        self.stack[self.len] = v;
        self.len += 1;
    }

    #[inline]
    fn pop(&mut self) -> u64 {
        let v = self.stack[self.len - 1];
        self.len -= 1;
        v
    }

    #[inline]
    fn top(&self, n: usize) -> u64 {
        self.stack[self.len - (n + 1)]
    }

    #[inline]
    fn top_mut(&mut self, n: usize) -> &mut u64 {
        &mut self.stack[self.len - (n + 1)]
    }
}
