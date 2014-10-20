// Copyright (C) 2014 The 6502-rs Developers
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
// 3. Neither the names of the copyright holders nor the names of any
//    contributors may be used to endorse or promote products derived from this
//    software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

use log;

use std;

use address::{AddressDiff};
use instruction;
use instruction::{DecodedInstr};
use memory::Memory;
use registers::{ Registers, Status, StatusArgs };
use registers::{ PS_NEGATIVE, PS_OVERFLOW, PS_ZERO, PS_CARRY };

pub struct Machine {
    pub registers: Registers,
    pub memory:    Memory
}

impl Machine {
    pub fn new() -> Machine {
    	Machine{
    	    registers: Registers::new(),
    	    memory:    Memory::new()
    	}
    }
    
    pub fn reset(&mut self) {
    	*self = Machine::new();
    }

    pub fn fetch_next_and_decode(&mut self) -> Option<DecodedInstr> {
        let x: u8 = self.memory.get_byte(self.registers.program_counter);

        match instruction::OPCODES[x as uint] {
            Some((instr, am)) => {
                let extra_bytes = am.extra_bytes();
                let num_bytes = AddressDiff(1) + extra_bytes;

                let data_start = self.registers.program_counter
                               + AddressDiff(1);

                let slice = self.memory.get_slice(data_start, extra_bytes);
                let am_out = am.process(self, slice);

                // Increment program counter
                self.registers.program_counter =
                    self.registers.program_counter + num_bytes;

                Some((instr, am_out))
            }
            _ => None
        }
    }

    pub fn execute_instruction(&mut self, decoded_instr: DecodedInstr) {
        match decoded_instr {
            (instruction::ADC, instruction::UseImmediate(val)) => {
                log!(log::DEBUG, "add with carry immediate: {}", val);
                self.add_with_carry(val as i8);
            },
            (instruction::ADC, instruction::UseAddress(addr)) => {
                let val = self.memory.get_byte(addr) as i8;
                log!(log::DEBUG, "add with carry. address: {}. value: {}",
                                 addr, val);
                self.add_with_carry(val);
            },

            (instruction::DEX, instruction::UseImplied) => {
                self.dec_x();
            }

            (instruction::LDA, instruction::UseImmediate(val)) => {
                log!(log::DEBUG, "load A immediate: {}", val);
                self.load_accumulator(val as i8);
            },
            (instruction::LDA, instruction::UseAddress(addr)) => {
                let val = self.memory.get_byte(addr);
                log!(log::DEBUG, "load A. address: {}. value: {}", addr, val);
                self.load_accumulator(val as i8);
            },

            (instruction::LDX, instruction::UseImmediate(val)) => {
                log!(log::DEBUG, "load X immediate: {}", val);
                self.load_x_register(val as i8);
            },
            (instruction::LDX, instruction::UseAddress(addr)) => {
                let val = self.memory.get_byte(addr);
                log!(log::DEBUG, "load X. address: {}. value: {}", addr, val);
                self.load_x_register(val as i8);
            },

            (instruction::LDY, instruction::UseImmediate(val)) => {
                log!(log::DEBUG, "load Y immediate: {}", val);
                self.load_y_register(val as i8);
            },
            (instruction::LDY, instruction::UseAddress(addr)) => {
                let val = self.memory.get_byte(addr);
                log!(log::DEBUG, "load Y. address: {}. value: {}", addr, val);
                self.load_y_register(val as i8);
            },

            (instruction::NOP, _) => {
                log!(log::DEBUG, "nop instr");
            },
            (_, _) => {
                log!(log::DEBUG, "attempting to execute unimplemented \
                                  instruction");
            },
        };
    }

    pub fn run(&mut self) {
        loop {
            if let Some(decoded_instr) = self.fetch_next_and_decode() {
                self.execute_instruction(decoded_instr);
            } else {
                break
            }
        }
    }

    fn load_register_with_flags(register: &mut i8,
                                status: &mut Status,
                                value: i8) {
        *register = value;

        let is_zero = value == 0;
        let is_negative = value < 0;

        status.set_with_mask(
            PS_ZERO | PS_NEGATIVE,
            Status::new(StatusArgs { zero: is_zero,
                                     negative: is_negative,
                                     ..StatusArgs::none() } ));
    }

