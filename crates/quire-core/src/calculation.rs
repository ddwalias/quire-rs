use crate::{RegionContent, Worksheet};
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathError(pub String);

impl fmt::Display for MathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for MathError {}
fn error(message: impl Into<String>) -> MathError {
    MathError(message.into())
}

/// Finite real value in SI base units. The only base dimension is length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    pub si_value: f64,
    pub length_power: i16,
}

impl Quantity {
    fn new(si_value: f64, length_power: i16) -> Result<Self, MathError> {
        if !si_value.is_finite() {
            return Err(error("Non-finite result (overflow or invalid calculation)"));
        }
        if !(-64..=64).contains(&length_power) {
            return Err(error(
                "Dimension exponent exceeds the supported range -64..64",
            ));
        }
        Ok(Self {
            si_value,
            length_power,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Calculation {
    pub binding: Option<String>,
    pub quantity: Quantity,
    pub display_value: f64,
    pub display_unit: String,
}

impl fmt::Display for Calculation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_value)?;
        if !self.display_unit.is_empty() {
            write!(f, " {}", self.display_unit)?;
        }
        Ok(())
    }
}

fn unit(name: &str) -> Option<f64> {
    match name {
        "m" => Some(1.0),
        "cm" => Some(0.01),
        "mm" => Some(0.001),
        "km" => Some(1000.0),
        _ => None,
    }
}
fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
type Bindings = HashMap<String, Result<Quantity, MathError>>;

pub(crate) fn calculate(sheet: &Worksheet) -> Vec<Option<Result<Calculation, MathError>>> {
    let mut bindings = Bindings::new();
    sheet
        .regions
        .iter()
        .map(|region| match &region.content {
            RegionContent::Text(_) => None,
            RegionContent::Math(source) => Some(calculate_region(source, &mut bindings)),
        })
        .collect()
}

fn calculate_region(source: &str, bindings: &mut Bindings) -> Result<Calculation, MathError> {
    let (binding, expression) = match source.split_once(":=") {
        Some((name, rhs)) => {
            let name = name.trim();
            if !identifier(name) {
                return Err(error("A definition needs an ASCII identifier before ':='"));
            }
            if unit(name).is_some() {
                return Err(error(format!("'{name}' is a reserved length unit")));
            }
            (Some(name.to_owned()), rhs)
        }
        None => (None, source),
    };
    let result = if source.len() > 16_384 {
        Err(error("Math region exceeds 16384 bytes"))
    } else {
        evaluate(expression, bindings)
    };
    if let Some(name) = &binding {
        bindings.insert(
            name.clone(),
            result.as_ref().map(|c| c.quantity).map_err(Clone::clone),
        );
    }
    result.map(|mut c| {
        c.binding = binding;
        c
    })
}

fn evaluate(source: &str, bindings: &Bindings) -> Result<Calculation, MathError> {
    let (expression, target) = match source.split_once("->") {
        Some((expr, target)) => {
            let target = target.trim();
            if unit(target).is_none() {
                return Err(error("Conversion target must be m, cm, mm, or km"));
            }
            (expr, Some(target))
        }
        None => (source, None),
    };
    let tokens = tokenize(expression)?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        depth: 0,
        bindings,
    };
    let quantity = parser.expression(0)?;
    if parser.cursor != parser.tokens.len() {
        return Err(parser.fail("Unexpected token; use explicit operators"));
    }
    let (display_value, display_unit) = if let Some(target) = target {
        if quantity.length_power != 1 {
            return Err(error(
                "Dimensional error: conversion requires a length (m^1)",
            ));
        }
        let value = quantity.si_value / unit(target).expect("validated target");
        if !value.is_finite() {
            return Err(error("Conversion overflow"));
        }
        (value, target.to_owned())
    } else {
        let dimension = match quantity.length_power {
            0 => String::new(),
            1 => "m".into(),
            n => format!("m^{n}"),
        };
        (quantity.si_value, dimension)
    };
    Ok(Calculation {
        binding: None,
        quantity,
        display_value,
        display_unit,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
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
                tokens.push(Token::Number(value));
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

struct Parser<'a> {
    tokens: Vec<Token>,
    cursor: usize,
    depth: usize,
    bindings: &'a Bindings,
}

impl Parser<'_> {
    fn fail(&self, message: &str) -> MathError {
        error(format!("{message} at token {}", self.cursor + 1))
    }

    // Powers bind more tightly than unary minus and associate rightward.
    // Implicit multiplication is allowed only before a known unit.
    fn expression(&mut self, minimum: u8) -> Result<Quantity, MathError> {
        self.depth += 1;
        if self.depth > 128 {
            return Err(self.fail("Expression nesting exceeds 128"));
        }
        let result = self.expression_inner(minimum);
        self.depth -= 1;
        result
    }

    fn expression_inner(&mut self, minimum: u8) -> Result<Quantity, MathError> {
        let token = self
            .tokens
            .get(self.cursor)
            .cloned()
            .ok_or_else(|| self.fail("Expected an expression"))?;
        self.cursor += 1;
        let mut left = match token {
            Token::Number(value) => Quantity::new(value, 0)?,
            Token::Name(name) => {
                if let Some(scale) = unit(&name) {
                    Quantity::new(scale, 1)?
                } else {
                    self.bindings
                        .get(&name)
                        .ok_or_else(|| {
                            error(format!(
                                "Undefined variable '{name}' (definitions must precede use)"
                            ))
                        })?
                        .clone()
                        .map_err(|_| {
                            error(format!(
                                "Definition '{name}' is invalid; correct its earlier definition"
                            ))
                        })?
                }
            }
            Token::Plus => self.expression(5)?,
            Token::Minus => {
                let value = self.expression(5)?;
                Quantity::new(-value.si_value, value.length_power)?
            }
            Token::Left => {
                let value = self.expression(0)?;
                if self.tokens.get(self.cursor) != Some(&Token::Right) {
                    return Err(self.fail("Expected ')'"));
                }
                self.cursor += 1;
                value
            }
            _ => return Err(self.fail("Expected a number, variable, or '('")),
        };
        loop {
            let implicit = matches!(self.tokens.get(self.cursor), Some(Token::Name(name)) if unit(name).is_some());
            let (left_bp, right_bp) = match self.tokens.get(self.cursor) {
                Some(Token::Plus | Token::Minus) => (1, 2),
                Some(Token::Star | Token::Slash) => (3, 4),
                Some(Token::Power) => (7, 6),
                _ if implicit => (3, 4),
                _ => break,
            };
            if left_bp < minimum {
                break;
            }
            let operator = if implicit {
                Token::Star
            } else {
                let t = self.tokens[self.cursor].clone();
                self.cursor += 1;
                t
            };
            let right = self.expression(right_bp)?;
            left = match operator {
                Token::Plus | Token::Minus => {
                    if left.length_power != right.length_power {
                        return Err(error(format!(
                            "Dimensional error: cannot add/subtract m^{} and m^{}",
                            left.length_power, right.length_power
                        )));
                    }
                    let value = if operator == Token::Plus {
                        left.si_value + right.si_value
                    } else {
                        left.si_value - right.si_value
                    };
                    Quantity::new(value, left.length_power)?
                }
                Token::Star => Quantity::new(
                    left.si_value * right.si_value,
                    left.length_power + right.length_power,
                )?,
                Token::Slash => {
                    if right.si_value == 0.0 {
                        return Err(error("Division by zero"));
                    }
                    Quantity::new(
                        left.si_value / right.si_value,
                        left.length_power - right.length_power,
                    )?
                }
                Token::Power => {
                    if right.length_power != 0
                        || right.si_value.fract() != 0.0
                        || !(-16.0..=16.0).contains(&right.si_value)
                    {
                        return Err(error(
                            "Exponent must be a dimensionless integer from -16 to 16",
                        ));
                    }
                    let exponent = right.si_value as i16;
                    if left.si_value == 0.0 && exponent <= 0 {
                        return Err(error("Zero to a non-positive power is undefined"));
                    }
                    Quantity::new(
                        left.si_value.powi(i32::from(exponent)),
                        left.length_power * exponent,
                    )?
                }
                _ => unreachable!(),
            };
        }
        Ok(left)
    }
}
