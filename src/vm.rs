use enum_map::EnumMap;

use num_traits::FromPrimitive;

use byteorder::{BigEndian, ReadBytesExt};

use std::sync::{Arc, Mutex};

use crate::bits::{
    sign_extend, ConditionFlags, DiagnosticStatus, MemoryMappedRegister, Opcode, Register, TrapCode,
};

#[derive(Debug)]
pub struct VirtualMachine {
    memory: Box<[u16]>,
    registers: EnumMap<Register, u16>,
    running: bool,
    diagnostic_mutex: Option<Arc<Mutex<DiagnosticStatus>>>,
}

impl VirtualMachine {
    pub fn with_memory(amount: usize) -> Self {
        VirtualMachine {
            memory: vec![0; amount].into_boxed_slice(),
            registers: enum_map! {
                _ => 0,
            },
            running: false,
            diagnostic_mutex: None,
        }
    }

    pub fn add_diagnostic_mutex(&mut self, sender: Arc<Mutex<DiagnosticStatus>>) {
        self.diagnostic_mutex = Some(sender);
    }

    pub fn read_image(&mut self, image: &[u8]) {
        use byteorder::ByteOrder;
        let origin = BigEndian::read_u16(&image[0..=1]);
        debug!("ORIGIN: {:x}", origin);
        for (offset, word) in image
            .chunks(2)
            .skip(1)
            .filter_map(|mut x| x.read_u16::<BigEndian>().ok())
            .enumerate()
        {
            self.mem_write(origin + offset as u16, word);
        }
        //self.disassemble_region(origin as usize, image.len(), true);
        /*let meme = &self.memory[origin as usize..(origin as usize+image.len())];
        hexdump::hexdump(unsafe {
            std::slice::from_raw_parts(
                meme.as_ptr() as *const u8,
                meme.len() * std::mem::size_of::<u16>()
            )
        });*/
    }

    pub fn disassemble_region(&self, start: usize, length: usize, print_address: bool) {
        if print_address {
            for (address, instr) in self.memory[start..=start + length]
                .iter()
                .enumerate()
                .map(|(x, y)| (x + start, y))
            {
                println!(
                    "{:04X} | {:016b} | {}",
                    address,
                    instr,
                    crate::disasm::disassemble_instruction(*instr)
                        .as_ref()
                        .map(|x| &**x)
                        .unwrap_or("BAD OPCODE")
                )
            }
        } else {
            println!(
                "{}",
                crate::disasm::disassemble_program(&self.memory[start..=start + length])
            );
        }
    }

    pub fn memory_dump(&self) {
        hexdump::hexdump(unsafe {
            std::slice::from_raw_parts(
                self.memory.as_ptr() as *const u8,
                self.memory.len() * std::mem::size_of::<u16>(),
            )
        });
    }

    pub fn read_image_file(&mut self, image_file: &mut std::fs::File) {
        use std::io::Read;
        let mut buf = Vec::new();
        image_file.read_to_end(&mut buf).unwrap();
        self.read_image(&buf);
    }

    fn send_diagnostics(&self) {
        use std::iter::FromIterator;
        if let Some(arc) = &self.diagnostic_mutex {
            if let Ok(ref mut mutex) = arc.try_lock() {
                let requested_memory_range = mutex.memory_view_range;
                let (range_start, range_length) = requested_memory_range;
                mutex.registers = self.registers;
                mutex.memory_view = Vec::from_iter(
                    self.memory[range_start..=range_start + range_length]
                        .iter()
                        .cloned(),
                );
            }
        };
    }

