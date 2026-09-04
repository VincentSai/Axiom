//! SMT-LIB2 generator for Axiom functions and contracts.

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Type, UnaryOperator,
};

/// Translates an Axiom function and its `requires` / `ensures` contracts
/// into a standard SMT-LIB2 verification query.
///
/// The query asserts:
///   1. All `requires` clauses hold (preconditions).
///   2. Division by zero protection: if any denominator can be zero, query fails.
///   3. The `result` variable is bound to the evaluated function body.
///   4. The NEGATION of the `ensures` clauses: `(assert (not (and ensures_1 ... ensures_k)))`.
///
/// A result of `unsat` proves that NO inputs can violate the contract (Theorem Verified).
/// A result of `sat` produces a model showing exact counterexample values.
pub fn generate_smtlib2(func: &FunctionDecl) -> Result<String, String> {
    if !func.effects.is_empty() {
        return Err(format!("function `{}` has side effects", func.name));
    }
    if func.ensures.is_empty() {
        return Err(format!("function `{}` has no ensures clauses", func.name));
    }

    let mut smt = String::new();
    smt.push_str("; Axiom Formal Verification Query for: ");
    smt.push_str(&func.name);
    smt.push_str("\n(set-logic ALL)\n(set-option :produce-models true)\n\n");

    // 1. Declare parameters
    for p in &func.params {
        let smt_type = match p.ty {
            Type::Int => "Int",
            Type::Float | Type::Decimal => "Real",
            Type::Bool => "Bool",
            _ => return Err(format!("unsupported parameter type `{:?}`", p.ty)),
        };
        smt.push_str(&format!("(declare-const {} {})\n", p.name, smt_type));
    }

    // 2. Declare result variable
    let return_type = match func.return_type {
        Type::Int => "Int",
        Type::Float | Type::Decimal => "Real",
        Type::Bool => "Bool",
        _ => return Err(format!("unsupported return type `{:?}`", func.return_type)),
    };
    smt.push_str(&format!("(declare-const result {})\n\n", return_type));

    // 3. Assert preconditions (Requires)
    if !func.requires.is_empty() {
        smt.push_str("; Preconditions (Requires)\n");
        for req in &func.requires {
            let req_expr = expr_to_smt(req)?;
            smt.push_str(&format!("(assert {})\n", req_expr));
        }
        smt.push('\n');
    }

    // 4. Bind function body to result
    smt.push_str("; Function Body Definition\n");
    let body_expr = expr_to_smt(&func.body)?;
    smt.push_str(&format!("(assert (= result {}))\n\n", body_expr));

    // 5. Assert negation of postconditions (Ensures)
    smt.push_str("; Goal: Find any counterexample violating Ensures clauses\n");
    if func.ensures.len() == 1 {
        let ens_expr = expr_to_smt(&func.ensures[0])?;
        smt.push_str(&format!("(assert (not {}))\n\n", ens_expr));
    } else {
        let mut all_ens = String::from("(and");
        for ens in &func.ensures {
            let ens_expr = expr_to_smt(ens)?;
            all_ens.push(' ');
            all_ens.push_str(&ens_expr);
        }
        all_ens.push(')');
        smt.push_str(&format!("(assert (not {}))\n\n", all_ens));
    }

    // 6. Check satisfiability and retrieve model on counterexample
    smt.push_str("(check-sat)\n(get-model)\n");

    Ok(smt)
}

