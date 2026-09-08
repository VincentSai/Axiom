//! SMT-LIB2 generator for Axiom functions and contracts.

use std::collections::HashMap;

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeDef, UnaryOperator,
};

/// Context containing module records and function definitions.
#[derive(Debug, Clone, Default)]
pub struct SmtContext<'a> {
    pub records: HashMap<String, Vec<(String, Type)>>,
    pub functions: HashMap<String, &'a FunctionDecl>,
}

impl<'a> SmtContext<'a> {
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

/// Translates an Axiom function and its `requires` / `ensures` contracts
/// into a standard SMT-LIB2 verification query.
pub fn generate_smtlib2(func: &FunctionDecl) -> Result<String, String> {
    generate_smtlib2_with_module(func, None)
}

/// Translates an Axiom function with full module context into SMT-LIB2 format.
pub fn generate_smtlib2_with_module(
    func: &FunctionDecl,
    module: Option<&Module>,
) -> Result<String, String> {
    if !func.effects.is_empty() {
        return Err(format!("function `{}` has side effects", func.name));
    }
    if func.ensures.is_empty() {
        return Err(format!("function `{}` has no ensures clauses", func.name));
    }

    let empty_records = HashMap::new();
    let empty_functions = HashMap::new();
    let ctx = if let Some(m) = module {
        SmtContext::from_module(m)
    } else {
        SmtContext {
            records: empty_records,
            functions: empty_functions,
        }
    };

    let mut smt = String::new();
    smt.push_str("; Axiom Formal Verification Query for: ");
    smt.push_str(&func.name);
    smt.push_str("\n(reset)\n(set-logic ALL)\n(set-option :produce-models true)\n\n");

    // 1. Declare parameters (flattening records into scalar constants)
    let mut flat_params = Vec::new();
    for p in &func.params {
        flatten_type_to_smt(&p.name, &p.ty, &ctx.records, &mut flat_params, 0)?;
    }
    for (name, smt_ty) in &flat_params {
        smt.push_str(&format!("(declare-const {} {})\n", name, smt_ty));
    }

    // 2. Declare result variable (or flattened result record fields)
    let mut flat_results = Vec::new();
    flatten_type_to_smt("result", &func.return_type, &ctx.records, &mut flat_results, 0)?;
    for (name, smt_ty) in &flat_results {
        smt.push_str(&format!("(declare-const {} {})\n", name, smt_ty));
    }
    smt.push('\n');

    // 2.5 Physical representation bounds: 64-bit signed integer range to align with Rust backend
    smt.push_str("; Physical representation bounds: 64-bit signed integer range\n");
    for (name, smt_ty) in flat_params.iter().chain(flat_results.iter()) {
        if *smt_ty == "Int" {
            smt.push_str(&format!(
                "(assert (and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807)))\n",
                name, name
            ));
        }
    }
    smt.push('\n');

    // 3. Assert preconditions (Requires)
    if !func.requires.is_empty() {
        smt.push_str("; Preconditions (Requires)\n");
        for req in &func.requires {
            let inlined_req = inline_calls(req, &ctx, 0)?;
            let req_expr = expr_to_smt(&inlined_req)?;
            smt.push_str(&format!("(assert {})\n", req_expr));
        }
        smt.push('\n');
    }

    // 4. Bind function body to result
    smt.push_str("; Function Body Definition\n");
    let inlined_body = inline_calls(&func.body, &ctx, 0)?;
    if flat_results.len() == 1 && flat_results[0].0 == "result" {
        let body_expr = expr_to_smt(&inlined_body)?;
        smt.push_str(&format!("(assert (= result {}))\n\n", body_expr));
    } else {
        // Returned a record: bind individual fields if body is a RecordInit or let block
        bind_record_body_to_results(&inlined_body, &mut smt)?;
    }

    // 5. Assert negation of postconditions (Ensures)
    smt.push_str("; Goal: Find any counterexample violating Ensures clauses\n");
    if func.ensures.len() == 1 {
        let inlined_ens = inline_calls(&func.ensures[0], &ctx, 0)?;
        let ens_expr = expr_to_smt(&inlined_ens)?;
        smt.push_str(&format!("(assert (not {}))\n\n", ens_expr));
    } else {
        let mut all_ens = String::from("(and");
        for ens in &func.ensures {
            let inlined_ens = inline_calls(ens, &ctx, 0)?;
            let ens_expr = expr_to_smt(&inlined_ens)?;
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

fn flatten_type_to_smt(
    prefix: &str,
    ty: &Type,
    records: &HashMap<String, Vec<(String, Type)>>,
    out: &mut Vec<(String, &'static str)>,
    depth: usize,
) -> Result<(), String> {
    if depth > 8 {
        return Err("type nesting depth exceeded in SMT flattening".into());
    }
    match ty {
        Type::Int => {
            out.push((prefix.replace('.', "_"), "Int"));
            Ok(())
        }
        Type::Float | Type::Decimal => {
            out.push((prefix.replace('.', "_"), "Real"));
            Ok(())
        }
        Type::Bool => {
            out.push((prefix.replace('.', "_"), "Bool"));
            Ok(())
        }
        Type::Custom(name) => {
            if let Some(fields) = records.get(name) {
                for (f_name, f_ty) in fields {
                    let field_prefix = format!("{}.{}", prefix, f_name);
                    flatten_type_to_smt(&field_prefix, f_ty, records, out, depth + 1)?;
                }
                Ok(())
            } else {
                Err(format!("unsupported custom type `{}` in SMT export", name))
            }
        }
        _ => Err(format!("unsupported parameter type `{:?}` in SMT export", ty)),
    }
}

fn bind_record_body_to_results(body: &Expr, smt: &mut String) -> Result<(), String> {
    match body {
        Expr::RecordInit { fields, .. } => {
            for (f_name, f_expr) in fields {
                let val_smt = expr_to_smt(f_expr)?;
                smt.push_str(&format!("(assert (= result_{} {}))\n", f_name, val_smt));
            }
            smt.push('\n');
            Ok(())
        }
        Expr::Block { statements, tail } => {
            // If block contains let statements, wrap bindings around result asserts
            let mut let_bindings = Vec::new();
            for s in statements {
                if let Expr::Let { name, value } = s {
                    let val_smt = expr_to_smt(value)?;
                    let_bindings.push((name.clone(), val_smt));
                }
            }

            if let Some(tail_expr) = tail {
                if let Expr::RecordInit { fields, .. } = tail_expr.as_ref() {
                    for (f_name, f_expr) in fields {
                        let inner_val = expr_to_smt(f_expr)?;
                        let mut full_val = inner_val;
                        for (let_name, let_val) in let_bindings.iter().rev() {
                            full_val = format!("(let (({} {})) {})", let_name, let_val, full_val);
                        }
                        smt.push_str(&format!("(assert (= result_{} {}))\n", f_name, full_val));
                    }
                    smt.push('\n');
                    return Ok(());
                }
            }
            Err("record body must produce a record expression".into())
        }
        _ => Err("record function body must evaluate to a record".into()),
    }
}

fn inline_calls(expr: &Expr, ctx: &SmtContext, depth: usize) -> Result<Expr, String> {
    if depth > 16 {
        return Err("recursion limit exceeded while inlining SMT calls".into());
    }

    match expr {
        Expr::Call { callee, args } => {
            if let Expr::Ident(ref fn_name) = **callee {
                if let Some(target_func) = ctx.functions.get(fn_name) {
                    if !target_func.effects.is_empty() {
                        return Err(format!("cannot verify function calling effectful `{fn_name}`"));
                    }
                    let inlined_args: Vec<Expr> = args
                        .iter()
                        .map(|a| inline_calls(a, ctx, depth + 1))
                        .collect::<Result<_, _>>()?;

                    let mut subs = HashMap::new();
                    for (param, arg) in target_func.params.iter().zip(inlined_args.into_iter()) {
                        subs.insert(param.name.clone(), arg);
                    }
                    let replaced_body = substitute_expr(&target_func.body, &subs);
                    return inline_calls(&replaced_body, ctx, depth + 1);
                }
            }
            // If function is not known in module, keep as-is
            let inlined_callee = inline_calls(callee, ctx, depth + 1)?;
            let inlined_args = args
                .iter()
                .map(|a| inline_calls(a, ctx, depth + 1))
                .collect::<Result<_, _>>()?;
            Ok(Expr::Call {
                callee: Box::new(inlined_callee),
                args: inlined_args,
            })
        }
        Expr::FieldAccess { target, field } => {
            let inlined_target = inline_calls(target, ctx, depth)?;
            if let Expr::RecordInit { fields, .. } = &inlined_target {
                if let Some((_, f_expr)) = fields.iter().find(|(k, _)| k == field) {
                    return Ok(f_expr.clone());
                }
            }
            Ok(Expr::FieldAccess {
                target: Box::new(inlined_target),
                field: field.clone(),
            })
        }
        Expr::RecordInit { ty_name, fields } => {
            let new_fields = fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), inline_calls(v, ctx, depth)?)))
                .collect::<Result<_, String>>()?;
            Ok(Expr::RecordInit {
                ty_name: ty_name.clone(),
                fields: new_fields,
            })
        }
        Expr::RecordUpdate { base, updates } => {
            let new_base = inline_calls(base, ctx, depth)?;
            let new_updates = updates
                .iter()
                .map(|(k, v)| Ok((k.clone(), inline_calls(v, ctx, depth)?)))
                .collect::<Result<_, String>>()?;
            Ok(Expr::RecordUpdate {
                base: Box::new(new_base),
                updates: new_updates,
            })
        }
        Expr::BinaryOp { left, op, right } => Ok(Expr::BinaryOp {
            left: Box::new(inline_calls(left, ctx, depth)?),
            op: op.clone(),
            right: Box::new(inline_calls(right, ctx, depth)?),
        }),
        Expr::UnaryOp { op, operand } => Ok(Expr::UnaryOp {
            op: op.clone(),
            operand: Box::new(inline_calls(operand, ctx, depth)?),
        }),
        Expr::Let { name, value } => Ok(Expr::Let {
            name: name.clone(),
            value: Box::new(inline_calls(value, ctx, depth)?),
        }),
        Expr::Block { statements, tail } => {
            let new_stmts = statements
                .iter()
                .map(|s| inline_calls(s, ctx, depth))
                .collect::<Result<_, _>>()?;
            let new_tail = match tail {
                Some(t) => Some(Box::new(inline_calls(t, ctx, depth)?)),
                None => None,
            };
            Ok(Expr::Block {
                statements: new_stmts,
                tail: new_tail,
            })
        }
        Expr::Match { target, arms } => {
            let new_target = inline_calls(target, ctx, depth)?;
            let new_arms = arms
                .iter()
                .map(|arm| {
                    Ok(axiom_ast::MatchArm {
                        pattern: arm.pattern.clone(),
                        body: inline_calls(&arm.body, ctx, depth)?,
                    })
                })
                .collect::<Result<_, String>>()?;
            Ok(Expr::Match {
                target: Box::new(new_target),
                arms: new_arms,
            })
        }
        _ => Ok(expr.clone()),
    }
}

fn substitute_expr(expr: &Expr, subs: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Ident(name) => {
            if let Some(sub) = subs.get(name) {
                sub.clone()
            } else {
                Expr::Ident(name.clone())
            }
        }
        Expr::FieldAccess { target, field } => {
            let new_target = substitute_expr(target, subs);
            if let Expr::RecordInit { fields, .. } = &new_target {
                if let Some((_, f_expr)) = fields.iter().find(|(k, _)| k == field) {
                    return f_expr.clone();
                }
            }
            Expr::FieldAccess {
                target: Box::new(new_target),
                field: field.clone(),
            }
        }
        Expr::RecordInit { ty_name, fields } => {
            let new_fields = fields
                .iter()
                .map(|(k, v)| (k.clone(), substitute_expr(v, subs)))
                .collect();
            Expr::RecordInit {
                ty_name: ty_name.clone(),
                fields: new_fields,
            }
        }
        Expr::RecordUpdate { base, updates } => {
            let new_base = substitute_expr(base, subs);
            let new_updates = updates
                .iter()
                .map(|(k, v)| (k.clone(), substitute_expr(v, subs)))
                .collect();
            Expr::RecordUpdate {
                base: Box::new(new_base),
                updates: new_updates,
            }
        }
        Expr::Call { callee, args } => {
            let new_callee = substitute_expr(callee, subs);
            let new_args = args.iter().map(|a| substitute_expr(a, subs)).collect();
            Expr::Call {
                callee: Box::new(new_callee),
                args: new_args,
            }
        }
        Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(substitute_expr(left, subs)),
            op: op.clone(),
            right: Box::new(substitute_expr(right, subs)),
        },
        Expr::UnaryOp { op, operand } => Expr::UnaryOp {
            op: op.clone(),
            operand: Box::new(substitute_expr(operand, subs)),
        },
        Expr::Let { name, value } => {
            let new_value = substitute_expr(value, subs);
            let mut inner_subs = subs.clone();
            inner_subs.remove(name);
            Expr::Let {
                name: name.clone(),
                value: Box::new(new_value),
            }
        }
        Expr::Block { statements, tail } => {
            let mut new_stmts = Vec::new();
            let mut curr_subs = subs.clone();
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    let new_val = substitute_expr(value, &curr_subs);
                    curr_subs.remove(name);
                    new_stmts.push(Expr::Let {
                        name: name.clone(),
                        value: Box::new(new_val),
                    });
                } else {
                    new_stmts.push(substitute_expr(stmt, &curr_subs));
                }
            }
            let new_tail = tail.as_ref().map(|t| Box::new(substitute_expr(t, &curr_subs)));
            Expr::Block {
                statements: new_stmts,
                tail: new_tail,
            }
        }
        Expr::Match { target, arms } => {
            let new_target = substitute_expr(target, subs);
            let new_arms = arms
                .iter()
                .map(|arm| {
                    let mut arm_subs = subs.clone();
                    if let Pattern::Ident(ref id) = arm.pattern {
                        arm_subs.remove(id);
                    }
                    axiom_ast::MatchArm {
                        pattern: arm.pattern.clone(),
                        body: substitute_expr(&arm.body, &arm_subs),
                    }
                })
                .collect();
            Expr::Match {
                target: Box::new(new_target),
                arms: new_arms,
            }
        }
        _ => expr.clone(),
    }
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
        Expr::Ident(name) => Ok(name.replace('.', "_")),
        Expr::FieldAccess { .. } => {
            if let Some(path) = expr_to_field_path(expr) {
                Ok(path.replace('.', "_"))
            } else {
                Err("unresolvable field access in SMT translation".into())
            }
        }
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
                BinaryOperator::Divide => {
                    if l.contains('.') || r.contains('.') {
                        "/"
                    } else {
                        "div"
                    }
                }
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
            if arms.len() == 2 {
                let cond_str = expr_to_smt(target)?;
                let mut true_body = None;
                let mut false_body = None;
                for arm in arms {
                    match &arm.pattern {
                        Pattern::Literal(Literal::Bool(b)) => {
                            let arm_smt = expr_to_smt(&arm.body)?;
                            if *b {
                                true_body = Some(arm_smt);
                            } else {
                                false_body = Some(arm_smt);
                            }
                        }
                        Pattern::Wildcard | Pattern::Ident(_) => {
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
