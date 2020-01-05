use std::convert::TryFrom;
use std::iter::Peekable;
use std::str::Chars;

use crate::common::{Keyword, Pos, Symbol};
use crate::selector::{Attribute, AttributeKind, Selector};
use crate::units::Unit;
use crate::{Token, TokenKind, Whitespace};

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    tokens: Vec<Token>,
    buf: Peekable<Chars<'a>>,
    pos: Pos,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! symbol {
            ($self:ident, $symbol:ident) => {{
                $self.buf.next();
                $self.pos.next_char();
                TokenKind::Symbol(Symbol::$symbol)
            }};
        }
        macro_rules! whitespace {
            ($self:ident, $whitespace:ident) => {{
                $self.buf.next();
                $self.pos.next_char();
                TokenKind::Whitespace(Whitespace::$whitespace)
            }};
        }
        let kind: TokenKind = match self.buf.peek().unwrap_or(&'\0') {
            'a'..='z' | 'A'..='Z' => self.lex_ident(),
            '@' => self.lex_at_rule(),
            '0'..='9' => self.lex_num(),
            '$' => self.lex_variable(),
            ':' => symbol!(self, Colon),
            ',' => symbol!(self, Comma),
            '.' => symbol!(self, Period),
            ';' => symbol!(self, SemiColon),
            '+' => symbol!(self, Plus),
            '~' => symbol!(self, Tilde),
            '\'' => symbol!(self, SingleQuote),
            '"' => symbol!(self, DoubleQuote),
            ' ' => whitespace!(self, Space),
            '\t' => whitespace!(self, Tab),
            '\n' => whitespace!(self, Newline),
            '\r' => whitespace!(self, CarriageReturn),
            '#' => symbol!(self, Hash),
            '{' => symbol!(self, OpenBrace),
            '*' => symbol!(self, Mul),
            '}' => symbol!(self, CloseBrace),
            '/' => self.lex_forward_slash(),
            '%' => {
                self.buf.next();
                self.pos.next_char();
                TokenKind::Unit(Unit::Percent)
            }
            '[' => {
                self.buf.next();
                self.pos.next_char();
                self.lex_attr()
            }
            '<' => symbol!(self, Lt),
            '>' => symbol!(self, Gt),
            '\0' => return None,
            _ => todo!("unknown char"),
        };
        self.pos.next_char();
        Some(Token {
            kind,
            pos: self.pos,
        })
    }
}

fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\n' || c == '\r'
}

