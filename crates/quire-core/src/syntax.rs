//! Shared syntax for evaluation and visual equation layout.
use crate::{
    MathError,
    calculation::{error, unit},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number {
        value: f64,
        source: String,
    },
    Symbol(String),
    Unary {
        negative: bool,
        operand: Box<Expr>,
    },
    Group(Box<Expr>),
    Binary {
        operator: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

impl Expr {
    fn depth(&self) -> usize {
        match self {
            Self::Unary { operand, .. } | Self::Group(operand) => 1 + operand.depth(),
            Self::Binary { left, right, .. } => 1 + left.depth().max(right.depth()),
            _ => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MathExpression {
    pub binding: Option<String>,
    pub expression: Expr,
    pub display_unit: Option<String>,
}

pub(crate) fn definition(source: &str) -> Result<(Option<String>, &str), MathError> {
    match source.split_once(":=") {
        Some((name, rhs)) => {
            let name = name.trim();
            let mut chars = name.chars();
            if !chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(error("A definition needs an ASCII identifier before ':='"));
            }
            if unit(name).is_some() {
                return Err(error(format!("'{name}' is a reserved length unit")));
            }
            Ok((Some(name.to_owned()), rhs))
        }
        None => Ok((None, source)),
    }
}

/// Parse without looking up variables, so undefined or dimensionally invalid
/// expressions can still be displayed faithfully in the editor.
pub fn parse_math(source: &str) -> Result<MathExpression, MathError> {
    let (binding, rhs) = definition(source)?;
    if source.len() > 16_384 {
        return Err(error("Math region exceeds 16384 bytes"));
    }
    let (expression, display_unit) = match rhs.split_once("->") {
        Some((expr, target)) => {
            let target = target.trim();
            if unit(target).is_none() {
                return Err(error("Conversion target must be m, cm, mm, or km"));
            }
            (expr, Some(target.to_owned()))
        }
        None => (rhs, None),
    };
    let mut parser = Parser {
        tokens: tokenize(expression)?,
        cursor: 0,
        depth: 0,
    };
    let expression = parser.expression(0)?;
    if parser.cursor != parser.tokens.len() {
        return Err(parser.fail("Unexpected token; use explicit operators"));
    }
    Ok(MathExpression {
        binding,
        expression,
        display_unit,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64, String),
    Name(String),
    Plus,
    Minus,
    Star,
    Slash,
    Power,
    Left,
    Right,
}

fn tokenize(source: &str) -> Result<Vec<Token>, MathError> {
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < bytes.len() {
        let start = i;
        let ch = bytes[i];
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let token = match ch {
            b'+' => Token::Plus,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'^' => Token::Power,
            b'(' => Token::Left,
            b')' => Token::Right,
            b'0'..=b'9' | b'.' => {
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
                    i += 1;
                    if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
                        i += 1;
                    }
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let value: f64 = source[start..i]
                    .parse()
                    .map_err(|_| error(format!("Invalid number at byte {}", start + 1)))?;
                if !value.is_finite() {
                    return Err(error("Numeric literal is not finite"));
                }
                tokens.push(Token::Number(value, source[start..i].into()));
                if tokens.len() > 2048 {
                    return Err(error("Expression exceeds 2048 tokens"));
                }
                continue;
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                tokens.push(Token::Name(source[start..i].into()));
                if tokens.len() > 2048 {
                    return Err(error("Expression exceeds 2048 tokens"));
                }
                continue;
            }
            _ => return Err(error(format!("Unsupported syntax at byte {}", i + 1))),
        };
        i += 1;
        tokens.push(token);
        if tokens.len() > 2048 {
            return Err(error("Expression exceeds 2048 tokens"));
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    depth: usize,
}

impl Parser {
    fn fail(&self, message: &str) -> MathError {
        error(format!("{message} at token {}", self.cursor + 1))
    }
    fn expression(&mut self, minimum: u8) -> Result<Expr, MathError> {
        self.depth += 1;
        if self.depth > 128 {
            return Err(self.fail("Expression nesting exceeds 128"));
        }
        let result = self.expression_inner(minimum);
        self.depth -= 1;
        result
    }
    fn expression_inner(&mut self, minimum: u8) -> Result<Expr, MathError> {
        let token = self
            .tokens
            .get(self.cursor)
            .cloned()
            .ok_or_else(|| self.fail("Expected an expression"))?;
        self.cursor += 1;
        let mut left = match token {
            Token::Number(value, source) => Expr::Number { value, source },
            Token::Name(name) => Expr::Symbol(name),
            Token::Plus | Token::Minus => Expr::Unary {
                negative: token == Token::Minus,
                operand: Box::new(self.expression(5)?),
            },
            Token::Left => {
                let expr = self.expression(0)?;
                if self.tokens.get(self.cursor) != Some(&Token::Right) {
                    return Err(self.fail("Expected ')'"));
                }
                self.cursor += 1;
                Expr::Group(Box::new(expr))
            }
            _ => return Err(self.fail("Expected a number, variable, or '('")),
        };
        loop {
            let implicit = matches!(self.tokens.get(self.cursor), Some(Token::Name(name)) if unit(name).is_some());
            let (left_bp, right_bp, operator) = match self.tokens.get(self.cursor) {
                Some(Token::Plus) => (1, 2, BinaryOperator::Add),
                Some(Token::Minus) => (1, 2, BinaryOperator::Subtract),
                Some(Token::Star) => (3, 4, BinaryOperator::Multiply),
                Some(Token::Slash) => (3, 4, BinaryOperator::Divide),
                Some(Token::Power) => (7, 6, BinaryOperator::Power),
                _ if implicit => (3, 4, BinaryOperator::Multiply),
                _ => break,
            };
            if left_bp < minimum {
                break;
            }
            if !implicit {
                self.cursor += 1;
            }
            let right = self.expression(right_bp)?;
            left = Expr::Binary {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            };
            if left.depth() > 128 {
                return Err(self.fail("Expression tree exceeds 128 levels"));
            }
        }
        Ok(left)
    }
}
