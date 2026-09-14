//! Strict static type checker for Axiom AST.

use std::collections::HashMap;

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeDef, UnaryOperator,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub function_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModuleContext {
    pub records: HashMap<String, HashMap<String, Type>>,
    pub enums: HashMap<String, HashMap<String, Vec<Type>>>,
    pub functions: HashMap<String, (Vec<Type>, Type)>,
    pub effects: HashMap<String, HashMap<String, (Vec<Type>, Type)>>,
}

impl ModuleContext {
    pub fn from_module(module: &Module) -> Self {
        let mut ctx = ModuleContext::default();
        for ty_def in &module.types {
            match ty_def {
                TypeDef::Record { name, fields } => {
                    let mut field_map = HashMap::new();
                    for f in fields {
                        field_map.insert(f.name.clone(), f.ty.clone());
                    }
                    ctx.records.insert(name.clone(), field_map);
                }
                TypeDef::Enum { name, variants } => {
                    let mut var_map = HashMap::new();
                    for v in variants {
                        let f_types = v.fields.iter().map(|f| f.ty.clone()).collect();
                        var_map.insert(v.name.clone(), f_types);
                    }
                    ctx.enums.insert(name.clone(), var_map);
                }
            }
        }
        for eff in &module.effects {
            let mut op_map = HashMap::new();
            for op in &eff.operations {
                let param_types = op.params.iter().map(|p| p.ty.clone()).collect();
                op_map.insert(op.name.clone(), (param_types, op.return_type.clone()));
            }
            ctx.effects.insert(eff.name.clone(), op_map);
        }
        for func in &module.functions {
            let param_types = func.params.iter().map(|p| p.ty.clone()).collect();
            ctx.functions
                .insert(func.name.clone(), (param_types, func.return_type.clone()));
        }
        ctx
    }
}

pub fn check_module_types(module: &Module) -> Result<(), TypeError> {
    let ctx = ModuleContext::from_module(module);
    for func in &module.functions {
        check_function_types_with_ctx(func, &ctx)?;
    }
    Ok(())
}

pub fn check_function_types(func: &FunctionDecl) -> Result<(), TypeError> {
    let ctx = ModuleContext::default();
    check_function_types_with_ctx(func, &ctx)
}

pub fn check_function_types_with_ctx(
    func: &FunctionDecl,
    ctx: &ModuleContext,
) -> Result<(), TypeError> {
    if func.has_contracts() && func.contains_raw() {
        return Err(TypeError {
            function_name: func.name.clone(),
            message: format!(
                "Expr::Raw is forbidden in contracted functions to ensure formal verification integrity: `{}`",
                func.name
            ),
        });
    }

    let mut env = HashMap::new();
    for p in &func.params {
        env.insert(p.name.clone(), p.ty.clone());
    }

    // 1. Verify requires contracts evaluate to Bool
    for (i, req) in func.requires.iter().enumerate() {
        let mut req_env = env.clone();
        match infer_expr_type(req, &mut req_env, ctx) {
            Ok(ty) => {
                if ty != Type::Bool {
                    return Err(TypeError {
                        function_name: func.name.clone(),
                        message: format!(
                            "`requires` clause #{} must evaluate to Bool, got `{:?}`",
                            i + 1,
                            ty
                        ),
                    });
                }
            }
            Err(err_msg) => {
                return Err(TypeError {
                    function_name: func.name.clone(),
                    message: format!("type error in `requires` clause #{}: {}", i + 1, err_msg),
                });
            }
        }
    }

    // 2. Verify ensures contracts evaluate to Bool (with `result` in scope)
    for (i, ens) in func.ensures.iter().enumerate() {
        let mut ens_env = env.clone();
        ens_env.insert("result".to_string(), func.return_type.clone());
        match infer_expr_type(ens, &mut ens_env, ctx) {
            Ok(ty) => {
                if ty != Type::Bool {
                    return Err(TypeError {
                        function_name: func.name.clone(),
                        message: format!(
                            "`ensures` clause #{} must evaluate to Bool, got `{:?}`",
                            i + 1,
                            ty
                        ),
                    });
                }
            }
            Err(err_msg) => {
                return Err(TypeError {
                    function_name: func.name.clone(),
                    message: format!("type error in `ensures` clause #{}: {}", i + 1, err_msg),
                });
            }
        }
    }

    // 3. Verify function body evaluates to declared return type
    if matches!(func.body, Expr::Raw(_)) {
        // Non-contracted legacy functions with raw body are accepted without formal guarantee
        return Ok(());
    }

    match infer_expr_type(&func.body, &mut env, ctx) {
        Ok(inferred) => {
            if !is_type_compatible(&func.return_type, &inferred) {
                return Err(TypeError {
                    function_name: func.name.clone(),
                    message: format!(
                        "Function `{}` declared return type `{:?}`, but body evaluates to `{:?}`",
                        func.name, func.return_type, inferred
                    ),
                });
            }
        }
        Err(err_msg) => {
            return Err(TypeError {
                function_name: func.name.clone(),
                message: format!("Type error in function `{}`: {}", func.name, err_msg),
            });
        }
    }

    Ok(())
}

