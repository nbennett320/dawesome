pub enum Opcode {
    Return = 0,
    Constant,
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    Mod,
    Nil,
    True,
    False,
    Not,
    Equal,
    Greater,
    Less,
    LogicalAnd,
    LogicalOr,
    BitwiseAnd,
    BitwiseOr,
    Print,
    Pop,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    JumpIfFalse,
    Jump,
    Loop,
    Call,
    DawesomeGlobal,

    Unknown,
}

impl Opcode {
    pub fn from(x: u8) -> Opcode {
        match x {
            0 => Opcode::Return,
            1 => Opcode::Constant,
            2 => Opcode::Negate,
            3 => Opcode::Add,
            4 => Opcode::Subtract,
            5 => Opcode::Multiply,
            6 => Opcode::Divide,
            7 => Opcode::Mod,
            8 => Opcode::Nil,
            9 => Opcode::True,
            10 => Opcode::False,
            11 => Opcode::Not,
            12 => Opcode::Equal,
            13 => Opcode::Greater,
            14 => Opcode::Less,
            15 => Opcode::LogicalAnd,
            16 => Opcode::LogicalOr,
            17 => Opcode::BitwiseAnd,
            18 => Opcode::BitwiseOr,
            19 => Opcode::Print,
            20 => Opcode::Pop,
            21 => Opcode::GetGlobal,
            22 => Opcode::SetGlobal,
            23 => Opcode::GetLocal,
            24 => Opcode::SetLocal,
            25 => Opcode::JumpIfFalse,
            26 => Opcode::Jump,
            27 => Opcode::Loop,
            28 => Opcode::Call, 
            29 => Opcode::DawesomeGlobal,

            _ => Opcode::Unknown,
        }
    }
}
