use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum Operator {
    Plus,
    Minus,
    Star,
    Slash,
    Mod,
    LessThan,
    GreaterThan,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Plus => write!(f, "+"),
            Operator::Minus => write!(f, "-"),
            Operator::Star => write!(f, "*"),
            Operator::Slash => write!(f, "/"),
            Operator::Mod => write!(f, "%"),
            Operator::LessThan => write!(f, "<"),
            Operator::GreaterThan => write!(f, ">"),
            Operator::Amp => write!(f, "&"),
            Operator::AmpAmp => write!(f, "&&"),
            Operator::Pipe => write!(f, "|"),
            Operator::PipePipe => write!(f, "||"),
        }
    }
}