pub use axiom_ast::expr_contains_raw;

fn infer_expr_type(
    expr: &Expr,
    env: &mut HashMap<String, Type>,
    ctx: &ModuleContext,
) -> Result<Type, String> {
    match expr {
        Expr::Literal(Literal::Int(_)) => Ok(Type::Int),
        Expr::Literal(Literal::Float(_)) => Ok(Type::Float),
        Expr::Literal(Literal::Decimal(_)) => Ok(Type::Decimal),
        Expr::Literal(Literal::Bool(_)) => Ok(Type::Bool),
        Expr::Literal(Literal::String(_)) => Ok(Type::String),
        Expr::Literal(Literal::Unit) => Ok(Type::Unit),
        Expr::Ident(name) => {
            if let Some(ty) = env.get(name) {
                Ok(ty.clone())
            } else if name == "Ok" || name == "Err" {
                Ok(Type::Custom("Result".to_string()))
            } else if name == "Some" || name == "None" {
                Ok(Type::Custom("Option".to_string()))
            } else {
                // Check if it corresponds to an enum unit variant
                for (enum_name, variants) in &ctx.enums {
                    if let Some(fields) = variants.get(name) {
                        if fields.is_empty() {
                            return Ok(Type::Custom(enum_name.clone()));
                        }
                    }
                }
                Err(format!("undefined variable `{}`", name))
            }
        }
        Expr::Hole(_) => Ok(Type::Unit),
        Expr::Let { name, value } => {
            let ty = infer_expr_type(value, env, ctx)?;
            env.insert(name.clone(), ty.clone());
            Ok(ty)
        }
        Expr::Block { statements, tail } => {
            for s in statements {
                infer_expr_type(s, env, ctx)?;
            }
            if let Some(t) = tail {
                infer_expr_type(t, env, ctx)
            } else {
                Ok(Type::Unit)
            }
        }
        Expr::UnaryOp { op, operand } => {
            let inner = infer_expr_type(operand, env, ctx)?;
            match op {
                UnaryOperator::Negate => {
                    if is_numeric(&inner) {
                        Ok(inner)
                    } else {
                        Err(format!(
                            "cannot apply unary negation '-' to non-numeric type `{:?}`",
                            inner
                        ))
                    }
                }
                UnaryOperator::Not => {
                    if inner == Type::Bool {
                        Ok(Type::Bool)
                    } else {
                        Err(format!(
                            "cannot apply logical not '!' to non-boolean type `{:?}`",
                            inner
                        ))
                    }
                }
            }
        }
        Expr::BinaryOp { left, op, right } => {
            if *op == BinaryOperator::Is {
                let l = infer_expr_type(left, env, ctx)?;
                let is_variant_match = match right.as_ref() {
                    Expr::Ident(name) => {
                        let name_str = name.as_str();
                        if name_str == "Ok" || name_str == "Err" {
                            matches!(&l, Type::Custom(s) if s.starts_with("Result"))
                        } else if name_str == "Some" || name_str == "None" {
                            matches!(&l, Type::Custom(s) if s.starts_with("Option"))
                        } else if let Type::Custom(enum_name) = &l {
                            if let Some(variants) = ctx.enums.get(enum_name) {
                                variants.contains_key(name_str)
                            } else {
                                ctx.enums.values().any(|vars| vars.contains_key(name_str))
                            }
                        } else {
                            ctx.enums.values().any(|vars| vars.contains_key(name_str))
                        }
                    }
                    Expr::FieldAccess { target, field } => {
                        let field_str = field.as_str();
                        if let Expr::Ident(enum_name) = target.as_ref() {
                            if let Some(variants) = ctx.enums.get(enum_name) {
                                variants.contains_key(field_str)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if is_variant_match {
                    return Ok(Type::Bool);
                }

                let r = infer_expr_type(right, env, ctx)?;
                if l != r && !is_type_compatible(&l, &r) && !is_type_compatible(&r, &l) {
                    return Err(format!(
                        "cannot compare equality between incompatible types `{:?}` and `{:?}`",
                        l, r
                    ));
                }
                return Ok(Type::Bool);
            }

            let mut l = infer_expr_type(left, env, ctx)?;
            let mut r = infer_expr_type(right, env, ctx)?;

            // Contextual literal refinement for Decimal
            if l == Type::Decimal && matches!(right.as_ref(), Expr::Literal(Literal::Float(_))) {
                r = Type::Decimal;
            } else if r == Type::Decimal
                && matches!(left.as_ref(), Expr::Literal(Literal::Float(_)))
            {
                l = Type::Decimal;
            }

            // Strictly forbid mixing Decimal with Float
            if (l == Type::Decimal && r == Type::Float) || (l == Type::Float && r == Type::Decimal)
            {
                return Err(format!(
                    "cannot operate between incompatible types `{:?}` and `{:?}`: Decimal (fixed-point) cannot be mixed with Float (IEEE-754)",
                    l, r
                ));
            }

            match op {
                BinaryOperator::Add => {
                    if l == Type::String && r == Type::String {
                        return Ok(Type::String);
                    }
                    if !is_numeric(&l) || !is_numeric(&r) {
                        return Err(format!(
                            "cannot apply arithmetic operator to types `{:?}` and `{:?}`",
                            l, r
                        ));
                    }
                    if l == r {
                        Ok(l)
                    } else if l == Type::Decimal || r == Type::Decimal {
                        Ok(Type::Decimal)
                    } else if is_type_compatible(&Type::Float, &l)
                        && is_type_compatible(&Type::Float, &r)
                    {
                        Ok(Type::Float)
                    } else {
                        Ok(l)
                    }
                }
                BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Modulo => {
                    if !is_numeric(&l) || !is_numeric(&r) {
                        return Err(format!(
                            "cannot apply arithmetic operator to types `{:?}` and `{:?}`",
                            l, r
                        ));
                    }
                    if l == r {
                        Ok(l)
                    } else if l == Type::Decimal || r == Type::Decimal {
                        Ok(Type::Decimal)
                    } else if is_type_compatible(&Type::Float, &l)
                        && is_type_compatible(&Type::Float, &r)
                    {
                        Ok(Type::Float)
                    } else {
                        Ok(l)
                    }
                }
                BinaryOperator::GreaterThan
                | BinaryOperator::GreaterThanOrEqual
                | BinaryOperator::LessThan
                | BinaryOperator::LessThanOrEqual => {
                    if !is_numeric(&l) || !is_numeric(&r) {
                        return Err(format!(
                            "cannot apply comparison operator to types `{:?}` and `{:?}`",
                            l, r
                        ));
                    }
                    Ok(Type::Bool)
                }
                BinaryOperator::Equal | BinaryOperator::NotEqual => {
                    if l != r && !is_type_compatible(&l, &r) && !is_type_compatible(&r, &l) {
                        return Err(format!(
                            "cannot compare equality between incompatible types `{:?}` and `{:?}`",
                            l, r
                        ));
                    }
                    Ok(Type::Bool)
                }
                BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies => {
                    if l != Type::Bool || r != Type::Bool {
                        return Err(format!(
                            "logical operator requires Bool operands, got `{:?}` and `{:?}`",
                            l, r
                        ));
                    }
                    Ok(Type::Bool)
                }
                BinaryOperator::Is => unreachable!(),
            }
        }
        Expr::Match { target, arms } => {
            let target_ty = infer_expr_type(target, env, ctx)?;
            if arms.is_empty() {
                return Ok(Type::Unit);
            }
            let mut first_ty = None;
            for arm in arms {
                let mut arm_env = env.clone();
                bind_pattern_types(&arm.pattern, &target_ty, &mut arm_env, ctx)?;
                let arm_ty = infer_expr_type(&arm.body, &mut arm_env, ctx)?;
                if let Some(ref prev_ty) = first_ty {
                    if !is_type_compatible(prev_ty, &arm_ty)
                        && !is_type_compatible(&arm_ty, prev_ty)
                    {
                        return Err(format!(
                            "match arms return incompatible types `{:?}` and `{:?}`",
                            prev_ty, arm_ty
                        ));
                    }
                } else {
                    first_ty = Some(arm_ty);
                }
            }
            Ok(first_ty.unwrap_or(Type::Unit))
        }
        Expr::RecordInit { ty_name, fields } => {
            for (_, expr) in fields {
                infer_expr_type(expr, env, ctx)?;
            }
            if let Some(ref name) = ty_name {
                if let Some(expected_fields) = ctx.records.get(name) {
                    for (fname, fexpr) in fields {
                        let fty = infer_expr_type(fexpr, env, ctx)?;
                        if let Some(expected_ty) = expected_fields.get(fname) {
                            if !is_type_compatible(expected_ty, &fty) {
                                return Err(format!(
                                    "field `{}` of `{}` expects `{:?}`, got `{:?}`",
                                    fname, name, expected_ty, fty
                                ));
                            }
                        } else {
                            return Err(format!("record type `{}` has no field `{}`", name, fname));
                        }
                    }
                }
                Ok(Type::Custom(name.clone()))
            } else {
                Ok(Type::Unit)
            }
        }
        Expr::RecordUpdate { base, updates } => {
            let mut base_ty = infer_expr_type(base, env, ctx)?;
            for (_, expr) in updates {
                infer_expr_type(expr, env, ctx)?;
            }
            if !matches!(base_ty, Type::Custom(_)) {
                let update_field_names: Vec<&str> =
                    updates.iter().map(|(f, _)| f.as_str()).collect();
                for (rec_name, rec_fields) in &ctx.records {
                    if update_field_names
                        .iter()
                        .all(|f| rec_fields.contains_key(*f))
                    {
                        base_ty = Type::Custom(rec_name.clone());
                        break;
                    }
                }
            }
            if let Type::Custom(ref name) = base_ty {
                if let Some(expected_fields) = ctx.records.get(name) {
                    for (fname, fexpr) in updates {
                        let fty = infer_expr_type(fexpr, env, ctx)?;
                        if let Some(expected_ty) = expected_fields.get(fname) {
                            if !is_type_compatible(expected_ty, &fty) {
                                return Err(format!(
                                    "update field `{}` of `{}` expects `{:?}`, got `{:?}`",
                                    fname, name, expected_ty, fty
                                ));
                            }
                        } else {
                            return Err(format!("record type `{}` has no field `{}`", name, fname));
                        }
                    }
                }
            }
            Ok(base_ty)
        }
        Expr::FieldAccess { target, field } => {
            if let Expr::Ident(ref name) = **target {
                if let Some(variants) = ctx.enums.get(name) {
                    if variants.contains_key(field) {
                        return Ok(Type::Custom(name.clone()));
                    }
                }
                if let Some(ops) = ctx.effects.get(name) {
                    if let Some((_, ret_ty)) = ops.get(field) {
                        return Ok(ret_ty.clone());
                    }
                }
            }
            let target_ty = infer_expr_type(target, env, ctx)?;
            if let Type::Custom(ref name) = target_ty {
                if let Some(expected_fields) = ctx.records.get(name) {
                    if let Some(fty) = expected_fields.get(field) {
                        return Ok(fty.clone());
                    } else {
                        return Err(format!("record `{}` has no field `{}`", name, field));
                    }
                }
            }
            // Fallback for types not registered or opaque types
            Ok(Type::Float)
        }
        Expr::Call { callee, args } => {
            let mut arg_types = Vec::new();
            for arg in args {
                arg_types.push(infer_expr_type(arg, env, ctx)?);
            }
            if let Expr::Ident(ref callee_name) = **callee {
                if callee_name == "Ok" || callee_name == "Err" {
                    return Ok(Type::Custom("Result".to_string()));
                }
                if callee_name == "Some" || callee_name == "None" {
                    return Ok(Type::Custom("Option".to_string()));
                }
                for (enum_name, variants) in &ctx.enums {
                    if variants.contains_key(callee_name) {
                        return Ok(Type::Custom(enum_name.clone()));
                    }
                }
                if let Some((param_tys, ret_ty)) = ctx.functions.get(callee_name) {
                    if args.len() != param_tys.len() {
                        return Err(format!(
                            "function `{}` expects {} arguments, got {}",
                            callee_name,
                            param_tys.len(),
                            args.len()
                        ));
                    }
                    for (i, (expected_arg, actual_arg)) in
                        param_tys.iter().zip(arg_types.iter()).enumerate()
                    {
                        if !is_type_compatible(expected_arg, actual_arg) {
                            return Err(format!(
                                "argument #{} to `{}` expects `{:?}`, got `{:?}`",
                                i + 1,
                                callee_name,
                                expected_arg,
                                actual_arg
                            ));
                        }
                    }
                    return Ok(ret_ty.clone());
                }
            }
            if let Expr::FieldAccess { target, field } = callee.as_ref() {
                if field == "to_string" {
                    return Ok(Type::String);
                }
                if let Expr::Ident(ref target_name) = **target {
                    if let Some(ops) = ctx.effects.get(target_name) {
                        if let Some((_, ret_ty)) = ops.get(field) {
                            return Ok(ret_ty.clone());
                        }
                    }
                    if let Some(variants) = ctx.enums.get(target_name) {
                        if variants.contains_key(field) {
                            return Ok(Type::Custom(target_name.clone()));
                        }
                    }
                }
            }
            // If callee is unknown function in external/runtime scope, default to Int
            Ok(Type::Int)
        }
        Expr::Raw(_) => Err("Expr::Raw cannot be safely typechecked".into()),
    }
}

fn parse_type_str(s: &str) -> Type {
    match s.trim() {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "Decimal" => Type::Decimal,
        "Bool" => Type::Bool,
        "String" => Type::String,
        "()" => Type::Unit,
        other => Type::Custom(other.to_string()),
    }
}

fn extract_first_type_arg(ty: &Type) -> Option<Type> {
    if let Type::Custom(ref s) = ty {
        if let Some(start) = s.find('<') {
            if let Some(end) = s.rfind('>') {
                let inner = &s[start + 1..end];
                let first = inner.split(',').next()?.trim();
                return Some(parse_type_str(first));
            }
        }
    }
    None
}

fn extract_second_type_arg(ty: &Type) -> Option<Type> {
    if let Type::Custom(ref s) = ty {
        if let Some(start) = s.find('<') {
            if let Some(end) = s.rfind('>') {
                let inner = &s[start + 1..end];
                let mut parts = inner.split(',');
                let _ = parts.next()?;
                let second = parts.next()?.trim();
                return Some(parse_type_str(second));
            }
        }
    }
    None
}

fn bind_pattern_types(
    pattern: &Pattern,
    target_ty: &Type,
    env: &mut HashMap<String, Type>,
    ctx: &ModuleContext,
) -> Result<(), String> {
    match pattern {
        Pattern::Wildcard => Ok(()),
        Pattern::Literal(lit) => {
            let lit_ty = match lit {
                Literal::Int(_) => Type::Int,
                Literal::Float(_) => Type::Float,
                Literal::Decimal(_) => Type::Decimal,
                Literal::Bool(_) => Type::Bool,
                Literal::String(_) => Type::String,
                Literal::Unit => Type::Unit,
            };
            if !is_type_compatible(target_ty, &lit_ty) && !is_type_compatible(&lit_ty, target_ty) {
                return Err(format!(
                    "pattern literal `{:?}` incompatible with match target `{:?}`",
                    lit_ty, target_ty
                ));
            }
            Ok(())
        }
        Pattern::Ident(name) => {
            if let Type::Custom(enum_name) = target_ty {
                if let Some(variants) = ctx.enums.get(enum_name) {
                    if variants.contains_key(name) {
                        return Ok(());
                    }
                }
            }
            env.insert(name.clone(), target_ty.clone());
            Ok(())
        }
        Pattern::Variant {
            enum_name,
            variant_name,
            fields,
        } => {
            if variant_name == "Some" {
                if let Some(inner_ty) = extract_first_type_arg(target_ty) {
                    if let Some(first_field) = fields.first() {
                        return bind_pattern_types(first_field, &inner_ty, env, ctx);
                    }
                }
                for pat in fields {
                    if let Pattern::Ident(pname) = pat {
                        env.insert(pname.clone(), Type::Int);
                    }
                }
                return Ok(());
            }
            if variant_name == "None" {
                return Ok(());
            }
            if variant_name == "Ok" {
                if let Some(inner_ty) = extract_first_type_arg(target_ty) {
                    if let Some(first_field) = fields.first() {
                        return bind_pattern_types(first_field, &inner_ty, env, ctx);
                    }
                }
                return Ok(());
            }
            if variant_name == "Err" {
                if let Some(inner_ty) = extract_second_type_arg(target_ty) {
                    if let Some(first_field) = fields.first() {
                        return bind_pattern_types(first_field, &inner_ty, env, ctx);
                    }
                }
                return Ok(());
            }
            let enum_key = enum_name
                .as_ref()
                .unwrap_or(if let Type::Custom(n) = target_ty {
                    n
                } else {
                    variant_name
                });
            if let Some(variants) = ctx.enums.get(enum_key) {
                if let Some(field_tys) = variants.get(variant_name) {
                    if fields.len() != field_tys.len() {
                        return Err(format!(
                            "variant `{}` expects {} fields, got {}",
                            variant_name,
                            field_tys.len(),
                            fields.len()
                        ));
                    }
                    for (pat, fty) in fields.iter().zip(field_tys.iter()) {
                        bind_pattern_types(pat, fty, env, ctx)?;
                    }
                    return Ok(());
                }
            }
            for pat in fields {
                if let Pattern::Ident(pname) = pat {
                    env.insert(pname.clone(), Type::Int);
                }
            }
            Ok(())
        }
    }
}

fn is_numeric(ty: &Type) -> bool {
    matches!(ty, Type::Int | Type::Float | Type::Decimal)
}

fn is_type_compatible(expected: &Type, actual: &Type) -> bool {
    if expected == actual {
        return true;
    }
    match (expected, actual) {
        (Type::Float, Type::Int) | (Type::Decimal, Type::Int) => true,
        (Type::Custom(e), Type::Custom(a)) => {
            if e == a {
                return true;
            }
            if e.starts_with("Result") && a.starts_with("Result") {
                return true;
            }
            if e.starts_with("Option") && a.starts_with("Option") {
                return true;
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_ast::{BinaryOperator, Expr, FunctionDecl, FunctionParam, Literal, Type};

    #[test]
    fn contracted_function_rejects_expr_raw_as_body() {
        let func = FunctionDecl {
            name: "transfer".into(),
            params: vec![FunctionParam {
                name: "amount".into(),
                ty: Type::Int,
            }],
            return_type: Type::Int,
            effects: vec![],
            requires: vec![Expr::BinaryOp {
                op: BinaryOperator::GreaterThanOrEqual,
                left: Box::new(Expr::Ident("amount".into())),
                right: Box::new(Expr::Literal(Literal::Int(0))),
            }],
            ensures: vec![],
            body: Expr::Raw("amount + 10".into()),
        };
        let ctx = ModuleContext::default();
        let res = check_function_types_with_ctx(&func, &ctx);
        assert!(
            res.is_err(),
            "contracted function must reject Expr::Raw body"
        );
        let err = res.unwrap_err();
        assert!(
            err.message
                .contains("Expr::Raw is forbidden in contracted functions"),
            "expected error message about Expr::Raw forbidden, got: {}",
            err.message
        );
    }

    #[test]
    fn decimal_and_float_mixing_is_rejected() {
        let func = FunctionDecl {
            name: "mix_decimal_float".into(),
            params: vec![
                FunctionParam {
                    name: "amount".into(),
                    ty: Type::Decimal,
                },
                FunctionParam {
                    name: "rate".into(),
                    ty: Type::Float,
                },
            ],
            return_type: Type::Decimal,
            effects: vec![],
            requires: vec![],
            ensures: vec![],
            body: Expr::BinaryOp {
                op: BinaryOperator::Multiply,
                left: Box::new(Expr::Ident("amount".into())),
                right: Box::new(Expr::Ident("rate".into())),
            },
        };
        let ctx = ModuleContext::default();
        let res = check_function_types_with_ctx(&func, &ctx);
        assert!(res.is_err(), "mixing Decimal and Float must fail typecheck");
        let err = res.unwrap_err();
        assert!(
            err.message
                .contains("Decimal (fixed-point) cannot be mixed with Float (IEEE-754)"),
            "unexpected error message: {}",
            err.message
        );
    }
}
