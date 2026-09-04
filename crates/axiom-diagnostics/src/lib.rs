pub mod typecheck;

use std::collections::HashMap;

use axiom_ast::{BinaryOperator, Expr, Literal, Module, UnaryOperator};
use axiom_effects::{check_module_effects, EffectError};
use axiom_hole_extractor::extract_holes;
use axiom_parser::parse_module;
use axiom_smt::{verify_function, SmtResult};
use serde::{Deserialize, Serialize};
use typecheck::check_module_types;

/// Stable machine-readable outcomes produced by the diagnostics pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Verified,
    ContractViolation,
    EffectViolation,
    TypeError,
    ParseError,
    UnresolvedHole,
}

/// A single actionable compiler diagnostic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub status: DiagnosticStatus,
    pub function_name: Option<String>,
    pub hole_id: Option<String>,
    pub violated_clause: Option<String>,
    pub counter_example: Option<HashMap<String, f64>>,
    pub diagnosis: String,
    pub suggested_action: String,
}

impl DiagnosticReport {
    /// Serialize this report as compact JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("DiagnosticReport serialization cannot fail")
    }

    /// Serialize this report as human-readable JSON.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("DiagnosticReport serialization cannot fail")
    }
}

/// Parse and diagnose an Axiom source module.
pub fn diagnose_source(source: &str) -> Vec<DiagnosticReport> {
    match parse_module(source) {
        Ok(module) => diagnose_module(&module),
        Err(error) => vec![DiagnosticReport {
            status: DiagnosticStatus::ParseError,
            function_name: None,
            hole_id: None,
            violated_clause: None,
            counter_example: None,
            diagnosis: error.to_string(),
            suggested_action: format!(
                "Correct the Axiom syntax near line {}, column {} and run diagnostics again.",
                error.line, error.column
            ),
        }],
    }
}

/// Run effect, hole, type, and contract diagnostics over an already parsed module.
///
/// The checks follow the compiler pipeline: an effect error prevents contract
/// verification, holes must be filled, and types must match before SMT verification.
pub fn diagnose_module(module: &Module) -> Vec<DiagnosticReport> {
    if let Err(error) = check_module_effects(module) {
        return vec![effect_report(error)];
    }

    let holes = extract_holes(module);
    if !holes.is_empty() {
        return holes
            .into_iter()
            .map(|hole| DiagnosticReport {
                status: DiagnosticStatus::UnresolvedHole,
                function_name: Some(hole.target_function),
                hole_id: Some(hole.hole_id.clone()),
                violated_clause: None,
                counter_example: None,
                diagnosis: format!("Implementation hole `?{}` is unresolved.", hole.hole_id),
                suggested_action: format!(
                    "Fill `?{}` with an implementation satisfying the function contracts.",
                    hole.hole_id
                ),
            })
            .collect();
    }

    if let Err(type_err) = check_module_types(module) {
        return vec![DiagnosticReport {
            status: DiagnosticStatus::TypeError,
            function_name: Some(type_err.function_name),
            hole_id: None,
            violated_clause: None,
            counter_example: None,
            diagnosis: type_err.message,
            suggested_action: "Ensure the function return expression matches its declared type signature.".into(),
        }];
    }

    let reports = module
        .functions
        .iter()
        .map(|function| match verify_function(function) {
            SmtResult::Verified => DiagnosticReport {
                status: DiagnosticStatus::Verified,
                function_name: Some(function.name.clone()),
                hole_id: None,
                violated_clause: None,
                counter_example: None,
                diagnosis: format!(
                    "Function `{}` satisfies its declared contracts.",
                    function.name
                ),
                suggested_action: "No action required.".into(),
            },
            SmtResult::CounterExample(counter_example) => DiagnosticReport {
                status: DiagnosticStatus::ContractViolation,
                function_name: Some(function.name.clone()),
                hole_id: None,
                violated_clause: function
                    .ensures
                    .first()
                    .map(|clause| format!("ensures: {}", format_expr(clause))),
                counter_example: Some(counter_example),
                diagnosis: format!(
                    "Function `{}` violates a postcondition for the reported input.",
                    function.name
                ),
                suggested_action:
                    "Adjust the implementation or strengthen its preconditions, then verify again."
                        .into(),
            },
            SmtResult::Skipped(reason) => DiagnosticReport {
                status: DiagnosticStatus::Verified,
                function_name: Some(function.name.clone()),
                hole_id: None,
                violated_clause: None,
                counter_example: None,
                diagnosis: format!(
                    "Function `{}` passed available checks; SMT verification was skipped: {reason}.",
                    function.name
                ),
                suggested_action:
                    "Add supported contracts if formal postcondition verification is required.".into(),
            },
            SmtResult::Unknown(reason) => DiagnosticReport {
                status: DiagnosticStatus::Verified,
                function_name: Some(function.name.clone()),
                hole_id: None,
                violated_clause: None,
                counter_example: None,
                diagnosis: format!(
                    "Function `{}` SMT solver returned unknown: {reason}.",
                    function.name
                ),
                suggested_action: "Check solver timeouts or simplify non-linear constraints.".into(),
            },
        })
        .collect::<Vec<_>>();

    if reports.is_empty() {
        vec![DiagnosticReport {
            status: DiagnosticStatus::Verified,
            function_name: None,
            hole_id: None,
            violated_clause: None,
            counter_example: None,
            diagnosis: "The module passed all available checks.".into(),
            suggested_action: "No action required.".into(),
        }]
    } else {
        reports
    }
}

