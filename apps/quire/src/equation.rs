//! Small typesetter for the same expression tree used by the evaluator.
//! This lays out expressions; structural cursor editing is not implemented.
use eframe::egui::{self, Color32, FontFamily, FontId, Painter, Pos2, Vec2};
use quire_core::{
    Calculation,
    syntax::{BinaryOperator as Op, Expr, parse_math},
};
use std::sync::Arc;

pub const INK: Color32 = Color32::from_rgb(28, 36, 46);
pub const BLUE: Color32 = Color32::from_rgb(29, 78, 137);

enum Fragment {
    Text {
        offset: Vec2,
        galley: Arc<egui::Galley>,
        color: Color32,
    },
    Rule {
        from: Vec2,
        to: Vec2,
        color: Color32,
    },
}

pub struct Formula {
    pub size: Vec2,
    axis: f32,
    fragments: Vec<Fragment>,
}

impl Formula {
    fn text(painter: &Painter, text: &str, size: f32, color: Color32) -> Self {
        let font = FontId::new(size, FontFamily::Name("math".into()));
        let galley = painter.layout_no_wrap(text.to_owned(), font, color);
        Self {
            size: galley.size(),
            axis: galley.size().y * 0.52,
            fragments: vec![Fragment::Text {
                offset: Vec2::ZERO,
                galley,
                color,
            }],
        }
    }

    fn place(&mut self, child: Self, offset: Vec2) {
        self.fragments
            .extend(child.fragments.into_iter().map(|fragment| match fragment {
                Fragment::Text {
                    offset: p,
                    galley,
                    color,
                } => Fragment::Text {
                    offset: p + offset,
                    galley,
                    color,
                },
                Fragment::Rule { from, to, color } => Fragment::Rule {
                    from: from + offset,
                    to: to + offset,
                    color,
                },
            }));
    }

    fn row(children: Vec<Self>, gap: f32) -> Self {
        let axis = children.iter().map(|child| child.axis).fold(0.0, f32::max);
        let below = children
            .iter()
            .map(|child| child.size.y - child.axis)
            .fold(0.0, f32::max);
        let mut result = Self {
            size: Vec2::new(0.0, axis + below),
            axis,
            fragments: Vec::new(),
        };
        let count = children.len();
        for (index, child) in children.into_iter().enumerate() {
            let width = child.size.x;
            let offset = Vec2::new(result.size.x, axis - child.axis);
            result.place(child, offset);
            result.size.x += width + if index + 1 < count { gap } else { 0.0 };
        }
        result
    }

    fn fraction(numerator: Self, denominator: Self, gap: f32, color: Color32) -> Self {
        let width = numerator.size.x.max(denominator.size.x) + gap * 2.0;
        let rule_y = numerator.size.y + gap;
        let height = rule_y + gap + denominator.size.y;
        let mut result = Self {
            size: Vec2::new(width, height),
            axis: rule_y,
            fragments: Vec::new(),
        };
        let top = Vec2::new((width - numerator.size.x) / 2.0, 0.0);
        let bottom = Vec2::new((width - denominator.size.x) / 2.0, rule_y + gap);
        result.place(numerator, top);
        result.place(denominator, bottom);
        result.fragments.push(Fragment::Rule {
            from: Vec2::new(0.0, rule_y),
            to: Vec2::new(width, rule_y),
            color,
        });
        result
    }

    fn power(base: Self, exponent: Self) -> Self {
        let base_y = exponent.size.y * 0.65;
        let exponent_x = base.size.x + 1.5;
        let mut result = Self {
            size: Vec2::new(
                exponent_x + exponent.size.x,
                (base_y + base.size.y).max(exponent.size.y),
            ),
            axis: base_y + base.axis,
            fragments: Vec::new(),
        };
        result.place(base, Vec2::new(0.0, base_y));
        result.place(exponent, Vec2::new(exponent_x, 0.0));
        result
    }

    pub fn paint(&self, painter: &Painter, origin: Pos2, scale: f32) {
        for fragment in &self.fragments {
            match fragment {
                Fragment::Text {
                    offset,
                    galley,
                    color,
                } => {
                    painter.galley(origin + *offset, galley.clone(), *color);
                }
                Fragment::Rule { from, to, color } => {
                    painter.line_segment(
                        [origin + *from, origin + *to],
                        egui::Stroke::new(scale.max(0.75), *color),
                    );
                }
            }
        }
    }
}