impl<'a> Lexer<'a> {
    pub fn new(buf: &'a str) -> Lexer<'a> {
        Lexer {
            tokens: Vec::with_capacity(buf.len()),
            buf: buf.chars().peekable(),
            pos: Pos::new(),
        }
    }

    fn devour_whitespace(&mut self) {
        while let Some(c) = self.buf.peek() {
            if !is_whitespace(*c) {
                break;
            }
        }
    }

    fn lex_at_rule(&mut self) -> TokenKind {
        let mut string = String::with_capacity(99);
        while let Some(c) = self.buf.peek() {
            if !c.is_alphabetic() && c != &'-' {
                break;
            }
            let tok = self
                .buf
                .next()
                .expect("this is impossible because we have already peeked");
            self.pos.next_char();
            string.push(tok);
        }

        if let Ok(kw) = Unit::try_from(string.as_ref()) {
            TokenKind::Unit(kw)
        } else {
            panic!("expected ident after `@`")
        }
    }

    fn lex_forward_slash(&mut self) -> TokenKind {
        self.buf.next();
        self.pos.next_char();
        match self.buf.peek().expect("expected something after '/'") {
            '/' => {
                self.buf.by_ref().skip_while(|x| x != &'\n').count();
            }
            '*' => {
                while let Some(tok) = self.buf.next() {
                    if tok == '*' && self.buf.next() == Some('/') {
                        break;
                    }
                }
            }
            _ => return TokenKind::Symbol(Symbol::Div),
        }
        TokenKind::Whitespace(Whitespace::Newline)
    }

    fn lex_num(&mut self) -> TokenKind {
        let mut string = String::with_capacity(99);
        while let Some(c) = self.buf.peek() {
            if !c.is_numeric() && c != &'.' {
                break;
            }
            let tok = self
                .buf
                .next()
                .expect("this is impossible because we have already peeked");
            self.pos.next_char();
            string.push(tok);
        }

        TokenKind::Number(string)
    }

    fn lex_hash(&mut self) -> TokenKind {
        todo!()
    }

    fn lex_attr(&mut self) -> TokenKind {
        let mut attr = String::with_capacity(99);
        while let Some(c) = self.buf.peek() {
            if !c.is_alphabetic() && c != &'-' {
                break;
            }
            let tok = self
                .buf
                .next()
                .expect("this is impossible because we have already peeked");
            self.pos.next_char();
            attr.push(tok);
        }

        self.devour_whitespace();

        let kind = match self
            .buf
            .next()
            .expect("todo! expected kind (should be error)")
        {
            ']' => {
                return TokenKind::Selector(Selector::Attribute(Attribute {
                    kind: AttributeKind::Any,
                    attr,
                    value: String::new(),
                    case_sensitive: true,
                }))
            }
            'i' => {
                self.devour_whitespace();
                assert!(self.buf.next() == Some(']'));
                return TokenKind::Selector(Selector::Attribute(Attribute {
                    kind: AttributeKind::Any,
                    attr,
                    value: String::new(),
                    case_sensitive: false,
                }));
            }
            '=' => AttributeKind::Equals,
            '~' => AttributeKind::InList,
            '|' => AttributeKind::BeginsWithHyphenOrExact,
            '^' => AttributeKind::StartsWith,
            '$' => AttributeKind::EndsWith,
            '*' => AttributeKind::Contains,
            _ => todo!("expected kind (should be error)"),
        };

        if kind != AttributeKind::Equals {
            assert!(self.buf.next() == Some('='));
        }

        self.devour_whitespace();

        match self
            .buf
            .peek()
            .expect("todo! expected either value or quote")
        {
            '\'' | '"' => {
                self.buf.next();
            }
            _ => {}
        }

        let mut value = String::with_capacity(99);
        let mut case_sensitive = true;

        while let Some(c) = self.buf.peek() {
            if !c.is_alphabetic() && c != &'-' {
                break;
            }

            if c == &'i' {
                let tok = self
                    .buf
                    .next()
                    .expect("this is impossible because we have already peeked");
                self.pos.next_char();
                self.devour_whitespace();
                match self.buf.next() {
                    Some(']') => case_sensitive = false,
                    Some(val) => {
                        self.pos.next_char();
                        value.push(tok);
                        value.push(val);
                    }
                    None => todo!("expected something to come after "),
                }
                continue;
            }

            let tok = self
                .buf
                .next()
                .expect("this is impossible because we have already peeked");
            self.pos.next_char();
            value.push(tok);
        }

        match self
            .buf
            .peek()
            .expect("todo! expected either value or quote")
        {
            '\'' | '"' => {
                self.buf.next();
            }
            _ => {}
        }

        self.devour_whitespace();

        assert!(self.buf.next() == Some(']'));

        TokenKind::Selector(Selector::Attribute(Attribute {
            kind,
            attr,
            value,
            case_sensitive,
        }))
    }

    fn lex_variable(&mut self) -> TokenKind {
        let mut string = String::with_capacity(99);
        while let Some(c) = self.buf.peek() {
            if !c.is_alphabetic() && c != &'-' {
                break;
            }
            let tok = self
                .buf
                .next()
                .expect("this is impossible because we have already peeked");
            self.pos.next_char();
            string.push(tok);
        }
        TokenKind::Variable(string)
    }

    fn lex_ident(&mut self) -> TokenKind {
        let mut string = String::with_capacity(99);
        while let Some(c) = self.buf.peek() {
            // we know that the first char is alphabetic from peeking
            if !c.is_alphanumeric() && c != &'-' {
                break;
            }
            let tok = self
                .buf
                .next()
                .expect("this is impossible because we have already peeked");
            self.pos.next_char();
            string.push(tok);
        }

        if let Ok(kw) = Keyword::try_from(string.as_ref()) {
            return TokenKind::Keyword(kw);
        }

        if let Ok(kw) = Unit::try_from(string.as_ref()) {
            return TokenKind::Unit(kw);
        }

        TokenKind::Ident(string)
    }
}