//! Transpilation from the Axiom AST to standalone Rust source.

use std::fmt;

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeDef, UnaryOperator,
};
use axiom_parser::{parse_module, ParseError};

#[derive(Debug, Clone, PartialEq)]
pub enum TranspileError {
    Parse(ParseError),
    ContainsHole(String),
    UnsupportedExpression(String),
}

impl fmt::Display for TranspileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "failed to parse Axiom source: {error}"),
            Self::ContainsHole(name) => {
                write!(formatter, "module contains unresolved hole `?{name}`")
            }
            Self::UnsupportedExpression(kind) => {
                write!(formatter, "cannot transpile unsupported expression: {kind}")
            }
        }
    }
}

impl std::error::Error for TranspileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ParseError> for TranspileError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

pub fn transpile_to_rust(module: &Module) -> Result<String, TranspileError> {
    let mut output = String::new();
    for definition in &module.types {
        output.push_str(&transpile_type_definition(definition));
        output.push('\n');
    }
    for function in &module.functions {
        output.push_str(&transpile_function(function, module)?);
        output.push('\n');
    }
    Ok(output)
}

fn transpile_type_definition(definition: &TypeDef) -> String {
    match definition {
        TypeDef::Record { name, fields } => {
            let fields = fields
                .iter()
                .map(|field| format!("    pub {}: {},", field.name, rust_type(&field.ty)))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "#[derive(Debug, Clone, PartialEq)]\npub struct {name} {{\n{fields}\n}}\n"
            )
        }
        TypeDef::Enum { name, variants } => {
            let variants = variants
                .iter()
                .map(|variant| {
                    if variant.fields.is_empty() {
                        format!("    {},", variant.name)
                    } else {
                        let fields = variant
                            .fields
                            .iter()
                            .map(|field| rust_type(&field.ty))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("    {}({fields}),", variant.name)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "#[derive(Debug, Clone, PartialEq)]\npub enum {name} {{\n{variants}\n}}\n"
            )
        }
    }
}

pub fn transpile_source_to_rust(source: &str) -> Result<String, TranspileError> {
    transpile_to_rust(&parse_module(source)?)
}

fn transpile_function(function: &FunctionDecl, module: &Module) -> Result<String, TranspileError> {
    let params = function
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, rust_type(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");

    let mut body_code = String::new();

    // 1. Generate requires preconditions
    for req in &function.requires {
        let req_str = transpile_expr(req, module)?;
        body_code.push_str(&format!(
            "    debug_assert!({req_str}, \"requires clause failed: {req_str}\");\n"
        ));
    }

    // 2. Generate function body & postconditions
    let body_expr = transpile_expr(&function.body, module)?;
    if function.ensures.is_empty() {
        if function.requires.is_empty() {
            body_code.push_str(&format!("    {body_expr}\n"));
        } else {
            body_code.push_str(&format!("    let result = {body_expr};\n    result\n"));
        }
    } else {
        body_code.push_str(&format!("    let result = {body_expr};\n"));
        for ens in &function.ensures {
            let ens_str = transpile_expr(ens, module)?;
            body_code.push_str(&format!(
                "    debug_assert!({ens_str}, \"ensures clause failed: {ens_str}\");\n"
            ));
        }
        body_code.push_str("    result\n");
    }

    Ok(format!(
        "pub fn {}({params}) -> {} {{\n{body_code}}}\n",
        function.name,
        rust_type(&function.return_type)
    ))
}

fn rust_type(ty: &Type) -> String {
    match ty {
        Type::Int => "i64".into(),
        Type::Float | Type::Decimal => "f64".into(),
        Type::Bool => "bool".into(),
        Type::String => "String".into(),
        Type::Unit => "()".into(),
        Type::Custom(name) => name.clone(),
    }
}

fn transpile_expr(expr: &Expr, module: &Module) -> Result<String, TranspileError> {
    match expr {
        Expr::Literal(literal) => Ok(transpile_literal(literal)),
        Expr::Ident(name) => Ok(name.clone()),
        Expr::Hole(name) => Err(TranspileError::ContainsHole(name.clone())),
        Expr::BinaryOp { left, op, right } => Ok(format!(
            "({} {} {})",
            transpile_expr(left, module)?,
            binary_operator(op),
            transpile_expr(right, module)?
        )),
        Expr::UnaryOp { op, operand } => Ok(format!(
            "({}{})",
            match op {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            },
            transpile_expr(operand, module)?
        )),
        Expr::Let { name, value } => Ok(format!("let {name} = {};", transpile_expr(value, module)?)),
        Expr::Block { statements, tail } => {
            let mut rendered = String::from("{\n");
            for statement in statements {
                rendered.push_str("        ");
                rendered.push_str(&transpile_expr(statement, module)?);
                if !matches!(statement, Expr::Let { .. }) {
                    rendered.push(';');
                }
                rendered.push('\n');
            }
            if let Some(tail) = tail {
                rendered.push_str("        ");
                rendered.push_str(&transpile_expr(tail, module)?);
                rendered.push('\n');
            }
            rendered.push_str("    }");
            Ok(rendered)
        }
        Expr::Call { callee, args } => {
            let callee_str = transpile_expr(callee, module)?;
            let args_str = args
                .iter()
                .map(|a| transpile_expr(a, module))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            Ok(format!("{callee_str}({args_str})"))
        }
        Expr::FieldAccess { target, field } => {
            let target_str = transpile_expr(target, module)?;
            Ok(format!("{target_str}.{field}"))
        }
        Expr::RecordInit { ty_name, fields } => {
            let name = ty_name.as_deref().unwrap_or("Record");
            let fields_str = fields
                .iter()
                .map(|(k, v)| Ok(format!("{k}: {}", transpile_expr(v, module)?)))
                .collect::<Result<Vec<_>, TranspileError>>()?
                .join(", ");
            Ok(format!("{name} {{ {fields_str} }}"))
        }
        Expr::RecordUpdate { base, updates } => {
            let base_str = transpile_expr(base, module)?;
            let updates_str = updates
                .iter()
                .map(|(k, v)| Ok(format!("{k}: {}", transpile_expr(v, module)?)))
                .collect::<Result<Vec<_>, TranspileError>>()?
                .join(", ");
            
            // Try to infer record type name from base if it's a known identifier in module
            let ty_name = find_record_type_for_expr(base, module).unwrap_or_else(|| "Point".to_string());
            if updates.is_empty() {
                Ok(format!("{ty_name} {{ ..{base_str} }}"))
            } else {
                Ok(format!("{ty_name} {{ {updates_str}, ..{base_str} }}"))
            }
        }
        Expr::Match { target, arms } => {
            let target_str = transpile_expr(target, module)?;
            let mut arms_rendered = Vec::new();
            for arm in arms {
                let pat_str = transpile_pattern(&arm.pattern);
                let body_str = transpile_expr(&arm.body, module)?;
                arms_rendered.push(format!("        {pat_str} => {body_str},"));
            }
            Ok(format!("match {target_str} {{\n{}\n    }}", arms_rendered.join("\n")))
        }
        Expr::Raw(s) => Ok(s.clone()),
    }
}

fn find_record_type_for_expr(base: &Expr, module: &Module) -> Option<String> {
    if let Expr::Ident(name) = base {
        for func in &module.functions {
            for param in &func.params {
                if &param.name == name {
                    if let Type::Custom(ty_name) = &param.ty {
                        return Some(ty_name.clone());
                    }
                }
            }
        }
    }
    None
}

fn transpile_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "_".into(),
        Pattern::Literal(lit) => transpile_literal(lit),
        Pattern::Ident(name) => name.clone(),
        Pattern::Variant {
            enum_name,
            variant_name,
            fields,
        } => {
            let prefix = match enum_name {
                Some(e) => format!("{e}::{variant_name}"),
                None => variant_name.clone(),
            };
            if fields.is_empty() {
                prefix
            } else {
                let fields_str = fields.iter().map(transpile_pattern).collect::<Vec<_>>().join(", ");
                format!("{prefix}({fields_str})")
            }
        }
    }
}

fn transpile_literal(literal: &Literal) -> String {
    match literal {
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) => format!("{value:?}"),
        Literal::Bool(value) => value.to_string(),
        Literal::String(value) => format!("String::from({value:?})"),
        Literal::Unit => "()".into(),
    }
}

fn binary_operator(operator: &BinaryOperator) -> &'static str {
    match operator {
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
        BinaryOperator::Is => "==",
        BinaryOperator::And => "&&",
        BinaryOperator::Or => "||",
        BinaryOperator::Implies => "=>",
    }
}