    pub fn run(&mut self) {
        self.running = true;
        self.registers[Register::PC] = 0x3000;
        while self.running {
            self.send_diagnostics();
            let instr = self.mem_read(self.registers[Register::PC]);
            self.registers[Register::PC] += 1;
            let op = instr >> 12;
            trace!(
                "PC: {:x} INSTR: {:016b} OP: {:04b}",
                self.registers[Register::PC] - 1,
                instr,
                op
            );
            //::std::thread::sleep(::std::time::Duration::from_millis(500));
            match Opcode::from_u16(op) {
                Some(Opcode::ADD) => self.op_add(instr),
                Some(Opcode::AND) => self.op_and(instr),
                Some(Opcode::BR) => self.op_br(instr),
                Some(Opcode::JMP) => self.op_jmp(instr),
                Some(Opcode::JSR) => self.op_jsr(instr),
                Some(Opcode::LD) => self.op_ld(instr),
                Some(Opcode::LDI) => self.op_ldi(instr),
                Some(Opcode::LDR) => self.op_ldr(instr),
                Some(Opcode::LEA) => self.op_lea(instr),
                Some(Opcode::NOT) => self.op_not(instr),
                Some(Opcode::RTI) => self.op_rti(instr),
                Some(Opcode::ST) => self.op_st(instr),
                Some(Opcode::STI) => self.op_sti(instr),
                Some(Opcode::STR) => self.op_str(instr),
                Some(Opcode::TRAP) => self.op_trap(instr),
                _ => self.bad_opcode(),
            }
        }
    }

    fn bad_opcode(&mut self) {
        trace!("BAD");
        println!("bad opcode");
    }

    fn op_add(&mut self, instr: u16) {
        // destination register (DR)
        let r0: u16 = (instr >> 9) & 0x7;
        // first operand (SR1)
        let r1: u16 = (instr >> 6) & 0x7;
        // whether we are in immediate mode
        let imm_flag = (instr >> 5) & 0x1;
        if imm_flag == 1 {
            let imm5 = sign_extend(instr & 0x1F, 5);
            trace!("ADD DR: {} SR1: {} IMM: {}", r0, r1, imm5);
            self.registers[Register::from_u16(r0)] =
                self.registers[Register::from_u16(r1)].wrapping_add(imm5);
        } else {
            let r2 = instr & 0x7;
            trace!("ADD DR: {} SR1: {} R2: {}", r0, r1, r2);
            self.registers[Register::from_u16(r0)] = self.registers[Register::from_u16(r1)]
                .wrapping_add(self.registers[Register::from_u16(r2)]);
        }
        self.update_flags(Register::from_u16(r0));
    }

    fn op_and(&mut self, instr: u16) {
        // destination register (DR)
        let r0: u16 = (instr >> 9) & 0x7;
        // first operand (SR1)
        let r1: u16 = (instr >> 6) & 0x7;
        // whether we are in immediate mode
        let imm_flag = (instr >> 5) & 0x1;

        if imm_flag == 1 {
            let imm5 = sign_extend(instr & 0x1F, 5);
            trace!("AND R0: {} R1: {} IMM: {}", r0, r1, imm5);
            self.registers[Register::from_u16(r0)] = self.registers[Register::from_u16(r1)] & imm5;
        } else {
            let r2 = instr & 0x7;
            trace!("AND R0: {} R1: {} R2: {}", r0, r1, r2);
            self.registers[Register::from_u16(r0)] =
                self.registers[Register::from_u16(r1)] & self.registers[Register::from_u16(r2)];
        }
    }

    fn op_br(&mut self, instr: u16) {
        let cond_flag: u16 = (instr >> 9) & 0x7;
        let n = if cond_flag & ConditionFlags::NEG as u16 == 1 {
            "n"
        } else {
            ""
        };
        let z = if cond_flag & ConditionFlags::ZRO as u16 == 1 {
            "z"
        } else {
            ""
        };
        let p = if cond_flag & ConditionFlags::POS as u16 == 1 {
            "p"
        } else {
            ""
        };
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        trace!("BR{}{}{} OFFSET: {}", n, z, p, pc_offset);
        if cond_flag & self.registers[Register::COND] != 0 {
            self.registers[Register::PC] = self.registers[Register::PC].wrapping_add(pc_offset);
        }
    }

