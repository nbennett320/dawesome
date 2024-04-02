use crate::lang::precedence::Precedence;
use crate::lang::parser::Parser;

pub struct ParseRule {
    pub prefix: Option<fn(parser: &mut Parser, can_assign: bool)>,
    pub infix: Option<fn(parser: &mut Parser, can_assign: bool)>,
    pub precedence: Precedence,
}
