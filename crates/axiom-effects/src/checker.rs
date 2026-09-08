use std::collections::{HashMap, HashSet};
use std::fmt;

use axiom_ast::{Expr, FunctionDecl, Module};

/// An error found while validating a module's declared effect dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectError {
    /// A function names an effect that the module does not define.
    UndefinedEffect { function: String, effect: String },
    /// A function performs or propagates an effect absent from its `with` list.
    UndeclaredEffect {
        function: String,
        effect: String,
        source: String,
    },
    /// A function attempts an unauthorized external host call (e.g. std::fs).
    DisallowedExternalCall {
        function: String,
        target: String,
    },
}

impl fmt::Display for EffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndefinedEffect { function, effect } => write!(
                formatter,
                "function `{function}` declares undefined effect `{effect}`"
            ),
            Self::UndeclaredEffect {
                function,
                effect,
                source,
            } => write!(
                formatter,
                "function `{function}` requires effect `{effect}` through `{source}` but does not declare it"
            ),
            Self::DisallowedExternalCall { function, target } => write!(
                formatter,
                "function `{function}` makes disallowed external host call `{target}` without an effect wrapper"
            ),
        }
    }
}

impl std::error::Error for EffectError {}

/// Checks that every named effect exists and every call's effects are propagated.
///
/// Calling an effect operation directly requires that effect in the current
/// function's `with` list. Calling another function requires all effects from
/// the callee's `with` list.
pub fn check_module_effects(module: &Module) -> Result<(), EffectError> {
    let effects = module
        .effects
        .iter()
        .map(|effect| effect.name.as_str())
        .collect::<HashSet<_>>();

    for function in &module.functions {
        for effect in &function.effects {
            if !effects.contains(effect.as_str()) {
                return Err(EffectError::UndefinedEffect {
                    function: function.name.clone(),
                    effect: effect.clone(),
                });
            }
        }
    }

    let functions = module
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();

    let enums = module
        .types
        .iter()
        .filter_map(|ty| match ty {
            axiom_ast::TypeDef::Enum { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    for function in &module.functions {
        check_function(function, &functions, &effects, &enums)?;
    }

    Ok(())
}

fn check_function(
    function: &FunctionDecl,
    functions: &HashMap<&str, &FunctionDecl>,
    effects: &HashSet<&str>,
    enums: &HashSet<&str>,
) -> Result<(), EffectError> {
    let declared = function
        .effects
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut calls = Vec::new();
    collect_calls(&function.body, &mut calls);

    for call in calls {
        if call.len() == 2 && effects.contains(call[0].as_str()) {
            require_effect(function, &declared, &call[0], &call.join("."))?;
        } else if call.len() == 1 {
            if let Some(callee) = functions.get(call[0].as_str()) {
                for effect in &callee.effects {
                    require_effect(function, &declared, effect, &callee.name)?;
                }
            }
        } else if call.len() >= 2 {
            if enums.contains(call[0].as_str()) {
                continue;
            }
            if call[0] == "std" || call[0] == "core" || call[0] == "alloc" {
                return Err(EffectError::DisallowedExternalCall {
                    function: function.name.clone(),
                    target: call.join("::"),
                });
            }
        }
    }

    Ok(())
}

fn require_effect(
    function: &FunctionDecl,
    declared: &HashSet<&str>,
    effect: &str,
    source: &str,
) -> Result<(), EffectError> {
    if declared.contains(effect) {
        return Ok(());
    }

    Err(EffectError::UndeclaredEffect {
        function: function.name.clone(),
        effect: effect.to_owned(),
        source: source.to_owned(),
    })
}

fn collect_calls(expression: &Expr, calls: &mut Vec<Vec<String>>) {
    match expression {
        Expr::BinaryOp { left, right, .. } => {
            collect_calls(left, calls);
            collect_calls(right, calls);
        }
        Expr::UnaryOp { operand, .. } | Expr::Let { value: operand, .. } => {
            collect_calls(operand, calls);
        }
        Expr::Block { statements, tail } => {
            for statement in statements {
                collect_calls(statement, calls);
            }
            if let Some(tail) = tail {
                collect_calls(tail, calls);
            }
        }
        Expr::Call { callee, args } => {
            if let Some(path) = expression_path(callee) {
                calls.push(path);
            }
            collect_calls(callee, calls);
            for argument in args {
                collect_calls(argument, calls);
            }
        }
        Expr::FieldAccess { target, .. } => collect_calls(target, calls),
        Expr::RecordInit { fields, .. } => {
            for (_, value) in fields {
                collect_calls(value, calls);
            }
        }
        Expr::RecordUpdate { base, updates } => {
            collect_calls(base, calls);
            for (_, value) in updates {
                collect_calls(value, calls);
            }
        }
        Expr::Match { target, arms } => {
            collect_calls(target, calls);
            for arm in arms {
                collect_calls(&arm.body, calls);
            }
        }
        Expr::Raw(source) => collect_raw_calls(source, calls),
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) => {}
    }
}

fn expression_path(expression: &Expr) -> Option<Vec<String>> {
    match expression {
        Expr::Ident(name) => Some(vec![name.clone()]),
        Expr::FieldAccess { target, field } => {
            let mut path = expression_path(target)?;
            path.push(field.clone());
            Some(path)
        }
        _ => None,
    }
}

/// Raw bodies are retained by the parser for syntax it does not yet lower.
/// This lightweight scanner preserves effect safety until all control-flow
/// constructs have dedicated AST nodes.
fn collect_raw_calls(source: &str, calls: &mut Vec<Vec<String>>) {
    let bytes = source.as_bytes();
    let mut offset = 0;

    while offset < bytes.len() {
        if bytes[offset..].starts_with(b"//") {
            offset += 2;
            while offset < bytes.len() && bytes[offset] != b'\n' {
                offset += 1;
            }
        } else if bytes[offset..].starts_with(b"/*") {
            offset += 2;
            while offset + 1 < bytes.len() && !bytes[offset..].starts_with(b"*/") {
                offset += 1;
            }
            offset = (offset + 2).min(bytes.len());
        } else if bytes[offset] == b'"' {
            offset += 1;
            while offset < bytes.len() {
                match bytes[offset] {
                    b'\\' => offset = (offset + 2).min(bytes.len()),
                    b'"' => {
                        offset += 1;
                        break;
                    }
                    _ => offset += 1,
                }
            }
        } else if is_ident_start(bytes[offset]) {
            let mut path = Vec::new();
            loop {
                let start = offset;
                offset += 1;
                while offset < bytes.len() && is_ident_continue(bytes[offset]) {
                    offset += 1;
                }
                path.push(source[start..offset].to_owned());
                skip_ascii_whitespace(bytes, &mut offset);
                if offset < bytes.len() && bytes[offset] == b'.' {
                    offset += 1;
                    skip_ascii_whitespace(bytes, &mut offset);
                    if offset < bytes.len() && is_ident_start(bytes[offset]) {
                        continue;
                    }
                } else if offset + 1 < bytes.len() && bytes[offset] == b':' && bytes[offset + 1] == b':' {
                    offset += 2;
                    skip_ascii_whitespace(bytes, &mut offset);
                    if offset < bytes.len() && is_ident_start(bytes[offset]) {
                        continue;
                    }
                }
                break;
            }
            skip_ascii_whitespace(bytes, &mut offset);
            if offset < bytes.len() && bytes[offset] == b'(' {
                calls.push(path);
            }
        } else {
            offset += 1;
        }
    }
}

fn skip_ascii_whitespace(bytes: &[u8], offset: &mut usize) {
    while *offset < bytes.len() && bytes[*offset].is_ascii_whitespace() {
        *offset += 1;
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn is_ident_continue(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}
