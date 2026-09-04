//! Basic static type checker for Axiom AST.

use std::collections::HashMap;

use axiom_ast::{BinaryOperator, Expr, FunctionDecl, Literal, Module, Type, UnaryOperator};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub function_name: String,
    pub expected: Type,
    pub actual: Type,
    pub message: String,
}

pub fn check_module_types(module: &Module) -> Result<(), TypeError> {
    for func in &module.functions {
        check_function_types(func)?;
    }
    Ok(())
}

pub fn check_function_types(func: &FunctionDecl) -> Result<(), TypeError> {
    let mut env = HashMap::new();
    for p in &func.params {
        env.insert(p.name.clone(), p.ty.clone());
    }

    if let Some(inferred) = infer_expr_type(&func.body, &mut env) {
        if !is_type_compatible(&func.return_type, &inferred) {
            return Err(TypeError {
                function_name: func.name.clone(),
                expected: func.return_type.clone(),
                actual: inferred.clone(),
                message: format!(
                    "Function `{}` declared return type `{:?}`, but body evaluates to `{:?}`",
                    func.name, func.return_type, inferred
                ),
            });
        }
    }

    Ok(())
}

fn infer_expr_type(expr: &Expr, env: &mut HashMap<String, Type>) -> Option<Type> {
    match expr {
        Expr::Literal(Literal::Int(_)) => Some(Type::Int),
        Expr::Literal(Literal::Float(_)) => Some(Type::Float),
        Expr::Literal(Literal::Bool(_)) => Some(Type::Bool),
        Expr::Literal(Literal::String(_)) => Some(Type::String),
        Expr::Literal(Literal::Unit) => Some(Type::Unit),
        Expr::Ident(name) => env.get(name).cloned(),
        Expr::Let { name, value } => {
            let ty = infer_expr_type(value, env)?;
            env.insert(name.clone(), ty.clone());
            Some(ty)
        }
        Expr::Block { statements, tail } => {
            for s in statements {
                infer_expr_type(s, env);
            }
            if let Some(t) = tail {
                infer_expr_type(t, env)
            } else {
                Some(Type::Unit)
            }
        }
        Expr::UnaryOp { op, operand } => {
            let inner = infer_expr_type(operand, env)?;
            match op {
                UnaryOperator::Negate => Some(inner),
                UnaryOperator::Not => Some(Type::Bool),
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = infer_expr_type(left, env)?;
            let _r = infer_expr_type(right, env);
            match op {
                BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide => Some(l),
                BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterThanOrEqual
                | BinaryOperator::LessThan
                | BinaryOperator::LessThanOrEqual
                | BinaryOperator::And
                | BinaryOperator::Or
                | BinaryOperator::Implies
                | BinaryOperator::Is => Some(Type::Bool),
            }
        }
        Expr::Match { arms, .. } => {
            if let Some(first_arm) = arms.first() {
                infer_expr_type(&first_arm.body, env)
            } else {
                None
            }
        }
        Expr::RecordInit { ty_name, .. } => {
            ty_name.as_ref().map(|name| Type::Custom(name.clone()))
        }
        _ => None,
    }
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