    fn op_jmp(&mut self, instr: u16) {
        let r0 = (instr >> 6) & 0x7;
        let value = self.registers[Register::from_u16(r0)];
        trace!("JMP BASER: {} VAL: {:b}", r0, value);
        self.registers[Register::PC] = value;
    }

    fn op_jsr(&mut self, instr: u16) {
        trace!("JSR");
        self.registers[Register::R7] = self.registers[Register::PC];
        let long_flag = (instr >> 11) & 0x1;
        if long_flag == 0 {
            let base_r = (instr >> 6) & 0x7;
            self.registers[Register::PC] = self.registers[Register::from_u16(base_r)];
        } else {
            self.registers[Register::PC] =
                self.registers[Register::PC].wrapping_add(sign_extend(instr & 0x7ff, 11));
        }
    }

    fn op_ld(&mut self, instr: u16) {
        let dr: u16 = (instr >> 9) & 0x7;
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        trace!("LD DR: {} OFFSET: {}", dr, pc_offset);
        self.registers[Register::from_u16(dr)] =
            self.mem_read(self.registers[Register::PC].wrapping_add(pc_offset));
        self.update_flags(Register::from_u16(dr));
    }

    fn op_ldi(&mut self, instr: u16) {
        trace!("LDI");
        // destination register (DR)
        let r0: u16 = (instr >> 9) & 0x7;
        // PCoffset 9
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        // add pc_offset to the current PC, look at that memory location to get the final address

        let thing1 = self.mem_read(self.registers[Register::PC] + pc_offset);
        self.registers[Register::from_u16(r0)] = self.mem_read(thing1);
        self.update_flags(Register::from_u16(r0));
    }

    fn op_ldr(&mut self, instr: u16) {
        trace!("LDR");
        let dr: u16 = (instr >> 9) & 0x7;
        let base_r = self.registers[Register::from_u16((instr >> 6) & 0x7)];
        let offset = sign_extend(instr & 0x3f, 5);
        self.registers[Register::from_u16(dr)] = self.mem_read(base_r + offset);
        self.update_flags(Register::from_u16(dr));
    }

    fn op_lea(&mut self, instr: u16) {
        trace!("LEA");
        let dr: u16 = (instr >> 9) & 0x7;
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        self.registers[Register::from_u16(dr)] = self.registers[Register::PC] + pc_offset;
        self.update_flags(Register::from_u16(dr));
    }

    fn op_not(&mut self, instr: u16) {
        trace!("NOT");
        let dr: u16 = (instr >> 9) & 0x7;
        let sr: u16 = (instr >> 6) & 0x7;
        self.registers[Register::from_u16(dr)] = !self.registers[Register::from_u16(sr)];
        self.update_flags(Register::from_u16(dr));
    }

    fn op_rti(&mut self, _instr: u16) {
        trace!("RTI");
        if (self.registers[Register::PSR] >> 15 & 1) == 0 {
            self.registers[Register::PC] = self.mem_read(self.registers[Register::R6]); // R6 is the SSP
            self.registers[Register::R6] += 1;
            let temp = self.mem_read(self.registers[Register::R6]);
            self.registers[Register::R6] += 1;
            self.registers[Register::PSR] = temp;
        // the privilege mode and condition codes of the interrupted process are restored
        } else {
            // initiate a privilege mode exception
        }
    }

    fn op_st(&mut self, instr: u16) {
        trace!("ST");
        let sr = self.registers[Register::from_u16((instr >> 9) & 0x7)];
        let pc_offset = sign_extend(instr & 0x1ff, 9);
        self.mem_write(self.registers[Register::PC].wrapping_add(pc_offset), sr);
    }

