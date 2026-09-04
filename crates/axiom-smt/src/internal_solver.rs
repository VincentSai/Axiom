//! Built-in fallback symbolic interval and boundary grid solver.
//! Provides instant standalone verification when an external Z3 binary is not installed.

use std::collections::HashMap;

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeDef, UnaryOperator,
};

use crate::SmtResult;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Bool(bool),
    Record(HashMap<String, Value>),
}

impl Value {
    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(*n),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Self::Record(_) => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Module-level context for resolving types and dependent pure functions during verification.
#[derive(Debug, Clone, Default)]
pub struct SolverContext<'a> {
    pub records: HashMap<String, Vec<(String, Type)>>,
    pub functions: HashMap<String, &'a FunctionDecl>,
}

impl<'a> SolverContext<'a> {
    pub fn from_module(module: &'a Module) -> Self {
        let mut records = HashMap::new();
        for ty_def in &module.types {
            if let TypeDef::Record { name, fields } = ty_def {
                records.insert(
                    name.clone(),
                    fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect(),
                );
            }
        }
        let mut functions = HashMap::new();
        for f in &module.functions {
            functions.insert(f.name.clone(), f);
        }
        Self { records, functions }
    }
}

pub fn verify_with_internal_solver(func: &FunctionDecl) -> SmtResult {
    verify_with_internal_solver_context(func, None)
}

pub fn verify_with_internal_solver_context(
    func: &FunctionDecl,
    ctx: Option<&SolverContext>,
) -> SmtResult {
    if !func.effects.is_empty() {
        return SmtResult::Skipped(format!(
            "function `{}` has side effects {:?}",
            func.name, func.effects
        ));
    }

    if func.ensures.is_empty() {
        return SmtResult::Skipped(format!(
            "function `{}` has no ensures clauses",
            func.name
        ));
    }

    if contains_hole_or_unsupported(&func.body, ctx) {
        return SmtResult::Skipped(format!(
            "function `{}` contains unsupported expression or hole",
            func.name
        ));
    }

    // Flatten parameters (scalars and record fields)
    let empty_records = HashMap::new();
    let records = ctx.map(|c| &c.records).unwrap_or(&empty_records);
    let mut param_names = Vec::new();

    for p in &func.params {
        if !flatten_param(&p.name, &p.ty, records, &mut param_names, 0) {
            return SmtResult::Skipped(format!(
                "unsupported parameter type `{:?}` in function `{}`",
                p.ty, func.name
            ));
        }
    }

    // Extract boundary and characteristic constants from requires, ensures, and body
    let mut constants = vec![
        0.0, 1.0, -1.0, 0.5, -0.5, 2.0, 10.0, 100.0, 1000.0, 1000000.0, -1000000.0,
    ];

    for req in &func.requires {
        extract_constants(req, &mut constants);
    }
    for ens in &func.ensures {
        extract_constants(ens, &mut constants);
    }
    extract_constants(&func.body, &mut constants);

    // Expand constants with neighborhoods and boundary shifts
    let mut candidates = Vec::new();
    for &c in &constants {
        candidates.push(c);
        candidates.push(c - 1.0);
        candidates.push(c + 1.0);
        candidates.push(c - 0.001);
        candidates.push(c + 0.001);
        candidates.push(c * 2.0);
        candidates.push(-c);
        if c != 0.0 {
            candidates.push(c / 2.0);
        }
    }
    candidates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    candidates.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

    let grid = generate_grid(&param_names, &candidates);

    for point in &grid {
        let mut env = HashMap::new();
        populate_env_records(point, &mut env);

        // 1. Evaluate requires
        let mut req_met = true;
        for req in &func.requires {
            match eval_expr(req, &mut env.clone(), ctx) {
                Ok(Value::Bool(true)) => {}
                _ => {
                    req_met = false;
                    break;
                }
            }
        }

        if !req_met {
            continue;
        }

        // 2. Evaluate body
        let body_val = match eval_expr(&func.body, &mut env.clone(), ctx) {
            Ok(v) => v,
            Err(_) => {
                return SmtResult::CounterExample(point.clone());
            }
        };

        // 3. Evaluate ensures with result bound
        env.insert("result".into(), body_val.clone());
        if let Value::Record(ref map) = body_val {
            flatten_record_into_env("result", map, &mut env);
        }

        let mut ens_met = true;
        for ens in &func.ensures {
            match eval_expr(ens, &mut env.clone(), ctx) {
                Ok(Value::Bool(true)) => {}
                _ => {
                    ens_met = false;
                    break;
                }
            }
        }

        if !ens_met {
            return SmtResult::CounterExample(point.clone());
        }
    }

    SmtResult::Sampled {
        points_checked: grid.len(),
    }
}

