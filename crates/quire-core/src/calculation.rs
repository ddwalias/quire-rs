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
pub(crate) fn error(message: impl Into<String>) -> MathError {
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

pub(crate) fn unit(name: &str) -> Option<f64> {
    match name {
        "m" => Some(1.0),
        "cm" => Some(0.01),
        "mm" => Some(0.001),
        "km" => Some(1000.0),
        _ => None,
    }
}
type Bindings = HashMap<String, Result<Quantity, MathError>>;
use crate::syntax::{self, BinaryOperator as Op, Expr};

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
    // Recover a valid definition name even if the RHS fails parsing, so a
    // previous definition cannot survive a failed redefinition.
    let (binding, _) = syntax::definition(source)?;
    let result = syntax::parse_math(source).and_then(|parsed| {
        let quantity = evaluate(&parsed.expression, bindings)?;
        let (display_value, display_unit) = if let Some(target) = parsed.display_unit {
            if quantity.length_power != 1 {
                return Err(error(
                    "Dimensional error: conversion requires a length (m^1)",
                ));
            }
            let value = quantity.si_value / unit(&target).expect("validated conversion target");
            if !value.is_finite() {
                return Err(error("Conversion overflow"));
            }
            (value, target)
        } else {
            let dimension = match quantity.length_power {
                0 => String::new(),
                1 => "m".into(),
                n => format!("m^{n}"),
            };
            (quantity.si_value, dimension)
        };
        Ok(Calculation {
            binding: binding.clone(),
            quantity,
            display_value,
            display_unit,
        })
    });
    if let Some(name) = binding {
        bindings.insert(
            name,
            result.as_ref().map(|c| c.quantity).map_err(Clone::clone),
        );
    }
    result
}

fn evaluate(expr: &Expr, bindings: &Bindings) -> Result<Quantity, MathError> {
    match expr {
        Expr::Number { value, .. } => Quantity::new(*value, 0),
        Expr::Symbol(name) => {
            if let Some(scale) = unit(name) {
                return Quantity::new(scale, 1);
            }
            bindings
                .get(name)
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
                })
        }
        Expr::Group(operand) => evaluate(operand, bindings),
        Expr::Unary { negative, operand } => {
            let value = evaluate(operand, bindings)?;
            Quantity::new(
                if *negative {
                    -value.si_value
                } else {
                    value.si_value
                },
                value.length_power,
            )
        }
        Expr::Binary {
            operator,
            left,
            right,
        } => {
            let left = evaluate(left, bindings)?;
            let right = evaluate(right, bindings)?;
            match operator {
                Op::Add | Op::Subtract => {
                    if left.length_power != right.length_power {
                        return Err(error(format!(
                            "Dimensional error: cannot add/subtract m^{} and m^{}",
                            left.length_power, right.length_power
                        )));
                    }
                    Quantity::new(
                        if *operator == Op::Add {
                            left.si_value + right.si_value
                        } else {
                            left.si_value - right.si_value
                        },
                        left.length_power,
                    )
                }
                Op::Multiply => Quantity::new(
                    left.si_value * right.si_value,
                    left.length_power + right.length_power,
                ),
                Op::Divide => {
                    if right.si_value == 0.0 {
                        return Err(error("Division by zero"));
                    }
                    Quantity::new(
                        left.si_value / right.si_value,
                        left.length_power - right.length_power,
                    )
                }
                Op::Power => {
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
                    )
                }
            }
        }
    }
}
