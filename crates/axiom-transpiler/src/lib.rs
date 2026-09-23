//! Transpilation from the Axiom AST to standalone Rust source.

use std::collections::HashMap;
use std::fmt;

use axiom_ast::{
    BinaryOperator, EffectDecl, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeDef,
    UnaryOperator,
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

    let has_decimal = module.types.iter().any(|t| match t {
        TypeDef::Record { fields, .. } => fields.iter().any(|f| f.ty == Type::Decimal),
        _ => false,
    }) || module
        .functions
        .iter()
        .any(|f| f.return_type == Type::Decimal || f.params.iter().any(|p| p.ty == Type::Decimal));

    if has_decimal {
        output.push_str("pub type Decimal = i64;\n\n");
    }

    for definition in &module.types {
        output.push_str(&transpile_type_definition(definition));
        output.push('\n');
    }
    for effect in &module.effects {
        output.push_str(&transpile_effect_definition(effect));
        output.push('\n');
    }

    let mut func_effects = HashMap::new();
    for f in &module.functions {
        func_effects.insert(f.name.as_str(), f.effects.as_slice());
    }

    let enum_names: Vec<&str> = module
        .types
        .iter()
        .filter_map(|t| match t {
            TypeDef::Enum { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();

    for function in &module.functions {
        output.push_str(&transpile_function(
            function,
            module,
            &func_effects,
            &enum_names,
        )?);
        output.push('\n');
    }
    Ok(output)
}

fn is_scalar_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Int | Type::Float | Type::Decimal | Type::Bool | Type::Unit
    )
}

fn transpile_type_definition(definition: &TypeDef) -> String {
    match definition {
        TypeDef::Record { name, fields } => {
            let all_scalar = fields.iter().all(|f| is_scalar_type(&f.ty));
            let derives = if all_scalar {
                "#[derive(Debug, Clone, Copy, PartialEq)]"
            } else {
                "#[derive(Debug, Clone, PartialEq)]"
            };
            let fields = fields
                .iter()
                .map(|field| format!("    pub {}: {},", field.name, rust_type(&field.ty)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{derives}\npub struct {name} {{\n{fields}\n}}\n")
        }
        TypeDef::Enum { name, variants } => {
            let all_scalar = variants
                .iter()
                .all(|v| v.fields.iter().all(|f| is_scalar_type(&f.ty)));
            let derives = if all_scalar {
                "#[derive(Debug, Clone, Copy, PartialEq)]"
            } else {
                "#[derive(Debug, Clone, PartialEq)]"
            };
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
            format!("{derives}\npub enum {name} {{\n{variants}\n}}\n")
        }
    }
}

pub fn transpile_source_to_rust(source: &str) -> Result<String, TranspileError> {
    transpile_to_rust(&parse_module(source)?)
}

fn transpile_effect_definition(effect: &EffectDecl) -> String {
    let mut ops = Vec::new();
    for op in &effect.operations {
        let mut params = vec!["&mut self".to_string()];
        for p in &op.params {
            params.push(format!("{}: {}", p.name, rust_type(&p.ty)));
        }
        let ret_str = if op.return_type == Type::Unit {
            String::new()
        } else {
            format!(" -> {}", rust_type(&op.return_type))
        };
        ops.push(format!(
            "    fn {}({}){};",
            op.name,
            params.join(", "),
            ret_str
        ));
    }
    format!("pub trait {} {{\n{}\n}}\n", effect.name, ops.join("\n"))
}

fn camel_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

#[derive(Clone)]
struct TranspileContext<'a> {
    module: &'a Module,
    current_effects: &'a [String],
    func_effects: &'a HashMap<&'a str, &'a [String]>,
    enum_names: &'a [&'a str],
    type_env: axiom_ast::TypeEnv<'a>,
}

fn transpile_function(
    function: &FunctionDecl,
    module: &Module,
    func_effects: &HashMap<&str, &[String]>,
    enum_names: &[&str],
) -> Result<String, TranspileError> {
    let mut params_vec: Vec<String> = function
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, rust_type(&param.ty)))
        .collect();

    for eff in &function.effects {
        let handler_name = camel_to_snake(eff);
        params_vec.push(format!("{handler_name}: &mut impl {eff}"));
    }

    let params = params_vec.join(", ");

    let type_env = axiom_ast::TypeEnv::new(module, Some(function));
    let ctx = TranspileContext {
        module,
        current_effects: &function.effects,
        func_effects,
        enum_names,
        type_env,
    };

    if function.has_contracts() && function.contains_raw() {
        return Err(TranspileError::UnsupportedExpression(format!(
            "Expr::Raw is forbidden in contracted function `{}` to guarantee formal verification soundness",
            function.name
        )));
    }

    let mut body_code = String::new();

    // 1. Generate requires preconditions
    for req in &function.requires {
        let req_str = transpile_expr(req, &ctx)?;
        body_code.push_str(&format!(
            "    debug_assert!({req_str}, \"requires clause failed: {req_str}\");\n"
        ));
    }

    // 2. Generate function body & postconditions
    let body_expr = transpile_expr(&function.body, &ctx)?;
    if function.ensures.is_empty() {
        if function.requires.is_empty() {
            body_code.push_str(&format!("    {body_expr}\n"));
        } else {
            body_code.push_str(&format!(
                "    let result = {{\n        {body_expr}\n    }};\n    result\n"
            ));
        }
    } else {
        body_code.push_str(&format!(
            "    let result = {{\n        {body_expr}\n    }};\n"
        ));
        for ens in &function.ensures {
            let ens_str = transpile_expr(ens, &ctx)?;
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
        Type::Float => "f64".into(),
        Type::Decimal => "Decimal".into(),
        Type::Bool => "bool".into(),
        Type::String => "String".into(),
        Type::Unit => "()".into(),
        Type::Custom(name) => rust_type_str(name),
    }
}

