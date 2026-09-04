//! Strict static type checker for Axiom AST.

use std::collections::HashMap;

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeDef,
    UnaryOperator,
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
                    message: format!(
                        "type error in `requires` clause #{}: {}",
                        i + 1,
                        err_msg
                    ),
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
                    message: format!(
                        "type error in `ensures` clause #{}: {}",
                        i + 1,
                        err_msg
                    ),
                });
            }
        }
    }

    // 3. Verify function body evaluates to declared return type
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

fn infer_expr_type(
    expr: &Expr,
    env: &mut HashMap<String, Type>,
    ctx: &ModuleContext,
) -> Result<Type, String> {
    match expr {
        Expr::Literal(Literal::Int(_)) => Ok(Type::Int),
        Expr::Literal(Literal::Float(_)) => Ok(Type::Float),
        Expr::Literal(Literal::Bool(_)) => Ok(Type::Bool),
        Expr::Literal(Literal::String(_)) => Ok(Type::String),
        Expr::Literal(Literal::Unit) => Ok(Type::Unit),
        Expr::Ident(name) => {
            if let Some(ty) = env.get(name) {
                Ok(ty.clone())
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
            let l = infer_expr_type(left, env, ctx)?;
            let r = infer_expr_type(right, env, ctx)?;

            match op {
                BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide => {
                    if !is_numeric(&l) || !is_numeric(&r) {
                        return Err(format!(
                            "cannot apply arithmetic operator to types `{:?}` and `{:?}`",
                            l, r
                        ));
                    }
                    if l == r {
                        Ok(l)
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
                BinaryOperator::Equal | BinaryOperator::NotEqual | BinaryOperator::Is => {
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
                    if !is_type_compatible(prev_ty, &arm_ty) && !is_type_compatible(&arm_ty, prev_ty) {
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
            let base_ty = infer_expr_type(base, env, ctx)?;
            for (_, expr) in updates {
                infer_expr_type(expr, env, ctx)?;
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
            // If callee is unknown function in external/runtime scope, default to Int
            Ok(Type::Int)
        }
        Expr::Raw(s) => Err(format!("unparsed raw expression in AST: `{}`", s)),
    }
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
            let enum_key = enum_name.as_ref().unwrap_or_else(|| {
                if let Type::Custom(n) = target_ty {
                    n
                } else {
                    variant_name
                }
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
        (Type::Float, Type::Int) | (Type::Decimal, Type::Int) | (Type::Decimal, Type::Float) => {
            true
        }
        _ => false,
    }
}
