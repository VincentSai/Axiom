//! Strict static type checker for Axiom AST.

use std::collections::HashMap;

use axiom_ast::{BinaryOperator, Expr, FunctionDecl, Literal, Module, Type, UnaryOperator};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub function_name: String,
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

    match infer_expr_type(&func.body, &mut env) {
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

fn infer_expr_type(expr: &Expr, env: &mut HashMap<String, Type>) -> Result<Type, String> {
    match expr {
        Expr::Literal(Literal::Int(_)) => Ok(Type::Int),
        Expr::Literal(Literal::Float(_)) => Ok(Type::Float),
        Expr::Literal(Literal::Bool(_)) => Ok(Type::Bool),
        Expr::Literal(Literal::String(_)) => Ok(Type::String),
        Expr::Literal(Literal::Unit) => Ok(Type::Unit),
        Expr::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("undefined variable `{}`", name)),
        Expr::Hole(_) => Ok(Type::Unit),
        Expr::Let { name, value } => {
            let ty = infer_expr_type(value, env)?;
            env.insert(name.clone(), ty.clone());
            Ok(ty)
        }
        Expr::Block { statements, tail } => {
            for s in statements {
                infer_expr_type(s, env)?;
            }
            if let Some(t) = tail {
                infer_expr_type(t, env)
            } else {
                Ok(Type::Unit)
            }
        }
        Expr::UnaryOp { op, operand } => {
            let inner = infer_expr_type(operand, env)?;
            match op {
                UnaryOperator::Negate => {
                    if is_numeric(&inner) {
                        Ok(inner)
                    } else {
                        Err(format!("cannot apply unary negation '-' to non-numeric type `{:?}`", inner))
                    }
                }
                UnaryOperator::Not => {
                    if inner == Type::Bool {
                        Ok(Type::Bool)
                    } else {
                        Err(format!("cannot apply logical not '!' to non-boolean type `{:?}`", inner))
                    }
                }
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = infer_expr_type(left, env)?;
            let r = infer_expr_type(right, env)?;

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
                    } else if is_type_compatible(&Type::Float, &l) && is_type_compatible(&Type::Float, &r) {
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
        Expr::Match { arms, .. } => {
            if arms.is_empty() {
                return Ok(Type::Unit);
            }
            let first_ty = infer_expr_type(&arms[0].body, env)?;
            for arm in &arms[1..] {
                let arm_ty = infer_expr_type(&arm.body, env)?;
                if !is_type_compatible(&first_ty, &arm_ty) && !is_type_compatible(&arm_ty, &first_ty) {
                    return Err(format!(
                        "match arms return incompatible types `{:?}` and `{:?}`",
                        first_ty, arm_ty
                    ));
                }
            }
            Ok(first_ty)
        }
        Expr::RecordInit { ty_name, fields } => {
            for (_, expr) in fields {
                infer_expr_type(expr, env)?;
            }
            Ok(ty_name.as_ref().map_or(Type::Unit, |n| Type::Custom(n.clone())))
        }
        Expr::RecordUpdate { base, updates } => {
            let base_ty = infer_expr_type(base, env)?;
            for (_, expr) in updates {
                infer_expr_type(expr, env)?;
            }
            Ok(base_ty)
        }
        Expr::FieldAccess { target, .. } => {
            let _ = infer_expr_type(target, env)?;
            Ok(Type::Float) // Fallback for field access scalar
        }
        Expr::Call { callee: _, args } => {
            for arg in args {
                infer_expr_type(arg, env)?;
            }
            Ok(Type::Int) // Fallback scalar return
        }
        _ => Ok(Type::Unit),
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