fn rust_type_str(s: &str) -> String {
    let s = s.trim();
    if s == "Int" {
        "i64".into()
    } else if s == "Float" {
        "f64".into()
    } else if s == "Decimal" {
        "Decimal".into()
    } else if s == "Bool" {
        "bool".into()
    } else if s == "String" {
        "String".into()
    } else if s == "()" || s == "Unit" {
        "()".into()
    } else if s.starts_with("List<") && s.ends_with('>') {
        let inner = &s[5..s.len() - 1];
        format!("Vec<{}>", rust_type_str(inner))
    } else if s.starts_with("Option<") && s.ends_with('>') {
        let inner = &s[7..s.len() - 1];
        format!("Option<{}>", rust_type_str(inner))
    } else if s.starts_with("Result<") && s.ends_with('>') {
        let inner = &s[7..s.len() - 1];
        if let Some(idx) = find_top_level_comma(inner) {
            let left = rust_type_str(&inner[..idx]);
            let right = rust_type_str(&inner[idx + 1..]);
            format!("Result<{left}, {right}>")
        } else {
            format!("Result<{}>", rust_type_str(inner))
        }
    } else {
        s.to_string()
    }
}

fn find_top_level_comma(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth -= 1,
            ',' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn transpile_expr(expr: &Expr, ctx: &TranspileContext) -> Result<String, TranspileError> {
    match expr {
        Expr::Literal(literal) => Ok(transpile_literal(literal)),
        Expr::Ident(name) => Ok(name.clone()),
        Expr::Hole(name) => Err(TranspileError::ContainsHole(name.clone())),
        Expr::BinaryOp { left, op, right } => {
            if matches!(op, BinaryOperator::Implies) {
                let left_str = transpile_expr(left, ctx)?;
                let right_str = transpile_expr(right, ctx)?;
                return Ok(format!("(!{left_str} || {right_str})"));
            }
            let mut left_str = transpile_expr(left, ctx)?;
            let mut right_str = transpile_expr(right, ctx)?;
            let mut left_ty = ctx.type_env.infer_type(left);
            let mut right_ty = ctx.type_env.infer_type(right);

            if left_ty == Some(Type::Decimal) {
                if let Expr::Literal(Literal::Float(f)) = right.as_ref() {
                    let micros = axiom_ast::float_to_decimal_micros(*f);
                    right_str = format!("{micros}_i64");
                    right_ty = Some(Type::Decimal);
                }
            } else if right_ty == Some(Type::Decimal) {
                if let Expr::Literal(Literal::Float(f)) = left.as_ref() {
                    let micros = axiom_ast::float_to_decimal_micros(*f);
                    left_str = format!("{micros}_i64");
                    left_ty = Some(Type::Decimal);
                }
            }

            match (left_ty, right_ty) {
                (Some(Type::Decimal), Some(Type::Decimal)) => match op {
                    BinaryOperator::Multiply => {
                        return Ok(format!(
                            "(((({left_str} as i128) * ({right_str} as i128)) / 1_000_000_i128) as i64)"
                        ));
                    }
                    BinaryOperator::Divide => {
                        return Ok(format!(
                            "(((({left_str} as i128) * 1_000_000_i128) / ({right_str} as i128)) as i64)"
                        ));
                    }
                    _ => {}
                },
                (Some(Type::Decimal), Some(Type::Int)) => match op {
                    BinaryOperator::Multiply => {
                        return Ok(format!(
                            "(((({left_str} as i128) * ({right_str} as i128))) as i64)"
                        ));
                    }
                    BinaryOperator::Divide => {
                        return Ok(format!(
                            "(((({left_str} as i128) / ({right_str} as i128))) as i64)"
                        ));
                    }
                    _ => {}
                },
                (Some(Type::Int), Some(Type::Decimal)) => match op {
                    BinaryOperator::Multiply => {
                        return Ok(format!(
                            "(((({left_str} as i128) * ({right_str} as i128))) as i64)"
                        ));
                    }
                    BinaryOperator::Divide => {
                        return Ok(format!(
                            "(((({left_str} as i128) * 1_000_000_000_000_i128) / ({right_str} as i128)) as i64)"
                        ));
                    }
                    _ => {}
                },
                _ => {}
            }

            Ok(format!("({left_str} {} {right_str})", binary_operator(op)))
        }
        Expr::UnaryOp { op, operand } => Ok(format!(
            "({}{})",
            match op {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            },
            transpile_expr(operand, ctx)?
        )),
        Expr::Let { name, value } => Ok(format!("let {name} = {};", transpile_expr(value, ctx)?)),
        Expr::Block { statements, tail } => {
            let mut block_ctx = ctx.clone();
            block_ctx.type_env.push_scope();
            let mut rendered = String::from("{\n");
            for statement in statements {
                rendered.push_str("        ");
                rendered.push_str(&transpile_expr(statement, &block_ctx)?);
                if let Expr::Let { name, value } = statement {
                    if let Some(v_ty) = block_ctx.type_env.infer_type(value) {
                        block_ctx.type_env.insert(name.clone(), v_ty);
                    }
                } else {
                    rendered.push(';');
                }
                rendered.push('\n');
            }
            if let Some(tail) = tail {
                rendered.push_str("        ");
                rendered.push_str(&transpile_expr(tail, &block_ctx)?);
                rendered.push('\n');
            }
            rendered.push_str("    }");
            Ok(rendered)
        }
        Expr::Call { callee, args } => {
            // Case 1: Effect method call, e.g. Render.clear_screen() or Input.get_movement_axis()
            if let Expr::FieldAccess { target, field } = callee.as_ref() {
                if let Expr::Ident(target_name) = target.as_ref() {
                    let is_effect = ctx.current_effects.iter().any(|e| e == target_name)
                        || ctx.module.effects.iter().any(|e| &e.name == target_name);
                    if is_effect {
                        let handler_name = camel_to_snake(target_name);
                        let args_str = args
                            .iter()
                            .map(|a| transpile_expr(a, ctx))
                            .collect::<Result<Vec<_>, _>>()?
                            .join(", ");
                        return Ok(format!("{handler_name}.{field}({args_str})"));
                    }
                    if ctx.enum_names.iter().any(|&e| e == target_name) {
                        let args_str = args
                            .iter()
                            .map(|a| transpile_expr(a, ctx))
                            .collect::<Result<Vec<_>, _>>()?
                            .join(", ");
                        return Ok(format!("{target_name}::{field}({args_str})"));
                    }
                }
            }

            // Case 2: Standard call or internal function call with effect propagation
            if let Expr::Ident(func_name) = callee.as_ref() {
                if let Err(err) = ctx.type_env.check_call_types(func_name, args) {
                    return Err(TranspileError::UnsupportedExpression(format!(
                        "call argument type mismatch: {err}"
                    )));
                }
            }

            let callee_str = transpile_expr(callee, ctx)?;
            let mut args_rendered = args
                .iter()
                .map(|a| transpile_expr(a, ctx))
                .collect::<Result<Vec<_>, _>>()?;

            if let Expr::Ident(func_name) = callee.as_ref() {
                if let Some(needed_effects) = ctx.func_effects.get(func_name.as_str()) {
                    for eff in *needed_effects {
                        if ctx.current_effects.iter().any(|e| e == eff) {
                            args_rendered.push(camel_to_snake(eff));
                        }
                    }
                }
            }

            let args_str = args_rendered.join(", ");
            Ok(format!("{callee_str}({args_str})"))
        }
        Expr::FieldAccess { target, field } => {
            if let Expr::Ident(target_name) = target.as_ref() {
                if ctx.enum_names.iter().any(|&e| e == target_name) {
                    return Ok(format!("{target_name}::{field}"));
                }
                if ctx.current_effects.iter().any(|e| e == target_name)
                    || ctx.module.effects.iter().any(|e| &e.name == target_name)
                {
                    let handler_name = camel_to_snake(target_name);
                    return Ok(format!("{handler_name}.{field}"));
                }
            }
            let target_str = transpile_expr(target, ctx)?;
            Ok(format!("{target_str}.{field}"))
        }
        Expr::RecordInit { ty_name, fields } => {
            let name = ty_name.as_deref().unwrap_or("Record");
            let fields_str = fields
                .iter()
                .map(|(k, v)| Ok(format!("{k}: {}", transpile_expr(v, ctx)?)))
                .collect::<Result<Vec<_>, TranspileError>>()?
                .join(", ");
            Ok(format!("{name} {{ {fields_str} }}"))
        }
        Expr::RecordUpdate { base, updates } => {
            let base_str = transpile_expr(base, ctx)?;
            let updates_str = updates
                .iter()
                .map(|(k, v)| Ok(format!("{k}: {}", transpile_expr(v, ctx)?)))
                .collect::<Result<Vec<_>, TranspileError>>()?
                .join(", ");

            // Try to infer record type name from base if it's a known identifier in module or by fields
            let ty_name =
                find_record_type_for_expr(base, updates, ctx.module).ok_or_else(|| {
                    TranspileError::UnsupportedExpression(format!(
                        "cannot infer record type for record update on `{base_str}`"
                    ))
                })?;
            if updates.is_empty() {
                Ok(format!("{ty_name} {{ ..{base_str} }}"))
            } else {
                Ok(format!("{ty_name} {{ {updates_str}, ..{base_str} }}"))
            }
        }
        Expr::Match { target, arms } => {
            let target_str = transpile_expr(target, ctx)?;
            let target_ty = ctx.type_env.infer_type(target);
            let mut arms_rendered = Vec::new();
            for arm in arms {
                let mut arm_ctx = ctx.clone();
                arm_ctx.type_env.push_scope();
                if let Pattern::Ident(ref id) = arm.pattern {
                    if let Some(ref ty) = target_ty {
                        arm_ctx.type_env.insert(id.clone(), ty.clone());
                    }
                }
                let pat_str = transpile_pattern(&arm.pattern);
                let body_str = transpile_expr(&arm.body, &arm_ctx)?;
                arms_rendered.push(format!("        {pat_str} => {body_str},"));
            }
            Ok(format!(
                "match {target_str} {{\n{}\n    }}",
                arms_rendered.join("\n")
            ))
        }
        Expr::Raw(s) => Ok(s.clone()),
    }
}

fn find_record_type_for_expr(
    base: &Expr,
    updates: &[(String, Expr)],
    module: &Module,
) -> Option<String> {
    if let Expr::Ident(name) = base {
        for func in &module.functions {
            for param in &func.params {
                if &param.name == name {
                    if let Type::Custom(ty_name) = &param.ty {
                        return Some(ty_name.clone());
                    }
                }
            }
            if let Some(ty_name) = find_type_in_block(&func.body, name) {
                return Some(ty_name);
            }
        }
    }

    let update_keys: Vec<&str> = updates.iter().map(|(k, _)| k.as_str()).collect();
    let mut candidate_records = Vec::new();
    for ty_def in &module.types {
        if let TypeDef::Record { name, fields } = ty_def {
            if update_keys
                .iter()
                .all(|k| fields.iter().any(|f| &f.name == k))
            {
                candidate_records.push(name.clone());
            }
        }
    }

    if candidate_records.len() == 1 {
        return Some(candidate_records.remove(0));
    }

    let all_records: Vec<String> = module
        .types
        .iter()
        .filter_map(|t| match t {
            TypeDef::Record { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    if all_records.len() == 1 {
        return Some(all_records[0].clone());
    }

    None
}

fn find_type_in_block(expr: &Expr, target_var: &str) -> Option<String> {
    match expr {
        Expr::Let { name, value } => {
            if name == target_var {
                if let Expr::RecordInit {
                    ty_name: Some(ref ty),
                    ..
                } = **value
                {
                    return Some(ty.clone());
                }
            }
            find_type_in_block(value, target_var)
        }
        Expr::Block { statements, tail } => {
            for stmt in statements {
                if let Some(found) = find_type_in_block(stmt, target_var) {
                    return Some(found);
                }
            }
            if let Some(tail_expr) = tail {
                find_type_in_block(tail_expr, target_var)
            } else {
                None
            }
        }
        _ => None,
    }
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
                let fields_str = fields
                    .iter()
                    .map(transpile_pattern)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{prefix}({fields_str})")
            }
        }
    }
}

fn transpile_literal(literal: &Literal) -> String {
    match literal {
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) => format!("{value:?}"),
        Literal::Decimal(value) => format!("{value}_i64"),
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
        BinaryOperator::Modulo => "%",
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
