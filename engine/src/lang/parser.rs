use crate::lang::function::{Function, FunctionType};
use crate::lang::opcode::Opcode;
use crate::lang::precedence::Precedence;
use crate::lang::token::{Token, TokenType};
use crate::lang::value::Value;
use crate::lang::vm::InterpretError;
use crate::lang::scanner::Scanner;

pub struct Parser {
    current: Token,
    previous: Token,
    scanner: Scanner,
    functions: Vec<Function>,
    locals: Vec<Local>,
    had_error: bool,
    end_flag: bool,
    local_count: usize,
    scope_depth: usize,
}

/// represents a local variable
struct Local {
    name: String,
    depth: usize,
}

impl Parser {
    pub fn new(source: String) -> Parser {
        Parser {
            current: Token::new(TokenType::Error(String::from("current token")), 0, 0, 0),
            previous: Token::new(TokenType::Error(String::from("current token")), 0, 0, 0),
            scanner: Scanner::new(source),
            functions: Vec::new(),
            locals: Vec::new(),
            had_error: false,
            end_flag: false,
            local_count: 0,
            scope_depth: 0,
        }
    }

    pub fn compile(mut self) -> Result<Function, InterpretError> {
        // add top level functions to function stack
        self.functions
            .push(Function::new(String::new(), FunctionType::Script));

        self.advance();

        while !self.end_flag {
            self.declaration();
        }

        self.emit_op(Opcode::Return);
        Ok(self.functions[0].clone())
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn declaration(&mut self) {
        self.statement();
    }

    fn statement(&mut self) {
        match self.current.token_type.clone() {
            TokenType::Print => {
                self.advance();
                self.print_statement();
            }
            TokenType::LeftBrace => {
                self.advance();
                self.begin_scope();
                self.block();
                self.end_scope();
            }
            TokenType::If => {
                self.advance();
                self.if_statement();
            }
            TokenType::While => {
                self.advance();
                self.while_statement();
            }
            TokenType::Fn => {
                self.advance();
                self.function_definition();
            }
            TokenType::Return => {
                self.advance();
                self.return_statement();
            }
            _ => self.expression_statement(),
        }
    }

    fn function_definition(&mut self) {
        let function_name = match self.current.token_type.clone() {
            TokenType::Identifier(name) => name,
            _ => unreachable!("Not given an identifier in function_definition"),
        };

        let f = Function::new(function_name, FunctionType::Fn);
        self.functions.push(f);

        self.advance();

        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after function name");

        let mut num_params = 0;
        while !self.matches(TokenType::RightParen) {
            num_params += 1;
            match self.current.token_type.clone() {
                TokenType::Identifier(name) => self.add_local(name),
                _ => panic!("Expect identifier"),
            }
            self.advance();
        }

        let mut f = self.functions.pop().unwrap();
        f.num_params = num_params;
        self.functions.push(f);

        self.consume(TokenType::LeftBrace, "Expect '{' before function body");

        self.block();

        // add implicit nil for empty functions
        if self.functions.last_mut().unwrap().chunk.code.is_empty() {
            self.emit_constant(Value::Nil);
        }

        self.emit_op(Opcode::Return);

        let f = self.functions.pop().unwrap();
        f.chunk.disassemble(&f.name);
        let global = self.make_constant(Value::Function(f));
        self.emit_op(Opcode::SetGlobal);
        self.emit_byte(global as u8);
    }

    fn expression_statement(&mut self) {
        self.expression();
    }

    fn block(&mut self) {
        while self.current.token_type != TokenType::RightBrace {
            self.declaration()
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block");
    }

    fn if_statement(&mut self) {
        self.expression();
        let if_offset = self.emit_jump(Opcode::JumpIfFalse);
        self.emit_op(Opcode::Pop);
        self.statement();

        let else_offset = self.emit_jump(Opcode::Jump);
        self.emit_op(Opcode::Pop);

        self.patch_jump(if_offset);

        // compile optional else clause
        if self.matches(TokenType::Else) {
            self.statement();
        }

        self.patch_jump(else_offset);
    }

    fn while_statement(&mut self) {
        let loop_start = self.functions.last_mut().unwrap().chunk.code.len();
        self.expression();
        let exit_offset = self.emit_jump(Opcode::JumpIfFalse);
        self.emit_op(Opcode::Pop);
        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_offset);
        self.emit_op(Opcode::Pop);
    }

    fn return_statement(&mut self) {
        // TODO: don't parse expression if return is followed immediately by \n
        self.expression();
        self.emit_op(Opcode::Return);
    }

    pub fn advance(&mut self) {
        self.previous = self.current.clone();

        if let Some(tok) = self.scanner.next() {
            self.current = tok;
        } else {
            self.end_flag = true;
        }
    }

    fn consume(&mut self, token_type: TokenType, msg: &str) {
        if !(self.current.token_type == token_type) {
            self.error_at_current(msg);
        }

        self.advance();
    }

    pub fn string(&mut self, _can_assign: bool) {
        match &self.previous.token_type {
            TokenType::String(s) => self.emit_constant(Value::String(s.to_string())),
            _ => unreachable!("No string"),
        }
    }

    fn error(&mut self, msg: &str) {
        self.error_at(self.previous.clone(), msg);
    }

    fn error_at_current(&mut self, msg: &str) {
        self.error_at(self.current.clone(), msg);
    }

    fn error_at(&mut self, tok: Token, msg: &str) {
        println!("[line {}] Error: {}", tok.line, msg);
        self.had_error = true;
    }

    pub fn number(&mut self, _can_assign: bool) {
        if let TokenType::Number(num) = self.previous.token_type {
            self.emit_constant(Value::Number(num));
        }
    }

    fn emit_byte(&mut self, byte: u8) {
        self.functions
            .last_mut()
            .unwrap()
            .chunk
            .write(byte, self.previous.line);
    }

    fn emit_bytes(&mut self, a: u8, b: u8) {
        self.emit_byte(a);
        self.emit_byte(b);
    }

    fn emit_op(&mut self, op: Opcode) {
        self.emit_byte(op as u8);
    }

    fn emit_ops(&mut self, op1: Opcode, op2: Opcode) {
        self.emit_byte(op1 as u8);
        self.emit_byte(op2 as u8);
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value) as u8;
        self.emit_bytes(Opcode::Constant as u8, constant);
    }