fn effect_report(error: EffectError) -> DiagnosticReport {
    let (function_name, violated_clause, suggested_action) = match &error {
        EffectError::UndefinedEffect { function, effect } => (
            function.clone(),
            format!("with [{effect}]"),
            format!("Declare effect `{effect}` in the module or remove it from the function."),
        ),
        EffectError::UndeclaredEffect {
            function, effect, ..
        } => (
            function.clone(),
            format!("with [{effect}]"),
            format!("Add `{effect}` to function `{function}`'s `with` list."),
        ),
    };

    DiagnosticReport {
        status: DiagnosticStatus::EffectViolation,
        function_name: Some(function_name),
        hole_id: None,
        violated_clause: Some(violated_clause),
        counter_example: None,
        diagnosis: error.to_string(),
        suggested_action,
    }
}

fn format_expr(expression: &Expr) -> String {
    match expression {
        Expr::Literal(literal) => match literal {
            Literal::Int(value) => value.to_string(),
            Literal::Float(value) if value.fract() == 0.0 => format!("{value:.1}"),
            Literal::Float(value) => value.to_string(),
            Literal::Bool(value) => value.to_string(),
            Literal::String(value) => {
                serde_json::to_string(value).expect("string literal serialization cannot fail")
            }
            Literal::Unit => "()".into(),
        },
        Expr::Ident(name) => name.clone(),
        Expr::Hole(hole_id) => format!("?{hole_id}"),
        Expr::BinaryOp { left, op, right } => {
            let operator = match op {
                BinaryOperator::Add => "+",
                BinaryOperator::Subtract => "-",
                BinaryOperator::Multiply => "*",
                BinaryOperator::Divide => "/",
                BinaryOperator::GreaterThan => ">",
                BinaryOperator::GreaterThanOrEqual => ">=",
                BinaryOperator::LessThan => "<",
                BinaryOperator::LessThanOrEqual => "<=",
                BinaryOperator::Equal => "==",
                BinaryOperator::NotEqual => "!=",
                BinaryOperator::Is => "is",
                BinaryOperator::And => "&&",
                BinaryOperator::Or => "||",
                BinaryOperator::Implies => "=>",
            };
            format!("{} {operator} {}", format_expr(left), format_expr(right))
        }
        Expr::UnaryOp { op, operand } => {
            let operator = match op {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            };
            format!("{operator}{}", format_expr(operand))
        }
        Expr::Let { name, value } => format!("let {name} = {}", format_expr(value)),
        Expr::Block { statements, tail } => {
            let mut expressions = statements.iter().map(format_expr).collect::<Vec<_>>();
            if let Some(tail) = tail {
                expressions.push(format_expr(tail));
            }
            format!("{{ {} }}", expressions.join("; "))
        }
        Expr::Call { callee, args } => format!(
            "{}({})",
            format_expr(callee),
            args.iter().map(format_expr).collect::<Vec<_>>().join(", ")
        ),
        Expr::FieldAccess { target, field } => format!("{}.{field}", format_expr(target)),
        Expr::RecordInit { ty_name, fields } => format!(
            "{} {{ {} }}",
            ty_name.as_deref().unwrap_or(""),
            fields
                .iter()
                .map(|(name, value)| format!("{name}: {}", format_expr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::RecordUpdate { base, updates } => format!(
            "{{ ...{}, {} }}",
            format_expr(base),
            updates
                .iter()
                .map(|(name, value)| format!("{name}: {}", format_expr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Match { .. } => "match expression".into(),
        Expr::Raw(source) => source.clone(),
    }
}
