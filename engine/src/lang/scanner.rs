use crate::lang::token::{Token, TokenType};

fn is_digit(c: char) -> bool {
    matches!(c, '0'..='9')
}

fn is_alpha(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '_')
}

pub struct Scanner {
    source: String,
    start: usize, // index of beginning of lexeme being scanned
    pos: usize,   // current character being looked at
    line: usize,
}

impl Scanner {
    pub fn new(source: String) -> Scanner {
        Scanner {
            source,
            start: 0,
            pos: 0,
            line: 1,
        }
    }

    // used only in tests
    #[cfg(test)]
    fn scan_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(tok) = self.next() {
            tokens.push(tok);
        }
        tokens
    }

    pub fn next(&mut self) -> Option<Token> {
        // ignores whitespace between tokens
        self.skip_whitespace();

        self.start = self.pos;

        if self.eof() {
            return None;
        }

        let c = self.advance();
        match c {
            '(' => Some(self.make_token(TokenType::LeftParen)),
            ')' => Some(self.make_token(TokenType::RightParen)),
            '{' => Some(self.make_token(TokenType::LeftBrace)),
            '}' => Some(self.make_token(TokenType::RightBrace)),
            ';' => Some(self.make_token(TokenType::Semicolon)),
            ',' => Some(self.make_token(TokenType::Comma)),
            '.' => Some(self.make_token(TokenType::Dot)),
            '-' => Some(self.make_token(TokenType::Minus)),
            '+' => Some(self.make_token(TokenType::Plus)),
            '/' => Some(self.make_token(TokenType::Slash)),
            '*' => Some(self.make_token(TokenType::Star)),
            '%' => Some(self.make_token(TokenType::Mod)),
            '&' => {
                let token_type = if self.matches('&') {
                    TokenType::LogicalAnd
                } else {
                    TokenType::BitwiseAnd
                };
                Some(self.make_token(token_type))
            }
            '|' => {
                let token_type = if self.matches('|') {
                    TokenType::LogicalOr
                } else {
                    TokenType::BitwiseOr
                };
                Some(self.make_token(token_type))
            }
            '!' => {
                let token_type = if self.matches('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                Some(self.make_token(token_type))
            }
            '=' => {
                let token_type = if self.matches('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                Some(self.make_token(token_type))
            }
            '<' => {
                let token_type = if self.matches('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                Some(self.make_token(token_type))
            }
            '>' => {
                let token_type = if self.matches('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                Some(self.make_token(token_type))
            }
            '\'' | '\"' => Some(self.string(c)),
            '0'..='9' => Some(self.number()),
            'a'..='z' | 'A'..='Z' | '_' => Some(self.identifier()),
            _ => None,
        }
    }

    fn make_token(&self, token_type: TokenType) -> Token {
        let len = self.pos - self.start;
        println!("token: {}", self.line);
        Token::new(token_type, self.line, self.start, len)
    }

    /// emits a syntax error token
    fn error(&self, msg: &str) -> Token {
        Token::new(
            TokenType::Error(String::from(msg)),
            self.line,
            self.start,
            msg.len(),
        )
    }

    fn peek(&self) -> char {
        if self.eof() {
            '\0'
        } else {
            self.source[self.pos..].chars().next().unwrap()
        }
    }

    /// advances the scanner's position and returns the consumed character
    fn advance(&mut self) -> char {
        let c = self.peek();
        self.pos += 1;
        c
    }

    /// returns true if next character matches the expected character, consuming it
    fn matches(&mut self, expected: char) -> bool {
        if self.eof() || self.peek() != expected {
            false
        } else {
            self.pos += 1;
            true
        }
    }

    fn string(&mut self, delimiter: char) -> Token {
        assert!(delimiter == '\'' || delimiter == '\"');
        self.advance();
        while self.peek() != delimiter && !self.eof() {
            if self.peek() == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.eof() {
            self.error("Unterminated string")
        } else {
            // consume closing '
            self.advance();

            let string = &self.source[self.start + 1..self.pos - 1];
            self.make_token(TokenType::String(String::from(string)))
        }
    }

    fn number(&mut self) -> Token {
        while is_digit(self.peek()) {
            self.advance();
        }

        if self.peek() == '.' && is_digit(self.peek_next()) {
            // consume the '.'
            self.advance();

            while is_digit(self.peek()) {
                self.advance();
            }
        }

        let string = &self.source[self.start..self.pos];
        let num: f64 = string.parse().unwrap();
        self.make_token(TokenType::Number(num))
    }

    fn identifier(&mut self) -> Token {
        while is_alpha(self.peek()) || is_digit(self.peek()) {
            self.advance();
        }

        let tok = self.identifier_type();
        self.make_token(tok)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '#' => {
                    while self.peek() != '\n' && !self.eof() {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn peek_next(&self) -> char {
        let next = self.pos + 1;
        if self.eof() {
            '\0'
        } else {
            self.source[next..].chars().next().unwrap()
        }
    }

    fn identifier_type(&mut self) -> TokenType {
        let c = self.source[self.start..].chars().next().unwrap();
        let t = match c {
            'a' => self.check_keyword(1, 2, "nd", TokenType::And),
            'c' => self.check_keyword(1, 4, "lass", TokenType::Class),
            // 'd' => self.check_keyword(1, 10, "awesome_fn", TokenType::DawesomeFn),
            'e' => self.check_keyword(1, 3, "lse", TokenType::Else),
            'i' => self.check_keyword(1, 1, "f", TokenType::If),
            'f' => {
                if self.pos - self.start > 1 {
                    match self.source.chars().nth(self.start + 1).unwrap() {
                        'a' => self.check_keyword(2, 3, "lse", TokenType::False),
                        'o' => self.check_keyword(2, 1, "r", TokenType::For),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            'n' => self.check_keyword(1, 2, "il", TokenType::Nil),
            'o' => self.check_keyword(1, 1, "r", TokenType::Or),
            'p' => self.check_keyword(1, 4, "rint", TokenType::Print),
            'r' => self.check_keyword(1, 5, "eturn", TokenType::Return),
            's' => self.check_keyword(1, 4, "uper", TokenType::Super),
            't' => {
                if self.pos - self.start > 1 {
                    match self.source.chars().nth(self.start + 1).unwrap() {
                        'h' => self.check_keyword(2, 2, "is", TokenType::This),
                        'r' => self.check_keyword(2, 2, "ue", TokenType::True),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            'v' => self.check_keyword(1, 2, "ar", TokenType::Var),
            'w' => self.check_keyword(1, 4, "hile", TokenType::While),
            _ => None,
        };

        if let Some(token_type) = t {
            token_type
        } else {
            let ident = &self.source[self.start..self.pos];
            if ident == "fn" {
                TokenType::Fn
            } else if ident == "dawesome_fn" {
                println!("exec dawesome f");
                
                TokenType::DawesomeFn
            } else {
                TokenType::Identifier(String::from(ident))
            }
        }
    }

    fn check_keyword(
        &self,
        start: usize,
        len: usize,
        rest: &str,
        token_type: TokenType,
    ) -> Option<TokenType> {
        if (self.pos - self.start == start + len)
            && &self.source[self.start + start..self.start + start + len] == rest
        {
            Some(token_type)
        } else {
            None
        }
    }

    fn eof(&self) -> bool {
        self.pos >= self.source.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_returns_empty_vector_of_tokens() {
        let mut scanner = Scanner::new(String::from(""));
        assert_eq!(scanner.scan_all().len(), 0);
    }

    #[test]
    fn scans_keyword_return() {
        let mut scanner = Scanner::new(String::from("return"));
        let tokens = scanner.scan_all();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::new(TokenType::Return, 1, 0, 6));
    }

    #[test]
    fn scans_keyword_false() {
        let mut scanner = Scanner::new(String::from("false"));
        let tokens = scanner.scan_all();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::new(TokenType::False, 1, 0, 5));
    }

    #[test]
    fn scans_keyword_fn() {
        let mut scanner = Scanner::new(String::from("fn f fna"));
        let tokens = scanner.scan_all();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::new(TokenType::Fn, 1, 0, 2));
        assert_eq!(
            tokens[1],
            Token::new(TokenType::Identifier(String::from("f")), 1, 3, 1)
        );
        assert_eq!(
            tokens[2],
            Token::new(TokenType::Identifier(String::from("fna")), 1, 5, 3)
        );
    }

    #[test]
    fn scans_list_of_keywords() {
        let mut scanner = Scanner::new(String::from("nil if fn"));
        let tokens = scanner.scan_all();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::new(TokenType::Nil, 1, 0, 3));
        assert_eq!(tokens[1], Token::new(TokenType::If, 1, 4, 2));
        assert_eq!(tokens[2], Token::new(TokenType::Fn, 1, 7, 2));
    }

    #[test]
    fn ignores_whitespace() {
        let mut scanner = Scanner::new(String::from("  \tnil"));
        let tokens = scanner.scan_all();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::new(TokenType::Nil, 1, 3, 3));
    }

    #[test]
    fn finds_string() {
        let mut scanner = Scanner::new(String::from("'hello world'"));
        let tokens = scanner.scan_all();
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0],
            Token::new(TokenType::String(String::from("hello world")), 1, 0, 13)
        );
    }

    #[test]
    fn finds_number() {
        let mut scanner = Scanner::new(String::from("12.34"));
        let tokens = scanner.scan_all();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::new(TokenType::Number(12.34), 1, 0, 5));
    }
}
