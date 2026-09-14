//! Extraction of structured implementation tasks from Axiom holes.

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, UnaryOperator,
};
use axiom_parser::{parse_module, ParseError};
use serde::{Deserialize, Serialize};

const HOLE_TASK_SCHEMA: &str = "https://axiom-lang.org/schemas/hole-task.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoleParameter {
    pub name: String,
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contracts {
    pub requires: Vec<String>,
    pub ensures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoleTask {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub hole_id: String,
    pub target_function: String,
    pub parameters: Vec<HoleParameter>,
    pub return_type: String,
    pub allowed_effects: Vec<String>,
    pub contracts: Contracts,
}

impl HoleTask {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("HoleTask serialization cannot fail")
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("HoleTask serialization cannot fail")
    }
}

pub fn extract_holes(module: &Module) -> Vec<HoleTask> {
    module
        .functions
        .iter()
        .flat_map(|function| {
            let mut hole_ids = Vec::new();
            collect_hole_ids(&function.body, &mut hole_ids);
            hole_ids
                .into_iter()
                .map(|hole_id| build_task(function, hole_id))
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn extract_holes_from_source(source: &str) -> Result<Vec<HoleTask>, ParseError> {
    parse_module(source).map(|module| extract_holes(&module))
}

fn build_task(function: &FunctionDecl, hole_id: &str) -> HoleTask {
    HoleTask {
        schema: HOLE_TASK_SCHEMA.into(),
        hole_id: hole_id.into(),
        target_function: function.name.clone(),
        parameters: function
            .params
            .iter()
            .map(|parameter| HoleParameter {
                name: parameter.name.clone(),
                r#type: format_type(&parameter.ty),
            })
            .collect(),
        return_type: format_type(&function.return_type),
        allowed_effects: function.effects.clone(),
        contracts: Contracts {
            requires: function.requires.iter().map(format_expr).collect(),
            ensures: function.ensures.iter().map(format_expr).collect(),
        },
    }
}

fn collect_hole_ids<'a>(expression: &'a Expr, hole_ids: &mut Vec<&'a str>) {
    match expression {
        Expr::Hole(hole_id) => hole_ids.push(hole_id),
        Expr::BinaryOp { left, right, .. } => {
            collect_hole_ids(left, hole_ids);
            collect_hole_ids(right, hole_ids);
        }
        Expr::UnaryOp { operand, .. } | Expr::Let { value: operand, .. } => {
            collect_hole_ids(operand, hole_ids);
        }
        Expr::Block { statements, tail } => {
            for statement in statements {
                collect_hole_ids(statement, hole_ids);
            }
            if let Some(tail) = tail {
                collect_hole_ids(tail, hole_ids);
            }
        }
        Expr::Call { callee, args } => {
            collect_hole_ids(callee, hole_ids);
            for argument in args {
                collect_hole_ids(argument, hole_ids);
            }
        }
        Expr::FieldAccess { target, .. } => collect_hole_ids(target, hole_ids),
        Expr::RecordInit { fields, .. } => {
            for (_, value) in fields {
                collect_hole_ids(value, hole_ids);
            }
        }
        Expr::RecordUpdate { base, updates } => {
            collect_hole_ids(base, hole_ids);
            for (_, value) in updates {
                collect_hole_ids(value, hole_ids);
            }
        }
        Expr::Match { target, arms } => {
            collect_hole_ids(target, hole_ids);
            for arm in arms {
                collect_hole_ids(&arm.body, hole_ids);
            }
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::Raw(_) => {}
    }
}

fn format_type(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".into(),
        Type::Float => "Float".into(),
        Type::Decimal => "Decimal".into(),
        Type::Bool => "Bool".into(),
        Type::String => "String".into(),
        Type::Unit => "()".into(),
        Type::Custom(name) => name.clone(),
    }
}

fn format_expr(expression: &Expr) -> String {
    format_expr_at_precedence(expression, 0)
}

fn format_expr_at_precedence(expression: &Expr, parent_precedence: u8) -> String {
    let (text, precedence) = match expression {
        Expr::Literal(literal) => (format_literal(literal), 8),
        Expr::Ident(name) => (name.clone(), 8),
        Expr::Hole(hole_id) => (format!("?{hole_id}"), 8),
        Expr::BinaryOp { left, op, right } => {
            let (operator, precedence, right_precedence) = match op {
                BinaryOperator::Implies => ("=>", 1, 1),
                BinaryOperator::Or => ("||", 1, 2),
                BinaryOperator::And => ("&&", 2, 3),
                BinaryOperator::GreaterThan => (">", 3, 4),
                BinaryOperator::GreaterThanOrEqual => (">=", 3, 4),
                BinaryOperator::LessThan => ("<", 3, 4),
                BinaryOperator::LessThanOrEqual => ("<=", 3, 4),
                BinaryOperator::Equal => ("==", 3, 4),
                BinaryOperator::NotEqual => ("!=", 3, 4),
                BinaryOperator::Is => ("is", 3, 4),
                BinaryOperator::Add => ("+", 4, 5),
                BinaryOperator::Subtract => ("-", 4, 5),
                BinaryOperator::Multiply => ("*", 5, 6),
                BinaryOperator::Divide => ("/", 5, 6),
                BinaryOperator::Modulo => ("%", 5, 6),
            };
            (
                format!(
                    "{} {operator} {}",
                    format_expr_at_precedence(left, precedence),
                    format_expr_at_precedence(right, right_precedence)
                ),
                precedence,
            )
        }
        Expr::UnaryOp { op, operand } => {
            let operator = match op {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            };
            (
                format!("{operator}{}", format_expr_at_precedence(operand, 6)),
                6,
            )
        }
        Expr::Let { name, value } => (format!("let {name} = {}", format_expr(value)), 0),
        Expr::Block { statements, tail } => {
            let mut parts = statements.iter().map(format_expr).collect::<Vec<_>>();
            if let Some(tail) = tail {
                parts.push(format_expr(tail));
            }
            (format!("{{ {} }}", parts.join("; ")), 8)
        }
        Expr::Call { callee, args } => (
            format!(
                "{}({})",
                format_expr_at_precedence(callee, 7),
                args.iter().map(format_expr).collect::<Vec<_>>().join(", ")
            ),
            7,
        ),
        Expr::FieldAccess { target, field } => (
            format!("{}.{field}", format_expr_at_precedence(target, 7)),
            7,
        ),
        Expr::RecordInit { ty_name, fields } => {
            let name = ty_name.as_deref().unwrap_or("");
            let fields = fields
                .iter()
                .map(|(field, value)| format!("{field}: {}", format_expr(value)))
                .collect::<Vec<_>>()
                .join(", ");
            (format!("{name} {{ {fields} }}"), 8)
        }
        Expr::RecordUpdate { base, updates } => {
            let updates = updates
                .iter()
                .map(|(field, value)| format!("{field}: {}", format_expr(value)))
                .collect::<Vec<_>>()
                .join(", ");
            let separator = if updates.is_empty() { "" } else { ", " };
            (
                format!("{{ ...{}{separator}{updates} }}", format_expr(base)),
                8,
            )
        }
        Expr::Match { target, arms } => {
            let arms = arms
                .iter()
                .map(|arm| {
                    format!(
                        "{} => {}",
                        format_pattern(&arm.pattern),
                        format_expr(&arm.body)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            (format!("match {} {{ {arms} }}", format_expr(target)), 8)
        }
        Expr::Raw(source) => (source.clone(), 8),
    };

    if precedence < parent_precedence {
        format!("({text})")
    } else {
        text
    }
}

fn format_literal(literal: &Literal) -> String {
    match literal {
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) if value.fract() == 0.0 => format!("{value:.1}"),
        Literal::Float(value) => value.to_string(),
        Literal::Decimal(micros) => {
            let whole = micros / 1_000_000;
            let frac = (micros % 1_000_000).abs();
            format!("{whole}.{frac:06}d")
        }
        Literal::Bool(value) => value.to_string(),
        Literal::String(value) => {
            serde_json::to_string(value).expect("string literal serialization cannot fail")
        }
        Literal::Unit => "()".into(),
    }
}

fn format_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "_".into(),
        Pattern::Literal(literal) => format_literal(literal),
        Pattern::Ident(name) => name.clone(),
        Pattern::Variant {
            enum_name,
            variant_name,
            fields,
        } => {
            let prefix = enum_name
                .as_ref()
                .map(|name| format!("{name}."))
                .unwrap_or_default();
            if fields.is_empty() {
                format!("{prefix}{variant_name}")
            } else {
                format!(
                    "{prefix}{variant_name}({})",
                    fields
                        .iter()
                        .map(format_pattern)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}