    pub fn load_x_register(&mut self, value: i8) {
        Machine::load_register_with_flags(&mut self.registers.index_x,
                                          &mut self.registers.status,
                                          value);
    }

    pub fn load_y_register(&mut self, value: i8) {
        Machine::load_register_with_flags(&mut self.registers.index_y,
                                          &mut self.registers.status,
                                          value);
    }

    pub fn load_accumulator(&mut self, value: i8) {
        Machine::load_register_with_flags(&mut self.registers.accumulator,
                                          &mut self.registers.status,
                                          value);
    }

    // TODO akeeton: Implement binary-coded decimal.
    pub fn add_with_carry(&mut self, value: i8) {
        let a_before: i8 = self.registers.accumulator;
        let c_before: i8 = self.registers.status.get_carry();
        let a_after: i8 = a_before + c_before + value;

        debug_assert_eq!(a_after as u8, a_before as u8 + c_before as u8
                                        + value as u8);

        let did_carry = (a_after as u8) < (a_before as u8);

        let did_overflow   =
        	   (a_before < 0 && value < 0 && a_after >= 0)
        	|| (a_before > 0 && value > 0 && a_after <= 0);

        let mask = PS_CARRY | PS_OVERFLOW;

        self.registers.status.set_with_mask(mask,
            Status::new(StatusArgs { carry: did_carry,
                                     overflow: did_overflow,
                                     ..StatusArgs::none() } ));

        self.load_accumulator(a_after);

        log!(log::DEBUG, "accumulator: {}", self.registers.accumulator);
    }

    pub fn dec_x(&mut self) {
        let val = self.registers.index_x;
        self.load_x_register(val - 1);
    }
}

impl std::fmt::Show for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Machine Dump:\n\nAccumulator: {}",
               self.registers.accumulator)
    }
}

#[test]
fn add_with_carry_test() {

    let mut machine = Machine::new();

    machine.add_with_carry(1);
    assert_eq!(machine.registers.accumulator, 1);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), false);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.add_with_carry(-1);
    assert_eq!(machine.registers.accumulator, 0);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    true);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     true);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), false);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.add_with_carry(1);
    assert_eq!(machine.registers.accumulator, 2);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), false);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);
    
    let mut machine = Machine::new();

    machine.add_with_carry(127);
    assert_eq!(machine.registers.accumulator, 127);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), false);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.add_with_carry(-127);
    assert_eq!(machine.registers.accumulator, 0);
    assert_eq!(machine.registers.status.contains(PS_CARRY),     true);
    assert_eq!(machine.registers.status.contains(PS_ZERO),      true);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), false);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.registers.status.remove(PS_CARRY);
    machine.add_with_carry(-128);
    assert_eq!(machine.registers.accumulator, -128);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE),  true);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.add_with_carry(127);
    assert_eq!(machine.registers.accumulator, -1);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE),  true);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    let mut machine = Machine::new();

    machine.add_with_carry(127);
    assert_eq!(machine.registers.accumulator, 127);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), false);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.add_with_carry(1);
    assert_eq!(machine.registers.accumulator, -128);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE),  true);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW),  true);
}

#[test]
fn dec_x_test() {
    let mut machine = Machine::new();

    machine.dec_x();
    assert_eq!(machine.registers.index_x, -1);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), true);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.dec_x();
    assert_eq!(machine.registers.index_x, -2);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), true);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.load_x_register(5);
    machine.dec_x();
    assert_eq!(machine.registers.index_x, 4);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), false);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.dec_x();
    machine.dec_x();
    machine.dec_x();
    machine.dec_x();

    assert_eq!(machine.registers.index_x, 0);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     true);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), false);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);

    machine.dec_x();
    assert_eq!(machine.registers.index_x, -1);
    assert_eq!(machine.registers.status.contains(PS_CARRY),    false);
    assert_eq!(machine.registers.status.contains(PS_ZERO),     false);
    assert_eq!(machine.registers.status.contains(PS_NEGATIVE), true);
    assert_eq!(machine.registers.status.contains(PS_OVERFLOW), false);
}