fn italic_identifier(source: &str) -> String {
    source
        .chars()
        .map(|c| match c {
            'h' => '\u{210e}',
            'a'..='z' => char::from_u32(0x1d44e + c as u32 - 'a' as u32).unwrap(),
            'A'..='Z' => char::from_u32(0x1d434 + c as u32 - 'A' as u32).unwrap(),
            _ => c,
        })
        .collect()
}

fn expression(painter: &Painter, expr: &Expr, size: f32, color: Color32) -> Formula {
    match expr {
        Expr::Number { source, .. } => Formula::text(painter, source, size, color),
        Expr::Symbol(name) => {
            let is_unit = matches!(name.as_str(), "m" | "cm" | "mm" | "km");
            Formula::text(
                painter,
                &if is_unit {
                    name.clone()
                } else {
                    italic_identifier(name)
                },
                size,
                if is_unit { BLUE } else { color },
            )
        }
        Expr::Unary { negative, operand } => Formula::row(
            vec![
                Formula::text(painter, if *negative { "−" } else { "+" }, size, color),
                expression(painter, operand, size, color),
            ],
            2.0,
        ),
        Expr::Group(operand) => {
            let child = expression(painter, operand, size, color);
            let bracket_size = size.max(child.size.y * 0.95);
            Formula::row(
                vec![
                    Formula::text(painter, "(", bracket_size, color),
                    child,
                    Formula::text(painter, ")", bracket_size, color),
                ],
                1.0,
            )
        }
        Expr::Binary {
            operator: Op::Divide,
            left,
            right,
        } => Formula::fraction(
            expression(painter, left, size * 0.9, color),
            expression(painter, right, size * 0.9, color),
            size * 0.13,
            color,
        ),
        Expr::Binary {
            operator: Op::Power,
            left,
            right,
        } => Formula::power(
            expression(painter, left, size, color),
            expression(painter, right, (size * 0.67).max(10.0), color),
        ),
        Expr::Binary {
            operator,
            left,
            right,
        } => Formula::row(
            vec![
                expression(painter, left, size, color),
                Formula::text(
                    painter,
                    match operator {
                        Op::Add => "+",
                        Op::Subtract => "−",
                        Op::Multiply => "·",
                        _ => unreachable!(),
                    },
                    size,
                    color,
                ),
                expression(painter, right, size, color),
            ],
            size * 0.16,
        ),
    }
}

pub fn formula(
    painter: &Painter,
    source: &str,
    result: Option<&Calculation>,
    size: f32,
) -> Result<Formula, quire_core::MathError> {
    let parsed = parse_math(source)?;
    let mut parts = Vec::new();
    if let Some(binding) = &parsed.binding {
        parts.push(Formula::text(
            painter,
            &italic_identifier(binding),
            size,
            INK,
        ));
        parts.push(Formula::text(painter, ":=", size, INK));
    }
    parts.push(expression(painter, &parsed.expression, size, INK));
    if parsed.binding.is_none() {
        if let Some(result) = result {
            parts.push(Formula::text(painter, "=", size, INK));
            if let Ok(value) = parse_math(&result.display_value.to_string()) {
                let mut value_parts = vec![expression(painter, &value.expression, size, INK)];
                if !result.display_unit.is_empty()
                    && let Ok(unit) = parse_math(&result.display_unit)
                {
                    value_parts.push(expression(painter, &unit.expression, size, BLUE));
                }
                parts.push(Formula::row(value_parts, size * 0.2));
            }
        } else if let Some(unit) = &parsed.display_unit {
            parts.push(Formula::text(painter, &format!("→ {unit}"), size, BLUE));
        }
    }
    Ok(Formula::row(parts, size * 0.24))
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "STIX Two Math".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/STIXTwoMath-Regular.ttf"))
            .into(),
    );
    fonts.families.insert(
        FontFamily::Name("math".into()),
        vec!["STIX Two Math".into()],
    );
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .push("STIX Two Math".into());
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractions_have_a_rule_and_powers_raise_the_exponent_without_rewriting_source() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let fraction = formula(ui.painter(), "(a + b) / 2", None, 24.0).unwrap();
            assert!(
                fraction
                    .fragments
                    .iter()
                    .any(|fragment| matches!(fragment, Fragment::Rule { .. }))
            );
            let power = formula(ui.painter(), "x^2", None, 24.0).unwrap();
            let positions: Vec<_> = power
                .fragments
                .iter()
                .filter_map(|fragment| match fragment {
                    Fragment::Text { offset, .. } => Some(offset.y),
                    _ => None,
                })
                .collect();
            assert!(positions[0] > positions[1]);
            assert!(formula(ui.painter(), "sin(x)", None, 24.0).is_err());
        });
        output.textures_delta.clear();
    }
}