fn flatten_param(
    prefix: &str,
    ty: &Type,
    records: &HashMap<String, Vec<(String, Type)>>,
    out: &mut Vec<(String, Type)>,
    depth: usize,
) -> bool {
    if depth > 8 {
        return false;
    }
    match ty {
        Type::Int | Type::Float | Type::Decimal | Type::Bool => {
            out.push((prefix.to_string(), ty.clone()));
            true
        }
        Type::Custom(custom_name) => {
            if let Some(fields) = records.get(custom_name) {
                for (f_name, f_ty) in fields {
                    let field_prefix = format!("{}.{}", prefix, f_name);
                    if !flatten_param(&field_prefix, f_ty, records, out, depth + 1) {
                        return false;
                    }
                }
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn populate_env_records(point: &HashMap<String, f64>, env: &mut HashMap<String, Value>) {
    for (key, &val) in point {
        env.insert(key.clone(), Value::Num(val));
    }
    for (key, &val) in point {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() > 1 {
            insert_nested_record(env, &parts, Value::Num(val));
        }
    }
}

fn insert_nested_record(env: &mut HashMap<String, Value>, parts: &[&str], leaf: Value) {
    if parts.is_empty() {
        return;
    }
    let top_name = parts[0];
    let rest = &parts[1..];
    let mut current_top = match env.remove(top_name) {
        Some(Value::Record(map)) => map,
        _ => HashMap::new(),
    };
    insert_record_path(&mut current_top, rest, leaf);
    env.insert(top_name.to_string(), Value::Record(current_top));
}

fn insert_record_path(map: &mut HashMap<String, Value>, parts: &[&str], leaf: Value) {
    if parts.len() == 1 {
        map.insert(parts[0].to_string(), leaf);
    } else {
        let field = parts[0];
        let mut sub_map = match map.remove(field) {
            Some(Value::Record(m)) => m,
            _ => HashMap::new(),
        };
        insert_record_path(&mut sub_map, &parts[1..], leaf);
        map.insert(field.to_string(), Value::Record(sub_map));
    }
}

fn flatten_record_into_env(prefix: &str, map: &HashMap<String, Value>, env: &mut HashMap<String, Value>) {
    for (f_name, val) in map {
        let path = format!("{}.{}", prefix, f_name);
        env.insert(path.clone(), val.clone());
        if let Value::Record(ref sub) = val {
            flatten_record_into_env(&path, sub, env);
        }
    }
}

fn contains_hole_or_unsupported(expr: &Expr, ctx: Option<&SolverContext>) -> bool {
    match expr {
        Expr::Hole(_) => true,
        Expr::Raw(_) => true,
        Expr::FieldAccess { target, .. } => contains_hole_or_unsupported(target, ctx),
        Expr::RecordInit { fields, .. } => fields
            .iter()
            .any(|(_, e)| contains_hole_or_unsupported(e, ctx)),
        Expr::RecordUpdate { base, updates } => {
            contains_hole_or_unsupported(base, ctx)
                || updates
                    .iter()
                    .any(|(_, e)| contains_hole_or_unsupported(e, ctx))
        }
        Expr::Call { callee, args } => {
            if let Expr::Ident(name) = callee.as_ref() {
                if let Some(ctx) = ctx {
                    if let Some(target_func) = ctx.functions.get(name) {
                        if !target_func.effects.is_empty() {
                            return true;
                        }
                        return args
                            .iter()
                            .any(|a| contains_hole_or_unsupported(a, Some(ctx)));
                    }
                }
            }
            true
        }
        Expr::Match { target, arms } => {
            contains_hole_or_unsupported(target, ctx)
                || arms
                    .iter()
                    .any(|a| contains_hole_or_unsupported(&a.body, ctx))
        }
        Expr::BinaryOp { left, right, .. } => {
            contains_hole_or_unsupported(left, ctx) || contains_hole_or_unsupported(right, ctx)
        }
        Expr::UnaryOp { operand, .. } => contains_hole_or_unsupported(operand, ctx),
        Expr::Let { value, .. } => contains_hole_or_unsupported(value, ctx),
        Expr::Block { statements, tail } => {
            statements
                .iter()
                .any(|s| contains_hole_or_unsupported(s, ctx))
                || tail
                    .as_ref()
                    .is_some_and(|t| contains_hole_or_unsupported(t, ctx))
        }
        _ => false,
    }
}

fn extract_constants(expr: &Expr, list: &mut Vec<f64>) {
    match expr {
        Expr::Literal(Literal::Int(n)) => {
            list.push(*n as f64);
        }
        Expr::Literal(Literal::Float(f)) if f.is_finite() => {
            list.push(*f);
        }
        Expr::BinaryOp { left, right, .. } => {
            extract_constants(left, list);
            extract_constants(right, list);
        }
        Expr::UnaryOp { operand, .. } => {
            extract_constants(operand, list);
        }
        Expr::Let { value, .. } => {
            extract_constants(value, list);
        }
        Expr::Block { statements, tail } => {
            for s in statements {
                extract_constants(s, list);
            }
            if let Some(t) = tail {
                extract_constants(t, list);
            }
        }
        Expr::FieldAccess { target, .. } => {
            extract_constants(target, list);
        }
        Expr::RecordInit { fields, .. } => {
            for (_, f_expr) in fields {
                extract_constants(f_expr, list);
            }
        }
        Expr::RecordUpdate { base, updates } => {
            extract_constants(base, list);
            for (_, f_expr) in updates {
                extract_constants(f_expr, list);
            }
        }
        Expr::Call { args, .. } => {
            for a in args {
                extract_constants(a, list);
            }
        }
        Expr::Match { target, arms } => {
            extract_constants(target, list);
            for a in arms {
                extract_constants(&a.body, list);
            }
        }
        _ => {}
    }
}

fn generate_grid(
    params: &[(String, Type)],
    candidates: &[f64],
) -> Vec<HashMap<String, f64>> {
    if params.is_empty() {
        return vec![HashMap::new()];
    }

    let max_vals_per_param = match params.len() {
        1 => 100,
        2 => 40,
        3 => 15,
        4 => 8,
        _ => 3,
    };

    let mut sampled_candidates = Vec::new();
    let priority_keys = [0.0, 1.0, -1.0, 0.5, -0.5, 2.0, -2.0, 10.0, 100.0];
    for &pk in &priority_keys {
        if candidates.iter().any(|&c| (c - pk).abs() < 1e-6) {
            sampled_candidates.push(pk);
        }
    }
    for &c in candidates {
        if !sampled_candidates.iter().any(|&s| (s - c).abs() < 1e-6) {
            sampled_candidates.push(c);
        }
        if sampled_candidates.len() >= max_vals_per_param {
            break;
        }
    }

    let mut result = vec![HashMap::new()];
    for (name, ty) in params {
        let mut next = Vec::new();
        let values: Vec<f64> = match ty {
            Type::Bool => vec![0.0, 1.0],
            Type::Int => sampled_candidates.iter().map(|c| c.round()).collect(),
            _ => sampled_candidates.to_vec(),
        };

        for prefix in &result {
            for &val in &values {
                let mut map = prefix.clone();
                map.insert(name.clone(), val);
                next.push(map);
                if next.len() >= 5000 {
                    break;
                }
            }
            if next.len() >= 5000 {
                break;
            }
        }
        result = next;
    }
    result
}

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
    ctx: Option<&SolverContext>,
) -> Result<Value, String> {
    eval_expr_internal(expr, env, ctx, 0)
}

fn eval_expr_internal(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
    ctx: Option<&SolverContext>,
    depth: usize,
) -> Result<Value, String> {
    match expr {
        Expr::Literal(Literal::Int(n)) => Ok(Value::Num(*n as f64)),
        Expr::Literal(Literal::Float(f)) => Ok(Value::Num(*f)),
        Expr::Literal(Literal::Bool(b)) => Ok(Value::Bool(*b)),
        Expr::Literal(Literal::Unit) => Ok(Value::Num(0.0)),
        Expr::Literal(Literal::String(_)) => Err("strings unsupported in numeric SMT".into()),
        Expr::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("unknown variable `{name}`")),
        Expr::FieldAccess { target, field } => {
            // First try evaluating target to a Record Value
            if let Ok(Value::Record(fields)) = eval_expr_internal(target, env, ctx, depth + 1) {
                if let Some(v) = fields.get(field) {
                    return Ok(v.clone());
                }
            }
            // Second try resolving compound field access path like "p1.x"
            if let Some(path) = expr_to_field_path(expr) {
                if let Some(v) = env.get(&path) {
                    return Ok(v.clone());
                }
            }
            Err(format!("cannot access field `{field}` on expression"))
        }
        Expr::RecordInit { fields, .. } => {
            let mut map = HashMap::new();
            for (f_name, f_expr) in fields {
                let val = eval_expr_internal(f_expr, env, ctx, depth + 1)?;
                map.insert(f_name.clone(), val);
            }
            Ok(Value::Record(map))
        }
        Expr::RecordUpdate { base, updates } => {
            let base_val = eval_expr_internal(base, env, ctx, depth + 1)?;
            if let Value::Record(mut map) = base_val {
                for (f_name, f_expr) in updates {
                    let val = eval_expr_internal(f_expr, env, ctx, depth + 1)?;
                    map.insert(f_name.clone(), val);
                }
                Ok(Value::Record(map))
            } else {
                Err("cannot update non-record value".into())
            }
        }
        Expr::Match { target, arms } => {
            let target_val = eval_expr_internal(target, env, ctx, depth + 1)?;
            for arm in arms {
                let mut matched = false;
                let mut arm_env = env.clone();
                match &arm.pattern {
                    Pattern::Wildcard => {
                        matched = true;
                    }
                    Pattern::Literal(Literal::Bool(pb)) => {
                        if let Value::Bool(vb) = target_val {
                            if *pb == vb {
                                matched = true;
                            }
                        }
                    }
                    Pattern::Literal(Literal::Int(pi)) => {
                        if let Value::Num(vn) = target_val {
                            if (*pi as f64 - vn).abs() < 1e-9 {
                                matched = true;
                            }
                        }
                    }
                    Pattern::Literal(Literal::Float(pf)) => {
                        if let Value::Num(vn) = target_val {
                            if (*pf - vn).abs() < 1e-9 {
                                matched = true;
                            }
                        }
                    }
                    Pattern::Ident(name) => {
                        arm_env.insert(name.clone(), target_val.clone());
                        matched = true;
                    }
                    _ => {}
                }
                if matched {
                    return eval_expr_internal(&arm.body, &mut arm_env, ctx, depth + 1);
                }
            }
            Err("no pattern matched in match expression".into())
        }
        Expr::Call { callee, args } => {
            let fn_name = match callee.as_ref() {
                Expr::Ident(name) => name,
                _ => return Err("callee must be an identifier".into()),
            };
            let target_func = ctx
                .and_then(|c| c.functions.get(fn_name))
                .copied()
                .ok_or_else(|| format!("unknown or unresolvable function `{fn_name}`"))?;

            if !target_func.effects.is_empty() {
                return Err(format!("called function `{fn_name}` has side effects"));
            }
            if depth > 16 {
                return Err(format!("call stack overflow evaluating `{fn_name}`"));
            }

            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(eval_expr_internal(arg, env, ctx, depth + 1)?);
            }

            if arg_vals.len() != target_func.params.len() {
                return Err(format!(
                    "argument count mismatch for `{fn_name}`: expected {}, got {}",
                    target_func.params.len(),
                    arg_vals.len()
                ));
            }

            let mut sub_env = HashMap::new();
            for (param, val) in target_func.params.iter().zip(arg_vals.into_iter()) {
                sub_env.insert(param.name.clone(), val.clone());
                if let Value::Record(ref map) = val {
                    flatten_record_into_env(&param.name, map, &mut sub_env);
                }
            }

            eval_expr_internal(&target_func.body, &mut sub_env, ctx, depth + 1)
        }
        Expr::Let { name, value } => {
            let val = eval_expr_internal(value, env, ctx, depth + 1)?;
            env.insert(name.clone(), val.clone());
            if let Value::Record(ref map) = val {
                flatten_record_into_env(name, map, env);
            }
            Ok(val)
        }
        Expr::Block { statements, tail } => {
            for stmt in statements {
                eval_expr_internal(stmt, env, ctx, depth + 1)?;
            }
            if let Some(t) = tail {
                eval_expr_internal(t, env, ctx, depth + 1)
            } else {
                Ok(Value::Num(0.0))
            }
        }
        Expr::UnaryOp { op, operand } => {
            let val = eval_expr_internal(operand, env, ctx, depth + 1)?;
            match op {
                UnaryOperator::Negate => {
                    let num = val.as_f64().ok_or_else(|| "cannot negate non-number".to_string())?;
                    Ok(Value::Num(-num))
                }
                UnaryOperator::Not => {
                    let b = val.as_bool().ok_or_else(|| "cannot invert non-bool".to_string())?;
                    Ok(Value::Bool(!b))
                }
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l_val = eval_expr_internal(left, env, ctx, depth + 1)?;
            let r_val = eval_expr_internal(right, env, ctx, depth + 1)?;

            match op {
                BinaryOperator::Add => {
                    let l = l_val.as_f64().ok_or("type mismatch in +")?;
                    let r = r_val.as_f64().ok_or("type mismatch in +")?;
                    Ok(Value::Num(l + r))
                }
                BinaryOperator::Subtract => {
                    let l = l_val.as_f64().ok_or("type mismatch in -")?;
                    let r = r_val.as_f64().ok_or("type mismatch in -")?;
                    Ok(Value::Num(l - r))
                }
                BinaryOperator::Multiply => {
                    let l = l_val.as_f64().ok_or("type mismatch in *")?;
                    let r = r_val.as_f64().ok_or("type mismatch in *")?;
                    Ok(Value::Num(l * r))
                }
                BinaryOperator::Divide => {
                    let l = l_val.as_f64().ok_or("type mismatch in /")?;
                    let r = r_val.as_f64().ok_or("type mismatch in /")?;
                    if r == 0.0 {
                        Err("division by zero".into())
                    } else {
                        Ok(Value::Num(l / r))
                    }
                }
                BinaryOperator::GreaterThan => {
                    let l = l_val.as_f64().ok_or("type mismatch in >")?;
                    let r = r_val.as_f64().ok_or("type mismatch in >")?;
                    Ok(Value::Bool(l > r))
                }
                BinaryOperator::GreaterThanOrEqual => {
                    let l = l_val.as_f64().ok_or("type mismatch in >=")?;
                    let r = r_val.as_f64().ok_or("type mismatch in >=")?;
                    Ok(Value::Bool(l >= r))
                }
                BinaryOperator::LessThan => {
                    let l = l_val.as_f64().ok_or("type mismatch in <")?;
                    let r = r_val.as_f64().ok_or("type mismatch in <")?;
                    Ok(Value::Bool(l < r))
                }
                BinaryOperator::LessThanOrEqual => {
                    let l = l_val.as_f64().ok_or("type mismatch in <=")?;
                    let r = r_val.as_f64().ok_or("type mismatch in <=")?;
                    Ok(Value::Bool(l <= r))
                }
                BinaryOperator::Equal | BinaryOperator::Is => match (&l_val, &r_val) {
                    (Value::Num(l), Value::Num(r)) => Ok(Value::Bool((l - r).abs() < 1e-9)),
                    (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l == r)),
                    (Value::Record(l), Value::Record(r)) => Ok(Value::Bool(records_equal(l, r))),
                    _ => Ok(Value::Bool(false)),
                },
                BinaryOperator::NotEqual => match (&l_val, &r_val) {
                    (Value::Num(l), Value::Num(r)) => Ok(Value::Bool((l - r).abs() >= 1e-9)),
                    (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l != r)),
                    (Value::Record(l), Value::Record(r)) => Ok(Value::Bool(!records_equal(l, r))),
                    _ => Ok(Value::Bool(true)),
                },
                BinaryOperator::And => {
                    let l = l_val.as_bool().ok_or("type mismatch in &&")?;
                    let r = r_val.as_bool().ok_or("type mismatch in &&")?;
                    Ok(Value::Bool(l && r))
                }
                BinaryOperator::Or => {
                    let l = l_val.as_bool().ok_or("type mismatch in ||")?;
                    let r = r_val.as_bool().ok_or("type mismatch in ||")?;
                    Ok(Value::Bool(l || r))
                }
                BinaryOperator::Implies => {
                    let l = l_val.as_bool().ok_or("type mismatch in =>")?;
                    let r = r_val.as_bool().ok_or("type mismatch in =>")?;
                    Ok(Value::Bool(!l || r))
                }
            }
        }
        _ => Err("unsupported expression in internal solver".into()),
    }
}

fn expr_to_field_path(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(name) => Some(name.clone()),
        Expr::FieldAccess { target, field } => {
            let base = expr_to_field_path(target)?;
            Some(format!("{}.{}", base, field))
        }
        _ => None,
    }
}

fn records_equal(l: &HashMap<String, Value>, r: &HashMap<String, Value>) -> bool {
    if l.len() != r.len() {
        return false;
    }
    for (k, v_l) in l {
        match r.get(k) {
            Some(v_r) => match (v_l, v_r) {
                (Value::Num(n1), Value::Num(n2)) => {
                    if (*n1 - *n2).abs() >= 1e-9 {
                        return false;
                    }
                }
                (Value::Bool(b1), Value::Bool(b2)) => {
                    if b1 != b2 {
                        return false;
                    }
                }
                (Value::Record(r1), Value::Record(r2)) => {
                    if !records_equal(r1, r2) {
                        return false;
                    }
                }
                _ => return false,
            },
            None => return false,
        }
    }
    true
}
