use crate::lang::parse_rule::ParseRule;
use crate::lang::parser::Parser;
use crate::lang::precedence::Precedence;

use std::fmt::{self, Display, Formatter};

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    // 1 character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Mod,
    BitwiseAnd,
    BitwiseOr,

    // 1 or 2 character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    LogicalAnd,
    LogicalOr,

    // Literals
    Identifier(String),
    String(String),
    Number(f64),

    // Keywords
    And,
    Class,
    Else,
    False,
    For,
    Fn,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    // Misc tokens
    Error(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

impl Token {
    pub fn new(token_type: TokenType, line: usize, col: usize, len: usize) -> Token {
        Token {
            token_type,
            line,
            col,
            len,
        }
    }
}

impl TokenType {
    pub fn rule(&self) -> &'static ParseRule {
        match self {
            TokenType::LeftParen => &ParseRule {
                prefix: Some(Parser::grouping),
                infix: Some(Parser::call),
                precedence: Precedence::Call,
            },
            TokenType::RightParen => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::LeftBrace => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::RightBrace => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Comma => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Dot => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Minus => &ParseRule {
                prefix: Some(Parser::unary),
                infix: Some(Parser::binary),
                precedence: Precedence::Term,
            },
            TokenType::Plus => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Term,
            },
            TokenType::Semicolon => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Slash => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Factor,
            },
            TokenType::Star => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Factor,
            },
            TokenType::Mod => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Factor,
            },
            TokenType::BitwiseAnd => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Term,
            },
            TokenType::BitwiseOr => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Term,
            },
            TokenType::LogicalAnd => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::And,
            },
            TokenType::LogicalOr => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Or,
            },
            TokenType::Bang => &ParseRule {
                prefix: Some(Parser::unary),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::BangEqual => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Equality,
            },
            TokenType::Equal => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::EqualEqual => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::Greater => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::GreaterEqual => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::Less => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::LessEqual => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::Identifier(_) => &ParseRule {
                prefix: Some(Parser::variable),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::String(_) => &ParseRule {
                prefix: Some(Parser::string),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Number(_) => &ParseRule {
                prefix: Some(Parser::number),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::And => &ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::And,
            },
            TokenType::Class => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Else => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::False => &ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::For => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Fn => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::If => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Nil => &ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Or => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Print => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Return => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Super => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::This => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::True => &ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Var => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::While => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Error(_) => &ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        }
    }
}

impl Display for TokenType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::LeftParen => write!(f, "LeftParen"),
            TokenType::RightParen => write!(f, "RightParen"),
            TokenType::LeftBrace => write!(f, "LeftBrace"),
            TokenType::RightBrace => write!(f, "RightBrace"),
            TokenType::Comma => write!(f, "Comma"),
            TokenType::Dot => write!(f, "Dot"),
            TokenType::Minus => write!(f, "Minus"),
            TokenType::Plus => write!(f, "Plus"),
            TokenType::Semicolon => write!(f, "Semicolon"),
            TokenType::Slash => write!(f, "Slash"),
            TokenType::Star => write!(f, "Star"),
            TokenType::Mod => write!(f, "Mod"),
            TokenType::BitwiseAnd => write!(f, "BitwiseAnd"),
            TokenType::BitwiseOr => write!(f, "BitwiseOr"),
            TokenType::LogicalAnd => write!(f, "LogicalAnd"),
            TokenType::LogicalOr => write!(f, "LogicalOr"),
            TokenType::Bang => write!(f, "Bang"),
            TokenType::BangEqual => write!(f, "BangEqual"),
            TokenType::Equal => write!(f, "Equal"),
            TokenType::EqualEqual => write!(f, "EqualEqual"),
            TokenType::Greater => write!(f, "Greater"),
            TokenType::GreaterEqual => write!(f, "GreaterEqual"),
            TokenType::Less => write!(f, "Less"),
            TokenType::LessEqual => write!(f, "LessEqual"),
            TokenType::Identifier(_) => write!(f, "Identifier"),
            TokenType::String(_) => write!(f, "String"),
            TokenType::Number(_) => write!(f, "Number"),
            TokenType::And => write!(f, "And"),
            TokenType::Class => write!(f, "Class"),
            TokenType::Else => write!(f, "Else"),
            TokenType::False => write!(f, "False"),
            TokenType::For => write!(f, "For"),
            TokenType::Fn => write!(f, "Fn"),
            TokenType::If => write!(f, "If"),
            TokenType::Nil => write!(f, "Nil"),
            TokenType::Or => write!(f, "Or"),
            TokenType::Print => write!(f, "Print"),
            TokenType::Return => write!(f, "Return"),
            TokenType::Super => write!(f, "Super"),
            TokenType::This => write!(f, "This"),
            TokenType::True => write!(f, "True"),
            TokenType::Var => write!(f, "Var"),
            TokenType::While => write!(f, "While"),
            TokenType::Error(_) => write!(f, "Error"),
        }
    }
}