    fn emit_jump(&mut self, op: Opcode) -> usize {
        self.emit_byte(op as u8);
        self.emit_bytes(0xff, 0xff);
        self.functions.last_mut().unwrap().chunk.code.len() - 2
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.functions.last_mut().unwrap().chunk.code.len() - offset - 2;

        if jump > std::i16::MAX as usize {
            self.error("Jump is out of bounds");
        }

        self.functions.last_mut().unwrap().chunk.code[offset] = ((jump >> 8) & 0xff) as u8;
        self.functions.last_mut().unwrap().chunk.code[offset + 1] = (jump & 0xff) as u8;
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_op(Opcode::Loop);

        let offset = self.functions.last_mut().unwrap().chunk.code.len() - loop_start + 2;
        if offset as u16 > std::u16::MAX {
            self.error("Loop offset is out of bounds");
        }

        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }

    fn make_constant(&mut self, value: Value) -> usize {
        let constant = self.functions.last_mut().unwrap().chunk.add_constant(value);
        if constant > std::u8::MAX as usize {
            self.error("Too many constants in this chunk");
            0
        } else {
            constant
        }
    }

    pub fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression");
    }

    pub fn unary(&mut self, _can_assign: bool) {
        let operator = self.previous.token_type.clone();
        self.parse_precedence(Precedence::Unary);

        match operator {
            TokenType::Minus => self.emit_op(Opcode::Negate),
            TokenType::Bang => self.emit_op(Opcode::Not),
            _ => unreachable!("Impossible unary operator"),
        }
    }

    pub fn call(&mut self, _can_assign: bool) {
        let mut num_params = 0;

        if self.current.token_type.clone() != TokenType::RightParen {
            while {
                num_params += 1;
                self.expression();

                self.matches(TokenType::Comma)
            } {}
        }

        self.consume(TokenType::RightParen, "Expected ) after arguments");

        self.emit_op(Opcode::Call);
        self.emit_byte(num_params);
    }

    pub fn binary(&mut self, _can_assign: bool) {
        let operator = self.previous.token_type.clone();
        let rule = operator.rule();
        let precedence = Precedence::from(rule.precedence as usize + 1);
        self.parse_precedence(precedence);

        match operator {
            TokenType::Plus => self.emit_op(Opcode::Add),
            TokenType::Minus => self.emit_op(Opcode::Subtract),
            TokenType::Star => self.emit_op(Opcode::Multiply),
            TokenType::Slash => self.emit_op(Opcode::Divide),
            TokenType::Mod => self.emit_op(Opcode::Mod),
            TokenType::BangEqual => self.emit_ops(Opcode::Equal, Opcode::Not),
            TokenType::EqualEqual => self.emit_op(Opcode::Equal),
            TokenType::Greater => self.emit_op(Opcode::Greater),
            TokenType::GreaterEqual => self.emit_ops(Opcode::Less, Opcode::Not),
            TokenType::Less => self.emit_op(Opcode::Less),
            TokenType::LessEqual => self.emit_ops(Opcode::Greater, Opcode::Not),
            TokenType::BitwiseAnd => self.emit_op(Opcode::BitwiseAnd),
            TokenType::BitwiseOr => self.emit_op(Opcode::BitwiseOr),
            TokenType::LogicalAnd => self.emit_op(Opcode::LogicalAnd),
            TokenType::LogicalOr => self.emit_op(Opcode::LogicalOr),
            TokenType::And => self.emit_op(Opcode::LogicalAnd),
            _ => (),
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let rule = self.previous.token_type.rule();

        if let Some(prefix_rule) = rule.prefix {
            let can_assign = precedence as usize <= Precedence::Assignment as usize;
            prefix_rule(self, can_assign);

            let prec_u8 = precedence as u8;
            while prec_u8 <= self.current.token_type.rule().precedence as u8 {
                self.advance();
                if let Some(infix_rule) = self.previous.token_type.rule().infix {
                    infix_rule(self, can_assign);
                }
            }

            return;
        }

        self.error("Expected expression");
    }

    pub fn literal(&mut self, _can_assign: bool) {
        let token_type = self.previous.token_type.clone();
        match token_type {
            TokenType::False => self.emit_op(Opcode::False),
            TokenType::Nil => self.emit_op(Opcode::Nil),
            TokenType::True => self.emit_op(Opcode::True),
            _ => unreachable!("Impossible TokenType in literal"),
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.emit_op(Opcode::Print);
    }

    fn matches(&mut self, token_type: TokenType) -> bool {
        if self.current.token_type == token_type {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn variable(&mut self, can_assign: bool) {
        let identifier = self.previous.clone();
        let name = match identifier.token_type.clone() {
            TokenType::Identifier(name) => name,
            _ => unreachable!("In variable() without name"),
        };

        let (get_op, set_op, constant) = match self.resolve_local(&identifier) {
            Ok(id) => (Opcode::GetLocal, Opcode::SetLocal, id),
            Err(_) => (
                Opcode::GetGlobal,
                Opcode::SetGlobal,
                self.make_constant(Value::String(name)),
            ),
        };

        if can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit_op(set_op);
            self.emit_byte(constant as u8);
        } else {
            self.emit_op(get_op);
            self.emit_byte(constant as u8);
        }
    }

    fn add_local(&mut self, name: String) {
        // no more than 255 local variables
        if self.local_count == u8::MAX as usize {
            self.error("Too many local variables");
            return;
        }

        let local = Local {
            name,
            depth: self.scope_depth,
        };

        self.local_count += 1;
        self.locals.push(local);
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        // pop local variables introduced in this scope off  the stack
        while self.local_count > 0 && self.locals[self.local_count - 1].depth > self.scope_depth {
            self.emit_op(Opcode::Pop);
            self.local_count -= 1;
        }
    }

    fn resolve_local(&self, name: &Token) -> Result<usize, ()> {
        if self.locals.is_empty() {
            return Err(());
        }

        let mut local_count = self.local_count - 1;
        let identifier = match &name.token_type {
            TokenType::Identifier(id) => id,
            _ => unreachable!("Was not given an identifier to resolve_local"),
        };

        loop {
            let local = &self.locals[local_count];
            if local.name == *identifier {
                return Ok(local_count);
            }

            if local_count == 0 {
                break;
            }

            local_count -= 1;
        }

        Err(())
    }
}
