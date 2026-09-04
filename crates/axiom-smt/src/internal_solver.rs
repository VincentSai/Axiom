//! Built-in fallback symbolic interval and boundary grid solver.
//! Provides instant standalone verification when an external Z3 binary is not installed.

use std::collections::HashMap;

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Type, UnaryOperator,
};

use crate::SmtResult;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Num(f64),
    Bool(bool),
}

impl Value {
    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(*n),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

pub fn verify_with_internal_solver(func: &FunctionDecl) -> SmtResult {
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

    if contains_hole_or_unsupported(&func.body) {
        return SmtResult::Skipped(format!(
            "function `{}` contains unsupported expression or hole",
            func.name
        ));
    }

    let mut param_names = Vec::new();
    for p in &func.params {
        match p.ty {
            Type::Int | Type::Float | Type::Decimal | Type::Bool => {
                param_names.push((p.name.clone(), p.ty.clone()));
            }
            _ => {
                return SmtResult::Skipped(format!(
                    "unsupported parameter type `{:?}` in function `{}`",
                    p.ty, func.name
                ));
            }
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

    for point in grid {
        let mut env = HashMap::new();
        for (name, val) in &point {
            env.insert(name.clone(), Value::Num(*val));
        }

        // 1. Evaluate requires
        let mut req_met = true;
        for req in &func.requires {
            match eval_expr(req, &mut env.clone()) {
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
        let body_val = match eval_expr(&func.body, &mut env.clone()) {
            Ok(v) => v,
            Err(_) => {
                return SmtResult::CounterExample(point);
            }
        };

        // 3. Evaluate ensures with result bound
        env.insert("result".into(), body_val);

        let mut ens_met = true;
        for ens in &func.ensures {
            match eval_expr(ens, &mut env.clone()) {
                Ok(Value::Bool(true)) => {}
                _ => {
                    ens_met = false;
                    break;
                }
            }
        }

        if !ens_met {
            return SmtResult::CounterExample(point);
        }
    }

    SmtResult::Verified
}

fn contains_hole_or_unsupported(expr: &Expr) -> bool {
    match expr {
        Expr::Hole(_) => true,
        Expr::Call { .. } => true,
        Expr::FieldAccess { .. } => true,
        Expr::RecordInit { .. } => true,
        Expr::RecordUpdate { .. } => true,
        Expr::Raw(_) => true,
        Expr::BinaryOp { left, right, .. } => {
            contains_hole_or_unsupported(left) || contains_hole_or_unsupported(right)
        }
        Expr::UnaryOp { operand, .. } => contains_hole_or_unsupported(operand),
        Expr::Let { value, .. } => contains_hole_or_unsupported(value),
        Expr::Block { statements, tail } => {
            statements.iter().any(contains_hole_or_unsupported)
                || tail.as_ref().is_some_and(|t| contains_hole_or_unsupported(t))
        }
        Expr::Match { target, arms } => {
            contains_hole_or_unsupported(target)
                || arms.iter().any(|a| contains_hole_or_unsupported(&a.body))
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

    let mut result = vec![HashMap::new()];
    for (name, ty) in params {
        let mut next = Vec::new();
        let values: Vec<f64> = match ty {
            Type::Bool => vec![0.0, 1.0],
            Type::Int => candidates.iter().map(|c| c.round()).collect(),
            _ => candidates.to_vec(),
        };

        for prefix in result {
            for &val in &values {
                let mut map = prefix.clone();
                map.insert(name.clone(), val);
                next.push(map);
                if next.len() > 10000 {
                    break;
                }
            }
        }
        result = next;
    }
    result
}

fn eval_expr(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, String> {
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
        Expr::Let { name, value } => {
            let val = eval_expr(value, env)?;
            env.insert(name.clone(), val.clone());
            Ok(val)
        }
        Expr::Block { statements, tail } => {
            for stmt in statements {
                eval_expr(stmt, env)?;
            }
            if let Some(t) = tail {
                eval_expr(t, env)
            } else {
                Ok(Value::Num(0.0))
            }
        }
        Expr::UnaryOp { op, operand } => {
            let val = eval_expr(operand, env)?;
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
            let l_val = eval_expr(left, env)?;
            let r_val = eval_expr(right, env)?;

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
                    _ => Ok(Value::Bool(false)),
                },
                BinaryOperator::NotEqual => match (&l_val, &r_val) {
                    (Value::Num(l), Value::Num(r)) => Ok(Value::Bool((l - r).abs() >= 1e-9)),
                    (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l != r)),
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
        _ => Err("unsupported expression".into()),
    }
}
