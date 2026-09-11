pub mod typecheck;

use std::collections::HashMap;

use axiom_ast::{BinaryOperator, Expr, Literal, Module, UnaryOperator};
use axiom_effects::{check_module_effects, EffectError};
use axiom_hole_extractor::extract_holes;
use axiom_parser::parse_module;
use axiom_smt::{verify_function_in_module, SmtResult};
use serde::{Deserialize, Serialize};
use typecheck::check_module_types;

/// Stable machine-readable outcomes produced by the diagnostics pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Proved,
    Tested,
    Sampled,
    ContractViolation,
    VacuousContract,
    Skipped,
    Unknown,
    TypeError,
    EffectViolation,
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
    #[serde(default)]
    pub has_contracts: bool,
    #[serde(default)]
    pub obligations_checked: usize,
    #[serde(default)]
    pub solver_version: Option<String>,
    #[serde(default)]
    pub solver_path: Option<String>,
    #[serde(default)]
    pub solver_sha256: Option<String>,
}

impl Default for DiagnosticReport {
    fn default() -> Self {
        Self {
            status: DiagnosticStatus::Unknown,
            function_name: None,
            hole_id: None,
            violated_clause: None,
            counter_example: None,
            diagnosis: String::new(),
            suggested_action: String::new(),
            has_contracts: false,
            obligations_checked: 0,
            solver_version: None,
            solver_path: None,
            solver_sha256: None,
        }
    }
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
            diagnosis: error.to_string(),
            suggested_action: format!(
                "Correct the Axiom syntax near line {}, column {} and run diagnostics again.",
                error.line, error.column
            ),
            ..Default::default()
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
                has_contracts: false,
                ..Default::default()
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
            suggested_action:
                "Ensure the function return expression matches its declared type signature.".into(),
            has_contracts: false,
            ..Default::default()
        }];
    }

    let reports = module
        .functions
        .iter()
        .map(|function| {
            let has_contracts = !function.requires.is_empty() || !function.ensures.is_empty();
            match verify_function_in_module(function, module) {
                SmtResult::Proved {
                    solver,
                    solver_path,
                    solver_sha256,
                    version,
                    obligations_checked,
                } => DiagnosticReport {
                    status: DiagnosticStatus::Proved,
                    function_name: Some(function.name.clone()),
                    hole_id: None,
                    violated_clause: None,
                    counter_example: None,
                    diagnosis: format!(
                        "Function `{}` formally proved by {} (version {}). Obligations checked: {}.",
                        function.name, solver, version, obligations_checked
                    ),
                    suggested_action: "No action required.".into(),
                    has_contracts,
                    obligations_checked,
                    solver_version: Some(version),
                    solver_path,
                    solver_sha256,
                },
                SmtResult::VacuousContract { reason } => DiagnosticReport {
                    status: DiagnosticStatus::VacuousContract,
                    function_name: Some(function.name.clone()),
                    hole_id: None,
                    violated_clause: None,
                    counter_example: None,
                    diagnosis: reason,
                    suggested_action: "Correct or relax the preconditions so that they are mathematically satisfiable.".into(),
                    has_contracts,
                    obligations_checked: 0,
                    solver_version: None,
                    solver_path: None,
                    solver_sha256: None,
                },
                SmtResult::Sampled { points_checked } => DiagnosticReport {
                    status: DiagnosticStatus::Sampled,
                    function_name: Some(function.name.clone()),
                    hole_id: None,
                    violated_clause: None,
                    counter_example: None,
                    diagnosis: format!(
                        "Function `{}` passed discrete grid test across {} points (heuristic test, not a mathematical proof).",
                        function.name, points_checked
                    ),
                    suggested_action: "Install Microsoft Z3 for full first-order SMT theorem proving.".into(),
                    has_contracts,
                    obligations_checked: points_checked,
                    solver_version: None,
                    solver_path: None,
                    solver_sha256: None,
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
                    has_contracts,
                    ..Default::default()
                },
                SmtResult::Skipped(reason) => DiagnosticReport {
                    status: DiagnosticStatus::Skipped,
                    function_name: Some(function.name.clone()),
                    hole_id: None,
                    violated_clause: None,
                    counter_example: None,
                    diagnosis: format!(
                        "Function `{}` SMT verification was skipped: {reason}.",
                        function.name
                    ),
                    suggested_action:
                        "Add supported contracts if formal postcondition verification is required.".into(),
                    has_contracts,
                    ..Default::default()
                },
                SmtResult::Unknown(reason) => DiagnosticReport {
                    status: DiagnosticStatus::Unknown,
                    function_name: Some(function.name.clone()),
                    hole_id: None,
                    violated_clause: None,
                    counter_example: None,
                    diagnosis: format!(
                        "Function `{}` SMT solver returned unknown: {reason}.",
                        function.name
                    ),
                    suggested_action: "Check solver timeouts or simplify non-linear constraints.".into(),
                    has_contracts,
                    ..Default::default()
                },
            }
        })
        .collect::<Vec<_>>();

    if reports.is_empty() {
        vec![DiagnosticReport {
            status: DiagnosticStatus::Skipped,
            function_name: None,
            hole_id: None,
            violated_clause: None,
            counter_example: None,
            diagnosis: "The module contains no functions to verify.".into(),
            suggested_action: "No action required.".into(),
            has_contracts: false,
            ..Default::default()
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
        EffectError::DisallowedExternalCall { function, target } => (
            function.clone(),
            format!("external call `{target}`"),
            format!("Remove unauthorized external host call `{target}` or wrap it behind a declared effect handler."),
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
        has_contracts: false,
        ..Default::default()
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
                BinaryOperator::Modulo => "%",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_proved_includes_solver_identity() {
        let source = r#"
            fn add(x: Int, y: Int) -> Int
                requires { x > 0, y > 0, x < 1000, y < 1000 }
                ensures { result > 0 }
            {
                x + y
            }
        "#;
        let reports = diagnose_source(source);
        assert_eq!(reports.len(), 1);
        let r = &reports[0];
        if r.status == DiagnosticStatus::Proved {
            assert!(r.solver_version.is_some());
            assert!(r.solver_path.is_some());
            let path_str = r.solver_path.as_ref().unwrap();
            assert!(!path_str.is_empty());
            assert!(r.solver_sha256.is_some());
            let sha = r.solver_sha256.as_ref().unwrap();
            assert_eq!(sha.len(), 64);
        }
    }
}
