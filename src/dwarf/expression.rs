use crate::dwarf::consts::*;
use crate::dwarf::encoding::{decode_sleb128, decode_uleb128};
use crate::dwarf::{load_with_protect as load, DwarfError};
use crate::registers::Registers;
use std::ops::{Index, IndexMut};

pub fn evaluate(expression: u64, registers: &Registers, initial_stack: u64) -> Result<u64, DwarfError> {
    let mut loc = expression;
    let end = expression + decode_uleb128(&mut loc, expression + 20)?; // 20 is a tmp guard.
    let mut stack = EvaluateStack::default();
    stack.push(initial_stack);
    while loc < end {
        let mut u1: u64;
        let mut s1: i64;
        let s2: i64; // temporarily remove `mut` to avoid warning
        let reg: u32; // ditto
        let opcode = load::<u8>(loc)?;
        match opcode {
            DW_OP_ADDR => {
                // Push immediate address sized value.
                u1 = load::<u64>(loc)?;
                loc += 8;
                stack.push(u1);
            }
            DW_OP_DEREF => {
                // Pop stack, dereference, push result.
                u1 = stack.pop();
                stack.push(load::<u64>(u1)?);
            }
            DW_OP_CONST1U => {
                // Push immediate 1 byte value.
                u1 = load::<u8>(loc)? as u64;
                loc += 1;
                stack.push(u1);
            }
            DW_OP_CONST1S => {
                // Push immediate 1 byte signed value.
                s1 = load::<i8>(loc)? as i64;
                loc += 1;
                stack.push(s1 as u64);
            }
            DW_OP_CONST2U => {
                // Push immediate 2 byte value.
                u1 = load::<u16>(loc)? as u64;
                loc += 2;
                stack.push(u1);
            }
            DW_OP_CONST2S => {
                // Push immediate 2 byte signed value.
                s1 = load::<i16>(loc)? as i64;
                loc += 2;
                stack.push(s1 as u64);
            }
            DW_OP_CONST4U => {
                // Push immediate 4 byte value.
                u1 = load::<u32>(loc)? as u64;
                loc += 4;
                stack.push(u1);
            }
            DW_OP_CONST4S => {
                // Push immediate 4 byte signed value.
                s1 = load::<i32>(loc)? as i64;
                loc += 4;
                stack.push(s1 as u64);
            }
            DW_OP_CONST8U => {
                // Push immediate 8 byte value.
                u1 = load::<u64>(loc)?;
                loc += 8;
                stack.push(u1);
            }
            DW_OP_CONST8S => {
                // Push immediate 8 byte signed value.
                s1 = load::<i64>(loc)?;
                loc += 8;
                stack.push(s1 as u64);
            }
            DW_OP_CONSTU => {
                // Push immediate ULEB128 value.
                u1 = decode_uleb128(&mut loc, end)?;
                stack.push(u1);
            }
            DW_OP_CONSTS => {
                // Push immediate SLEB128 value.
                s1 = decode_sleb128(&mut loc, end)?;
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
                reg = load::<u8>(loc)? as u32;
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
                *stack.top_mut(0) = load::<u64>(u1)?;
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
                u1 = decode_uleb128(&mut loc, end)?;
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
                s1 = load::<i16>(loc)? as i64;
                loc += 2;
                loc = ((loc as i64) + s1) as u64;
            }
            DW_OP_BRA => {
                s1 = load::<i16>(loc)? as i64;
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
                reg = decode_uleb128(&mut loc, end)? as u32;
                stack.push(registers[reg as usize]);
            }
            DW_OP_BREG0..=DW_OP_BREG31 => {
                reg = (opcode - DW_OP_BREG0) as u32;
                s1 = decode_sleb128(&mut loc, end)?;
                s1 += registers[reg as usize] as i64;
                stack.push(s1 as u64);
            }
            DW_OP_BREGX => {
                reg = decode_uleb128(&mut loc, end)? as u32;
                s1 = decode_sleb128(&mut loc, end)?;
                s1 += registers[reg as usize] as i64;
                stack.push(s1 as u64);
            }
            DW_OP_DEREF_SIZE => {
                u1 = stack.pop();
                match load::<u8>(loc)? {
                    1 => u1 = load::<u8>(u1)? as u64,
                    2 => u1 = load::<u16>(u1)? as u64,
                    4 => u1 = load::<u32>(u1)? as u64,
                    8 => u1 = load::<u64>(u1)?,
                    v => return Err(DwarfError::InvalidExpressionDerefSize(v)),
                }
                loc += 1;
                stack.push(u1);
            }
            v => return Err(DwarfError::InvalidExpression(v)),
        }
    }
    Ok(stack.top(0))
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
