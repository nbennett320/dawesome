use crate::lang::opcode::Opcode;
use crate::lang::value::Value;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn disassemble(&self, name: &str) {
        // only disassemble in debug mode
        if !cfg!(debug_assertions) {
            return;
        }

        println!("== {} ==", name);

        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
            print!("   | ");
        } else {
            print!("{:4} ", self.lines[offset]);
        }

        let instruction = self.code[offset];
        match Opcode::from(instruction) {
            Opcode::Return => self.simple_instruction("Return", offset),
            Opcode::Constant => self.constant_instruction("Constant", offset),
            Opcode::Negate => self.simple_instruction("Negate", offset),
            Opcode::Add => self.simple_instruction("Add", offset),
            Opcode::Subtract => self.simple_instruction("Subtract", offset),
            Opcode::Multiply => self.simple_instruction("Multiply", offset),
            Opcode::Divide => self.simple_instruction("Divide", offset),
            Opcode::Mod => self.simple_instruction("Mod", offset),
            Opcode::Nil => self.simple_instruction("Nil", offset),
            Opcode::True => self.simple_instruction("True", offset),
            Opcode::False => self.simple_instruction("False", offset),
            Opcode::Not => self.simple_instruction("Not", offset),
            Opcode::Equal => self.simple_instruction("Equal", offset),
            Opcode::Greater => self.simple_instruction("Greater", offset),
            Opcode::Less => self.simple_instruction("Less", offset),
            Opcode::LogicalAnd => self.simple_instruction("LogicalAnd", offset),
            Opcode::LogicalOr => self.simple_instruction("LogicalOr", offset),
            Opcode::BitwiseAnd => self.simple_instruction("BitwiseAnd", offset),
            Opcode::BitwiseOr => self.simple_instruction("BitwiseOr", offset),
            Opcode::Print => self.simple_instruction("Print", offset),
            Opcode::Pop => self.simple_instruction("Pop", offset),
            Opcode::GetGlobal => self.constant_instruction("GetGlobal", offset),
            Opcode::SetGlobal => self.constant_instruction("SetGlobal", offset),
            Opcode::GetLocal => self.byte_instruction("GetLocal", offset),
            Opcode::SetLocal => self.byte_instruction("SetLocal", offset),
            Opcode::JumpIfFalse => self.jump_instruction("JumpIfFalse", 1, offset),
            Opcode::Jump => self.jump_instruction("Jump", 1, offset),
            Opcode::Loop => self.jump_instruction("Loop", -1, offset),
            Opcode::Call => self.byte_instruction("Call", offset),
            _ => {
                println!("Unknown opcode: {}", instruction);
                offset + 1
            }
        }
    }

    fn simple_instruction(&self, name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }

    fn constant_instruction(&self, name: &str, offset: usize) -> usize {
        let constant = self.code[offset + 1] as usize;
        print!("{} {} ", name, constant);
        println!("{}", self.constants[constant]);
        offset + 2
    }

    fn byte_instruction(&self, name: &str, offset: usize) -> usize {
        let slot = self.code[offset + 1];
        println!("{} {}", name, slot);
        offset + 2
    }

    fn jump_instruction(&self, name: &str, sign: i32, offset: usize) -> usize {
        let mut jump = (self.code[offset + 1] as u16) << 8;
        jump |= self.code[offset + 2] as u16;
        let to = sign * (jump as i32);
        println!(
            "{} {} -> {}",
            name,
            offset,
            ((offset as i64) + 3 + to as i64) as i64
        );
        offset + 3
    }
}
