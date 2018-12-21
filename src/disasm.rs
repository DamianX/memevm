use crate::bits::{sign_extend, Opcode};
use num_traits::FromPrimitive;

pub fn disassemble_program(program: &[u16]) -> String {
    let mut output = String::new();
    for instr in program {
        output.push_str(
            disassemble_instruction(*instr)
                .as_ref()
                .map(|x| &**x)
                .unwrap_or("BAD OPCODE"),
        );
        output.push('\n');
    }
    output
}

pub fn disassemble_instruction(instr: u16) -> Option<String> {
    let op = instr >> 12;
    match Opcode::from_u16(op) {
        Some(Opcode::ADD) => {
            let r0: u16 = (instr >> 9) & 0x7;
            let sr1: u16 = (instr >> 6) & 0x7;
            let imm_flag = (instr >> 5) & 0x1;
            if imm_flag == 1 {
                let imm5 = sign_extend(instr & 0x1F, 5) as i16;
                Some(format!("ADD R{}, R{}, #{}", r0, sr1, imm5))
            } else {
                let sr2 = instr & 0x7;
                Some(format!("ADD R{}, R{}, R{}", r0, sr1, sr2))
            }
        }
        Some(Opcode::AND) => {
            let r0: u16 = (instr >> 9) & 0x7;
            let sr1: u16 = (instr >> 6) & 0x7;
            let imm_flag = (instr >> 5) & 0x1;
            if imm_flag == 1 {
                let imm5 = sign_extend(instr & 0x1F, 5) as i16;
                Some(format!("AND R{}, R{}, #{}", r0, sr1, imm5))
            } else {
                let sr2 = instr & 0x7;
                Some(format!("AND R{}, R{}, R{}", r0, sr1, sr2))
            }
        }
        Some(Opcode::BR) => {
            use crate::bits::ConditionFlags;
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
            let pc_offset = sign_extend(instr & 0x1ff, 9) as i16;
            Some(format!("BR{}{}{} #{}", n, z, p, pc_offset))
        }
        Some(Opcode::JMP) => {
            let r0 = (instr >> 6) & 0x7;
            Some(format!("JMP R{}", r0))
        }
        Some(Opcode::JSR) => {
            let long_flag = (instr >> 11) & 0x1;
            if long_flag == 0 {
                let base_r = (instr >> 6) & 0x7;
                Some(format!("JSRR R{}", base_r))
            } else {
                let pc_offset = sign_extend(instr & 0x7ff, 11) as i16;
                Some(format!("JSR 0x{:04X}", pc_offset))
            }
        }
        Some(Opcode::LD) => {
            let dr = (instr >> 9) & 0x7;
            let pc_offset = sign_extend(instr & 0x1ff, 9) as i16;
            Some(format!("LD R{}, #{}", dr, pc_offset))
        }
        Some(Opcode::LDI) => {
            let dr = (instr >> 9) & 0x7;
            let pc_offset = sign_extend(instr & 0x1ff, 9) as i16;

            Some(format!("LD R{}, #{}", dr, pc_offset))
        }
        Some(Opcode::LDR) => {
            let dr = (instr >> 9) & 0x7;
            let base_r = (instr >> 6) & 0x7;
            let offset = sign_extend(instr & 0x3f, 5) as i16;

            Some(format!("LDR R{}, R{}, #{}", dr, base_r, offset))
        }
        Some(Opcode::LEA) => {
            let dr = (instr >> 9) & 0x7;
            let pc_offset = sign_extend(instr & 0x1ff, 9) as i16;

            Some(format!("LEA R{}, #{}", dr, pc_offset))
        }
        Some(Opcode::NOT) => {
            let dr = (instr >> 9) & 0x7;
            let sr = (instr >> 6) & 0x7;

            Some(format!("NOT R{}, R{}", dr, sr))
        }
        Some(Opcode::RTI) => Some("RTI".to_owned()),
        Some(Opcode::ST) => {
            let sr = (instr >> 9) & 0x7;
            let pc_offset = sign_extend(instr & 0x1ff, 9) as i16;
            Some(format!("ST R{}, #{}", sr, pc_offset))
        }
        Some(Opcode::STI) => {
            let sr = (instr >> 9) & 0x7;
            let pc_offset = sign_extend(instr & 0x1ff, 9) as i16;
            Some(format!("STI R{}, #{}", sr, pc_offset))
        }
        Some(Opcode::STR) => {
            let sr = (instr >> 9) & 0x7;
            let base_r = (instr >> 6) & 0x7;
            let offset = sign_extend(instr & 0x3F, 6) as i16;
            Some(format!("STR R{}, R{}, #{}", sr, base_r, offset))
        }
        Some(Opcode::TRAP) => {
            let trapvect = instr & 0xff;
            Some(format!("TRAP x{:04X}", trapvect))
        }
        _ => None,
    }
}

#[cfg(test)]
#[test]
fn test_simple() {
    assert_eq!(
        disassemble_instruction(0b0001_001_001_0_00001),
        Some("ADD R1, R1, R1".to_owned())
    );
    assert_eq!(
        disassemble_instruction(0b0001_001_001_1_00001),
        Some("ADD R1, R1, #1".to_owned())
    );
    assert_eq!(
        disassemble_instruction(0b0001_001_001_1_11111),
        Some("ADD R1, R1, #-1".to_owned())
    );
    assert_eq!(
        disassemble_instruction(0b0001_001_001_1_10001),
        Some("ADD R1, R1, #-15".to_owned())
    );
    assert_eq!(
        disassemble_instruction(0b0110_001_001_111111),
        Some("LDR R1, R1, #-1".to_owned())
    );
    assert_eq!(
        disassemble_instruction(0b1111_0000_0000_0000),
        Some("TRAP x0000".to_owned())
    );
}

#[test]
fn test_simple_program() {
    assert_eq!(
        &disassemble_program(&[0b0001_001_001_0_00_001, 0b0101_001_001_0_00_001,]),
        "ADD R1, R1, R1
AND R1, R1, R1
"
    );
}