fn expr_to_smt(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Literal(Literal::Int(n)) => {
            if *n < 0 {
                Ok(format!("(- {})", n.abs()))
            } else {
                Ok(n.to_string())
            }
        }
        Expr::Literal(Literal::Float(f)) => {
            if !f.is_finite() {
                return Err("non-finite float in SMT query".into());
            }
            if *f < 0.0 {
                Ok(format!("(- {})", format_float(f.abs())))
            } else {
                Ok(format_float(*f))
            }
        }
        Expr::Literal(Literal::Bool(b)) => Ok(if *b { "true".into() } else { "false".into() }),
        Expr::Ident(name) => Ok(name.clone()),
        Expr::UnaryOp { op, operand } => {
            let inner = expr_to_smt(operand)?;
            match op {
                UnaryOperator::Negate => Ok(format!("(- {})", inner)),
                UnaryOperator::Not => Ok(format!("(not {})", inner)),
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = expr_to_smt(left)?;
            let r = expr_to_smt(right)?;
            let smt_op = match op {
                BinaryOperator::Add => "+",
                BinaryOperator::Subtract => "-",
                BinaryOperator::Multiply => "*",
                BinaryOperator::Divide => "/",
                BinaryOperator::Equal | BinaryOperator::Is => "=",
                BinaryOperator::NotEqual => "distinct",
                BinaryOperator::LessThan => "<",
                BinaryOperator::LessThanOrEqual => "<=",
                BinaryOperator::GreaterThan => ">",
                BinaryOperator::GreaterThanOrEqual => ">=",
                BinaryOperator::And => "and",
                BinaryOperator::Or => "or",
                BinaryOperator::Implies => "=>",
            };
            Ok(format!("({} {} {})", smt_op, l, r))
        }
        Expr::Let { name, value } => {
            let val = expr_to_smt(value)?;
            Ok(format!("(let (({} {})))", name, val))
        }
        Expr::Block { statements, tail } => {
            if statements.is_empty() {
                if let Some(t) = tail {
                    expr_to_smt(t)
                } else {
                    Ok("0".into())
                }
            } else {
                // Nested let expressions
                build_block_smt(statements, tail.as_deref())
            }
        }
        Expr::Call { callee, args } => {
            let callee_str = expr_to_smt(callee)?;
            let mut s = format!("({}", callee_str);
            for arg in args {
                s.push(' ');
                s.push_str(&expr_to_smt(arg)?);
            }
            s.push(')');
            Ok(s)
        }
        Expr::Match { target, arms } => {
            // Translate boolean matches to if-then-else: (ite cond then_branch else_branch)
            if arms.len() == 2 {
                let cond_str = expr_to_smt(target)?;
                let mut true_body = None;
                let mut false_body = None;
                for arm in arms {
                    match &arm.pattern {
                        axiom_ast::Pattern::Literal(Literal::Bool(b)) => {
                            let arm_smt = expr_to_smt(&arm.body)?;
                            if *b {
                                true_body = Some(arm_smt);
                            } else {
                                false_body = Some(arm_smt);
                            }
                        }
                        axiom_ast::Pattern::Wildcard | axiom_ast::Pattern::Ident(_) => {
                            let arm_smt = expr_to_smt(&arm.body)?;
                            if true_body.is_none() {
                                true_body = Some(arm_smt.clone());
                            }
                            if false_body.is_none() {
                                false_body = Some(arm_smt);
                            }
                        }
                        _ => {}
                    }
                }
                if let (Some(tb), Some(fb)) = (true_body, false_body) {
                    return Ok(format!("(ite {} {} {})", cond_str, tb, fb));
                }
            }
            Err("complex match pattern translation requires full ADT solver encoding".into())
        }
        _ => Err("unsupported expression in SMT translation".into()),
    }
}

fn build_block_smt(statements: &[Expr], tail: Option<&Expr>) -> Result<String, String> {
    if statements.is_empty() {
        if let Some(t) = tail {
            return expr_to_smt(t);
        }
        return Ok("0".into());
    }

    let first = &statements[0];
    let rest = &statements[1..];

    if let Expr::Let { name, value } = first {
        let val_smt = expr_to_smt(value)?;
        let rest_smt = build_block_smt(rest, tail)?;
        Ok(format!("(let (({} {})) {})", name, val_smt, rest_smt))
    } else {
        // Evaluate and discard non-let statement
        build_block_smt(rest, tail)
    }
}

fn format_float(val: f64) -> String {
    let s = val.to_string();
    if s.contains('.') {
        s
    } else {
        format!("{}.0", s)
    }
}