    fn op_sti(&mut self, instr: u16) {
        trace!("STI");
        let sr = (instr >> 9) & 0x7;
        let pc_offset = sign_extend(instr & 0x1ff, 9);

        let thing = self.registers[Register::PC] + pc_offset;
        let thing = self.mem_read(thing);

        self.mem_write(thing, self.registers[Register::from_u16(sr)]);
    }

    fn op_str(&mut self, instr: u16) {
        let sr = (instr >> 9) & 0x7;
        let base_r = (instr >> 6) & 0x7;
        let offset = sign_extend(instr & 0x3F, 6);
        let base_r_contents = self.registers[Register::from_u16(base_r)];
        let data_to_write = self.registers[Register::from_u16(sr)];

        trace!("STR SR: {} BASER: {} OFFSET: {}", sr, base_r, offset);

        self.mem_write(base_r_contents.wrapping_add(offset), data_to_write);
    }

    fn op_trap(&mut self, instr: u16) {
        trace!("TRAP");
        let trapvect = instr & 0xff;
        match TrapCode::from_u16(trapvect) {
            Some(TrapCode::GetC) => self.trap_getc(),
            Some(TrapCode::Out) => self.trap_out(),
            Some(TrapCode::Puts) => self.trap_puts(),
            Some(TrapCode::In) => self.trap_in(),
            Some(TrapCode::PutSp) => self.trap_putsp(),
            Some(TrapCode::Halt) => self.trap_halt(),
            _ => self.bad_opcode(),
        };
    }

    fn trap_getc(&mut self) {
        trace!("GETC");
        use std::io::Read;
        self.registers[Register::R0] =
            u16::from(::std::io::stdin().lock().bytes().next().unwrap().unwrap());
    }

    fn trap_out(&self) {
        trace!("OUT");
        let r0_contents = self.registers[Register::R0];
        let bottom_half = r0_contents as u8;
        print!("{}", bottom_half as char);
    }

    fn trap_puts(&self) {
        trace!("PUTS");
        let r0_contents = self.registers[Register::R0];
        let string_bytes = self.memory[r0_contents as usize..]
            .iter()
            .map(|word| *word as u8)
            .take_while(|byte| *byte != 0)
            .collect::<Vec<u8>>();
        print!("{}", String::from_utf8_lossy(&string_bytes));
    }

    fn trap_in(&mut self) {
        trace!("IN");
        print!("IN: ");
        self.trap_getc();
    }

    fn trap_putsp(&mut self) {
        trace!("PUTSP");
        let mut index = self.registers[Register::R0];
        loop {
            let word = self.mem_read(index);
            let first_char = word as u8;
            if first_char == 0 {
                break;
            }
            print!("{}", first_char as char);
            let second_char = (word >> 8) as u8;
            if second_char == 0 {
                break;
            }
            print!("{}", second_char);
            index += 1;
        }
    }

    fn trap_halt(&mut self) {
        trace!("HALT");
        println!("HALTING");
        self.running = false;
    }

    fn mem_write(&mut self, addr: u16, val: u16) {
        self.memory[addr as usize] = val;
    }

    fn check_key(&self) -> bool {
        true
    }

    fn mem_read(&mut self, addr: u16) -> u16 {
        use std::io::Read;
        if addr == MemoryMappedRegister::KBSR as u16 {
            if self.check_key() {
                self.memory[MemoryMappedRegister::KBSR as usize] = 1 << 15;
                self.memory[MemoryMappedRegister::KBDR as usize] =
                    u16::from(::std::io::stdin().lock().bytes().next().unwrap().unwrap());
            } else {
                self.memory[MemoryMappedRegister::KBSR as usize] = 0;
            }
        }
        self.memory[addr as usize]
    }

    fn update_flags(&mut self, reg: Register) {
        self.registers[Register::COND] = match self.registers[reg] {
            0 => ConditionFlags::ZRO,
            x if (x >> 15) == 1 => {
                // a 1 in the left-most bit indicates negative
                ConditionFlags::NEG
            }
            _ => ConditionFlags::POS,
        } as u16;
    }
}
