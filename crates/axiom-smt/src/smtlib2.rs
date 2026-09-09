//! SMT-LIB2 generator for Axiom functions and contracts.

use std::collections::HashMap;

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeDef, UnaryOperator,
};

/// Minimum boundary of signed 64-bit physical machine word integer.
pub const I64_MIN: i64 = -9223372036854775808;
/// Maximum boundary of signed 64-bit physical machine word integer.
pub const I64_MAX: i64 = 9223372036854775807;

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
                    fields
                        .iter()
                        .map(|f| (f.name.clone(), f.ty.clone()))
                        .collect(),
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

/// Translates an Axiom function and returns query script with obligations count.
pub fn generate_smtlib2_and_obligations(func: &FunctionDecl) -> Result<(String, usize), String> {
    generate_smtlib2_with_module_and_obligations(func, None)
}

/// Translates an Axiom function with full module context into SMT-LIB2 format.
pub fn generate_smtlib2_with_module(
    func: &FunctionDecl,
    module: Option<&Module>,
) -> Result<String, String> {
    generate_smtlib2_with_module_and_obligations(func, module).map(|(query, _)| query)
}

/// Translates an Axiom function with full module context into SMT-LIB2 format,
/// and returns the total count of proof obligations checked (ensures + safety).
pub fn generate_smtlib2_with_module_and_obligations(
    func: &FunctionDecl,
    module: Option<&Module>,
) -> Result<(String, usize), String> {
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
    flatten_type_to_smt(
        "result",
        &func.return_type,
        &ctx.records,
        &mut flat_results,
        0,
    )?;
    for (name, smt_ty) in &flat_results {
        smt.push_str(&format!("(declare-const {} {})\n", name, smt_ty));
    }
    smt.push('\n');

    // 2.5 Physical representation bounds: 64-bit signed integer range for inputs
    smt.push_str("; Physical representation bounds: 64-bit signed integer range for inputs\n");
    for (name, smt_ty) in &flat_params {
        if *smt_ty == "Int" {
            smt.push_str(&format!(
                "(assert (and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807)))\n",
                name, name
            ));
        }
    }
    smt.push('\n');

    let mut var_types: HashMap<String, &'static str> = HashMap::new();
    for (name, smt_ty) in &flat_params {
        var_types.insert(name.clone(), *smt_ty);
    }
    for (name, smt_ty) in &flat_results {
        var_types.insert(name.clone(), *smt_ty);
    }

    // 3. Assert preconditions (Requires)
    if !func.requires.is_empty() {
        smt.push_str("; Preconditions (Requires)\n");
        for req in &func.requires {
            let inlined_req = inline_calls(req, &ctx, 0)?;
            let req_expr = expr_to_smt(&inlined_req, &var_types)?;
            smt.push_str(&format!("(assert {})\n", req_expr));
        }
        smt.push('\n');
    }

    // 4. Bind function body to result
    smt.push_str("; Function Body Definition\n");
    let inlined_body = inline_calls(&func.body, &ctx, 0)?;
    if flat_results.len() == 1 && flat_results[0].0 == "result" {
        let body_expr = expr_to_smt(&inlined_body, &var_types)?;
        smt.push_str(&format!("(assert (= result {}))\n\n", body_expr));
    } else {
        // Returned a record: bind individual fields if body is a RecordInit or let block
        bind_record_body_to_results(&inlined_body, &var_types, &mut smt)?;
    }

    // 5. Assert negation of postconditions (Ensures) and 64-bit safety range obligations
    smt.push_str(
        "; Goal: Find any counterexample violating Ensures clauses or 64-bit range safety\n",
    );
    let mut obligations = Vec::new();
    for ens in &func.ensures {
        let inlined_ens = inline_calls(ens, &ctx, 0)?;
        let ens_expr = expr_to_smt(&inlined_ens, &var_types)?;
        obligations.push(ens_expr);
    }
    // Safety range obligation: result Ints must not overflow 64-bit signed integer limits
    for (name, smt_ty) in &flat_results {
        if *smt_ty == "Int" {
            obligations.push(format!(
                "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                name, name
            ));
        }
    }

    // Collect arithmetic safety obligations from intermediate subexpressions in the inlined body
    let mut subexpr_obligations = Vec::new();
    let mut safety_subs = HashMap::new();
    collect_safety_obligations_recursive(
        &inlined_body,
        &var_types,
        &mut safety_subs,
        &[],
        &mut subexpr_obligations,
    )?;
    for safe_ob in subexpr_obligations {
        if !obligations.contains(&safe_ob) {
            obligations.push(safe_ob);
        }
    }

    let obligations_count = obligations.len();

    if obligations.len() == 1 {
        smt.push_str(&format!("(assert (not {}))\n\n", obligations[0]));
    } else {
        let mut all_obs = String::from("(and");
        for ob in &obligations {
            all_obs.push(' ');
            all_obs.push_str(ob);
        }
        all_obs.push(')');
        smt.push_str(&format!("(assert (not {}))\n\n", all_obs));
    }

    // 6. Check satisfiability and retrieve model on counterexample
    smt.push_str("(check-sat)\n(get-model)\n");

    Ok((smt, obligations_count))
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
        _ => Err(format!(
            "unsupported parameter type `{:?}` in SMT export",
            ty
        )),
    }
}

fn bind_record_body_to_results(
    body: &Expr,
    var_types: &HashMap<String, &'static str>,
    smt: &mut String,
) -> Result<(), String> {
    match body {
        Expr::RecordInit { fields, .. } => {
            for (f_name, f_expr) in fields {
                let val_smt = expr_to_smt(f_expr, var_types)?;
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
                    let val_smt = expr_to_smt(value, var_types)?;
                    let_bindings.push((name.clone(), val_smt));
                }
            }

            if let Some(tail_expr) = tail {
                if let Expr::RecordInit { fields, .. } = tail_expr.as_ref() {
                    for (f_name, f_expr) in fields {
                        let inner_val = expr_to_smt(f_expr, var_types)?;
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
                        return Err(format!(
                            "cannot verify function calling effectful `{fn_name}`"
                        ));
                    }
                    let inlined_args: Vec<Expr> = args
                        .iter()
                        .map(|a| inline_calls(a, ctx, depth + 1))
                        .collect::<Result<_, _>>()?;

                    let mut subs = HashMap::new();
                    for (param, arg) in target_func.params.iter().zip(inlined_args) {
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
            let new_tail = tail
                .as_ref()
                .map(|t| Box::new(substitute_expr(t, &curr_subs)));
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

/// 生成符合 Rust 向零截断（Truncation Towards Zero）语义的整数除法 SMT 表达式
pub fn emit_trunc_div_smt(l: &str, r: &str) -> String {
    format!(
        "(ite (>= {l} 0) (ite (>= {r} 0) (div {l} {r}) (- (div {l} (- {r})))) (ite (>= {r} 0) (- (div (- {l}) {r})) (div (- {l}) (- {r}))))",
        l = l,
        r = r
    )
}

/// 生成符合 Rust 规范（余数符号与被除数一致）的整数取模 SMT 表达式
pub fn emit_trunc_mod_smt(l: &str, r: &str) -> String {
    format!(
        "(ite (>= {l} 0) (ite (>= {r} 0) (mod {l} {r}) (mod {l} (- {r}))) (ite (>= {r} 0) (- (mod (- {l}) {r})) (- (mod (- {l}) (- {r})))))",
        l = l,
        r = r
    )
}

fn expr_to_smt(expr: &Expr, var_types: &HashMap<String, &'static str>) -> Result<String, String> {
    match expr {
        Expr::Literal(Literal::Int(n)) => {
            if *n == I64_MIN {
                Ok("(- 9223372036854775808)".into())
            } else if *n < 0 {
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
            let inner = expr_to_smt(operand, var_types)?;
            match op {
                UnaryOperator::Negate => Ok(format!("(- {})", inner)),
                UnaryOperator::Not => Ok(format!("(not {})", inner)),
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = expr_to_smt(left, var_types)?;
            let r = expr_to_smt(right, var_types)?;
            match op {
                BinaryOperator::Divide => {
                    let empty_subs = HashMap::new();
                    let is_real = infer_expr_type(left, var_types, &empty_subs) == "Real"
                        || infer_expr_type(right, var_types, &empty_subs) == "Real";
                    if is_real {
                        Ok(format!("(/ {} {})", l, r))
                    } else {
                        Ok(emit_trunc_div_smt(&l, &r))
                    }
                }
                BinaryOperator::Modulo => Ok(emit_trunc_mod_smt(&l, &r)),
                _ => {
                    let smt_op = match op {
                        BinaryOperator::Add => "+",
                        BinaryOperator::Subtract => "-",
                        BinaryOperator::Multiply => "*",
                        BinaryOperator::Equal | BinaryOperator::Is => "=",
                        BinaryOperator::NotEqual => "distinct",
                        BinaryOperator::LessThan => "<",
                        BinaryOperator::LessThanOrEqual => "<=",
                        BinaryOperator::GreaterThan => ">",
                        BinaryOperator::GreaterThanOrEqual => ">=",
                        BinaryOperator::And => "and",
                        BinaryOperator::Or => "or",
                        BinaryOperator::Implies => "=>",
                        BinaryOperator::Divide | BinaryOperator::Modulo => unreachable!(),
                    };
                    Ok(format!("({} {} {})", smt_op, l, r))
                }
            }
        }
        Expr::Let { name, value } => {
            let val = expr_to_smt(value, var_types)?;
            Ok(format!("(let (({} {})))", name, val))
        }
        Expr::Block { statements, tail } => {
            if statements.is_empty() {
                if let Some(t) = tail {
                    expr_to_smt(t, var_types)
                } else {
                    Ok("0".into())
                }
            } else {
                build_block_smt(statements, tail.as_deref(), var_types)
            }
        }
        Expr::Call { callee, args } => {
            let callee_str = expr_to_smt(callee, var_types)?;
            let mut s = format!("({}", callee_str);
            for arg in args {
                s.push(' ');
                s.push_str(&expr_to_smt(arg, var_types)?);
            }
            s.push(')');
            Ok(s)
        }
        Expr::Match { target, arms } => {
            if arms.is_empty() {
                return Err("empty match expression in SMT translation".into());
            }

            // 1. 按照物理执行顺序收集可达臂，遇到首个 catch-all (Wildcard 或 Ident) 立即截断后续不可达死分支
            let mut reachable_arms = Vec::new();
            for arm in arms {
                reachable_arms.push(arm);
                if matches!(arm.pattern, Pattern::Wildcard | Pattern::Ident(_)) {
                    break;
                }
            }

            let cond_str = expr_to_smt(target, var_types)?;

            // 2. 若首臂即为不可反驳 catch-all，直接翻译该臂 body（若是 Ident 则建立词法绑定）
            if reachable_arms.len() == 1 {
                if let Pattern::Ident(ref id) = reachable_arms[0].pattern {
                    let mut arm_var_types = var_types.clone();
                    let empty_subs = HashMap::new();
                    let target_ty = infer_expr_type(target, var_types, &empty_subs);
                    arm_var_types.insert(id.clone(), target_ty);
                    let body_smt = expr_to_smt(&reachable_arms[0].body, &arm_var_types)?;
                    return Ok(format!("(let (({} {})) {})", id, cond_str, body_smt));
                } else if matches!(reachable_arms[0].pattern, Pattern::Wildcard) {
                    return expr_to_smt(&reachable_arms[0].body, var_types);
                }
            }

            // 3. 情形 A：最后一条可达臂为 catch-all (通配符或标识符)
            let last_idx = reachable_arms.len() - 1;
            if matches!(
                reachable_arms[last_idx].pattern,
                Pattern::Wildcard | Pattern::Ident(_)
            ) {
                let last_arm = &reachable_arms[last_idx];
                let mut current_else = if let Pattern::Ident(ref id) = last_arm.pattern {
                    let mut arm_var_types = var_types.clone();
                    let empty_subs = HashMap::new();
                    let target_ty = infer_expr_type(target, var_types, &empty_subs);
                    arm_var_types.insert(id.clone(), target_ty);
                    let body_smt = expr_to_smt(&last_arm.body, &arm_var_types)?;
                    format!("(let (({} {})) {})", id, cond_str, body_smt)
                } else {
                    expr_to_smt(&last_arm.body, var_types)?
                };

                for arm in reachable_arms[..last_idx].iter().rev() {
                    let (pat_cond_opt, _) = pattern_condition_smt(&arm.pattern, &cond_str)?;
                    let pat_cond = pat_cond_opt.ok_or_else(|| {
                        "unexpected catch-all before last reachable arm".to_string()
                    })?;
                    let body_smt = expr_to_smt(&arm.body, var_types)?;
                    current_else = format!("(ite {} {} {})", pat_cond, body_smt, current_else);
                }
                return Ok(current_else);
            }

            // 4. 情形 B：布尔完备匹配 (true 与 false 双臂)
            if reachable_arms.len() == 2 {
                match (&reachable_arms[0].pattern, &reachable_arms[1].pattern) {
                    (
                        Pattern::Literal(Literal::Bool(true)),
                        Pattern::Literal(Literal::Bool(false)),
                    ) => {
                        let true_body = expr_to_smt(&reachable_arms[0].body, var_types)?;
                        let false_body = expr_to_smt(&reachable_arms[1].body, var_types)?;
                        return Ok(format!("(ite {} {} {})", cond_str, true_body, false_body));
                    }
                    (
                        Pattern::Literal(Literal::Bool(false)),
                        Pattern::Literal(Literal::Bool(true)),
                    ) => {
                        let false_body = expr_to_smt(&reachable_arms[0].body, var_types)?;
                        let true_body = expr_to_smt(&reachable_arms[1].body, var_types)?;
                        return Ok(format!("(ite {} {} {})", cond_str, true_body, false_body));
                    }
                    _ => {}
                }
            }

            Err("complex match pattern translation requires full ADT solver encoding or catch-all default".into())
        }
        _ => Err("unsupported expression in SMT translation".into()),
    }
}

/// 提取统一的模式匹配 SMT 条件生成器 (match_cond, neg_cond)
pub fn pattern_condition_smt(
    pattern: &Pattern,
    target_smt: &str,
) -> Result<(Option<String>, Option<String>), String> {
    match pattern {
        Pattern::Literal(Literal::Bool(true)) => Ok((
            Some(target_smt.to_string()),
            Some(format!("(not {})", target_smt)),
        )),
        Pattern::Literal(Literal::Bool(false)) => Ok((
            Some(format!("(not {})", target_smt)),
            Some(target_smt.to_string()),
        )),
        Pattern::Literal(Literal::Int(n)) => {
            let n_smt = if *n == I64_MIN {
                "(- 9223372036854775808)".to_string()
            } else if *n < 0 {
                format!("(- {})", n.abs())
            } else {
                n.to_string()
            };
            Ok((
                Some(format!("(= {} {})", target_smt, n_smt)),
                Some(format!("(not (= {} {}))", target_smt, n_smt)),
            ))
        }
        Pattern::Literal(Literal::Float(f)) => {
            if f.is_finite() {
                let f_smt = if *f < 0.0 {
                    format!("(- {})", format_float(f.abs()))
                } else {
                    format_float(*f)
                };
                Ok((
                    Some(format!("(= {} {})", target_smt, f_smt)),
                    Some(format!("(not (= {} {}))", target_smt, f_smt)),
                ))
            } else {
                Err("unsupported non-finite float literal pattern".into())
            }
        }
        Pattern::Wildcard | Pattern::Ident(_) => Ok((None, None)),
        _ => Err("complex match pattern translation requires full ADT solver encoding".into()),
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

fn build_block_smt(
    statements: &[Expr],
    tail: Option<&Expr>,
    var_types: &HashMap<String, &'static str>,
) -> Result<String, String> {
    if statements.is_empty() {
        if let Some(t) = tail {
            return expr_to_smt(t, var_types);
        }
        return Ok("0".into());
    }

    let first = &statements[0];
    let rest = &statements[1..];

    if let Expr::Let { name, value } = first {
        let val_smt = expr_to_smt(value, var_types)?;
        let empty_subs = HashMap::new();
        let val_ty = infer_expr_type(value, var_types, &empty_subs);
        let mut child_var_types = var_types.clone();
        child_var_types.insert(name.clone(), val_ty);
        let rest_smt = build_block_smt(rest, tail, &child_var_types)?;
        Ok(format!("(let (({} {})) {})", name, val_smt, rest_smt))
    } else {
        build_block_smt(rest, tail, var_types)
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

/// Translates ONLY the function's parameters and `requires` preconditions into an SMT-LIB2 query
/// to verify that the preconditions are mathematically satisfiable (preventing vacuous proofs).
pub fn generate_precondition_check_smtlib2(
    func: &FunctionDecl,
    module: Option<&Module>,
) -> Result<String, String> {
    if func.requires.is_empty() {
        return Ok(String::new());
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
    smt.push_str("; Axiom Precondition Satisfiability (Vacuity) Query for: ");
    smt.push_str(&func.name);
    smt.push_str("\n(reset)\n(set-logic ALL)\n\n");

    let mut flat_params = Vec::new();
    for p in &func.params {
        flatten_type_to_smt(&p.name, &p.ty, &ctx.records, &mut flat_params, 0)?;
    }
    for (name, smt_ty) in &flat_params {
        smt.push_str(&format!("(declare-const {} {})\n", name, smt_ty));
    }
    smt.push('\n');

    for (name, smt_ty) in &flat_params {
        if *smt_ty == "Int" {
            smt.push_str(&format!(
                "(assert (and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807)))\n",
                name, name
            ));
        }
    }
    smt.push('\n');

    let mut var_types: HashMap<String, &'static str> = HashMap::new();
    for (name, smt_ty) in &flat_params {
        var_types.insert(name.clone(), *smt_ty);
    }

    smt.push_str("; Assert all preconditions\n");
    for req in &func.requires {
        let inlined_req = inline_calls(req, &ctx, 0)?;
        let req_expr = expr_to_smt(&inlined_req, &var_types)?;
        smt.push_str(&format!("(assert {})\n", req_expr));
    }
    smt.push_str("\n(check-sat)\n");

    Ok(smt)
}

fn infer_expr_type(
    expr: &Expr,
    var_types: &HashMap<String, &'static str>,
    subs: &HashMap<String, Expr>,
) -> &'static str {
    match expr {
        Expr::Literal(Literal::Float(_)) => "Real",
        Expr::Literal(Literal::Int(_)) => "Int",
        Expr::Literal(Literal::Bool(_)) => "Bool",
        Expr::Literal(Literal::String(_)) => "String",
        Expr::Ident(name) => {
            if let Some(sub) = subs.get(name) {
                infer_expr_type(sub, var_types, subs)
            } else if let Some(ty) = var_types.get(name) {
                ty
            } else {
                "Int"
            }
        }
        Expr::FieldAccess { .. } => {
            if let Some(path) = expr_to_field_path(expr) {
                let key = path.replace('.', "_");
                if let Some(ty) = var_types.get(&key) {
                    return ty;
                }
            }
            "Int"
        }
        Expr::BinaryOp { left, op, right } => match op {
            BinaryOperator::Equal
            | BinaryOperator::Is
            | BinaryOperator::NotEqual
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
            | BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Implies => "Bool",
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulo => {
                let lt = infer_expr_type(left, var_types, subs);
                let rt = infer_expr_type(right, var_types, subs);
                if lt == "Real" || rt == "Real" {
                    "Real"
                } else {
                    "Int"
                }
            }
        },
        Expr::UnaryOp { op, operand } => match op {
            UnaryOperator::Not => "Bool",
            UnaryOperator::Negate => infer_expr_type(operand, var_types, subs),
        },
        Expr::Block { statements, tail } => {
            let mut local_var_types = var_types.clone();
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    let ty = infer_expr_type(value, &local_var_types, subs);
                    local_var_types.insert(name.clone(), ty);
                }
            }
            if let Some(t) = tail {
                infer_expr_type(t, &local_var_types, subs)
            } else {
                "Int"
            }
        }
        Expr::Match { arms, .. } => {
            if let Some(first_arm) = arms.first() {
                infer_expr_type(&first_arm.body, var_types, subs)
            } else {
                "Int"
            }
        }
        _ => "Int",
    }
}

fn emit_safety_obligation(
    basic_ob: String,
    path_conditions: &[String],
    obligations: &mut Vec<String>,
) {
    let final_ob = if path_conditions.is_empty() {
        basic_ob
    } else if path_conditions.len() == 1 {
        format!("(=> {} {})", path_conditions[0], basic_ob)
    } else {
        let mut cond_str = String::from("(and");
        for c in path_conditions {
            cond_str.push(' ');
            cond_str.push_str(c);
        }
        cond_str.push(')');
        format!("(=> {} {})", cond_str, basic_ob)
    };

    if !obligations.contains(&final_ob) {
        obligations.push(final_ob);
    }
}

fn substitute_expr_with_subs(expr: &Expr, subs: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Ident(name) => {
            if let Some(sub) = subs.get(name) {
                sub.clone()
            } else {
                Expr::Ident(name.clone())
            }
        }
        Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(substitute_expr_with_subs(left, subs)),
            op: op.clone(),
            right: Box::new(substitute_expr_with_subs(right, subs)),
        },
        Expr::UnaryOp { op, operand } => Expr::UnaryOp {
            op: op.clone(),
            operand: Box::new(substitute_expr_with_subs(operand, subs)),
        },
        Expr::Block { statements, tail } => {
            let mut curr_subs = subs.clone();
            let mut new_stmts = Vec::new();
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    let expanded = substitute_expr_with_subs(value, &curr_subs);
                    curr_subs.insert(name.clone(), expanded.clone());
                    new_stmts.push(Expr::Let {
                        name: name.clone(),
                        value: Box::new(expanded),
                    });
                } else {
                    new_stmts.push(substitute_expr_with_subs(stmt, &curr_subs));
                }
            }
            Expr::Block {
                statements: new_stmts,
                tail: tail
                    .as_ref()
                    .map(|t| Box::new(substitute_expr_with_subs(t, &curr_subs))),
            }
        }
        Expr::Let { name, value } => {
            let expanded = substitute_expr_with_subs(value, subs);
            Expr::Let {
                name: name.clone(),
                value: Box::new(expanded),
            }
        }
        Expr::Call { callee, args } => Expr::Call {
            callee: Box::new(substitute_expr_with_subs(callee, subs)),
            args: args
                .iter()
                .map(|a| substitute_expr_with_subs(a, subs))
                .collect(),
        },
        Expr::FieldAccess { target, field } => Expr::FieldAccess {
            target: Box::new(substitute_expr_with_subs(target, subs)),
            field: field.clone(),
        },
        Expr::RecordInit { ty_name, fields } => Expr::RecordInit {
            ty_name: ty_name.clone(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), substitute_expr_with_subs(v, subs)))
                .collect(),
        },
        Expr::RecordUpdate { base, updates } => Expr::RecordUpdate {
            base: Box::new(substitute_expr_with_subs(base, subs)),
            updates: updates
                .iter()
                .map(|(k, v)| (k.clone(), substitute_expr_with_subs(v, subs)))
                .collect(),
        },
        Expr::Match { target, arms } => Expr::Match {
            target: Box::new(substitute_expr_with_subs(target, subs)),
            arms: arms
                .iter()
                .map(|arm| {
                    let mut arm_subs = subs.clone();
                    if let Pattern::Ident(ref id) = arm.pattern {
                        arm_subs.remove(id);
                    }
                    axiom_ast::MatchArm {
                        pattern: arm.pattern.clone(),
                        body: substitute_expr_with_subs(&arm.body, &arm_subs),
                    }
                })
                .collect(),
        },
        _ => expr.clone(),
    }
}

fn collect_safety_obligations_recursive(
    expr: &Expr,
    var_types: &HashMap<String, &'static str>,
    subs: &mut HashMap<String, Expr>,
    path_conditions: &[String],
    obligations: &mut Vec<String>,
) -> Result<(), String> {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            collect_safety_obligations_recursive(
                left,
                var_types,
                subs,
                path_conditions,
                obligations,
            )?;
            collect_safety_obligations_recursive(
                right,
                var_types,
                subs,
                path_conditions,
                obligations,
            )?;

            let left_expanded = substitute_expr_with_subs(left, subs);
            let right_expanded = substitute_expr_with_subs(right, subs);

            let is_real = infer_expr_type(left, var_types, subs) == "Real"
                || infer_expr_type(right, var_types, subs) == "Real";

            if !is_real {
                let l_smt = expr_to_smt(&left_expanded, var_types)?;
                let r_smt = expr_to_smt(&right_expanded, var_types)?;

                match op {
                    BinaryOperator::Add => {
                        let op_smt = format!("(+ {} {})", l_smt, r_smt);
                        let ob = format!(
                            "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                            op_smt, op_smt
                        );
                        emit_safety_obligation(ob, path_conditions, obligations);
                    }
                    BinaryOperator::Subtract => {
                        let op_smt = format!("(- {} {})", l_smt, r_smt);
                        let ob = format!(
                            "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                            op_smt, op_smt
                        );
                        emit_safety_obligation(ob, path_conditions, obligations);
                    }
                    BinaryOperator::Multiply => {
                        let op_smt = format!("(* {} {})", l_smt, r_smt);
                        let ob = format!(
                            "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                            op_smt, op_smt
                        );
                        emit_safety_obligation(ob, path_conditions, obligations);
                    }
                    BinaryOperator::Divide => {
                        emit_safety_obligation(
                            format!("(not (= {} 0))", r_smt),
                            path_conditions,
                            obligations,
                        );
                        emit_safety_obligation(
                            format!(
                                "(not (and (= {} (- 9223372036854775808)) (= {} (- 1))))",
                                l_smt, r_smt
                            ),
                            path_conditions,
                            obligations,
                        );
                    }
                    BinaryOperator::Modulo => {
                        emit_safety_obligation(
                            format!("(not (= {} 0))", r_smt),
                            path_conditions,
                            obligations,
                        );
                        emit_safety_obligation(
                            format!(
                                "(not (and (= {} (- 9223372036854775808)) (= {} (- 1))))",
                                l_smt, r_smt
                            ),
                            path_conditions,
                            obligations,
                        );
                    }
                    _ => {}
                }
            }
        }
        Expr::UnaryOp { op, operand } => {
            collect_safety_obligations_recursive(
                operand,
                var_types,
                subs,
                path_conditions,
                obligations,
            )?;
            if *op == UnaryOperator::Negate {
                let is_real = infer_expr_type(operand, var_types, subs) == "Real";
                if !is_real {
                    let operand_expanded = substitute_expr_with_subs(operand, subs);
                    let inner_smt = expr_to_smt(&operand_expanded, var_types)?;
                    let ob = format!("(not (= {} (- 9223372036854775808)))", inner_smt);
                    emit_safety_obligation(ob, path_conditions, obligations);
                }
            }
        }
        Expr::Let { name, value } => {
            collect_safety_obligations_recursive(
                value,
                var_types,
                subs,
                path_conditions,
                obligations,
            )?;
            let expanded_val = substitute_expr_with_subs(value, subs);
            subs.insert(name.clone(), expanded_val);
        }
        Expr::Block { statements, tail } => {
            let mut curr_subs = subs.clone();
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    collect_safety_obligations_recursive(
                        value,
                        var_types,
                        &mut curr_subs,
                        path_conditions,
                        obligations,
                    )?;
                    let expanded_val = substitute_expr_with_subs(value, &curr_subs);
                    curr_subs.insert(name.clone(), expanded_val);
                } else {
                    collect_safety_obligations_recursive(
                        stmt,
                        var_types,
                        &mut curr_subs,
                        path_conditions,
                        obligations,
                    )?;
                }
            }
            if let Some(t) = tail {
                collect_safety_obligations_recursive(
                    t,
                    var_types,
                    &mut curr_subs,
                    path_conditions,
                    obligations,
                )?;
            }
        }
        Expr::Match { target, arms } => {
            collect_safety_obligations_recursive(
                target,
                var_types,
                subs,
                path_conditions,
                obligations,
            )?;
            let target_expanded = substitute_expr_with_subs(target, subs);
            let target_smt_res = expr_to_smt(&target_expanded, var_types);

            if let Ok(target_smt) = target_smt_res {
                let mut accumulated_negations: Vec<String> = path_conditions.to_vec();
                for arm in arms {
                    let mut arm_subs = subs.clone();
                    if let Pattern::Ident(ref id) = arm.pattern {
                        arm_subs.insert(id.clone(), target_expanded.clone());
                    }

                    // 调用统一的模式条件生成器计算当前臂模式的匹配条件与未匹配否定条件
                    let (match_cond_opt, neg_cond_opt) =
                        pattern_condition_smt(&arm.pattern, &target_smt)?;

                    // 当前臂内部生效路径条件 = 之前所有臂的未匹配条件 + 当前臂的匹配条件
                    let mut arm_paths = accumulated_negations.clone();
                    if let Some(mc) = match_cond_opt {
                        arm_paths.push(mc);
                    }

                    let is_catch_all = matches!(arm.pattern, Pattern::Wildcard | Pattern::Ident(_));

                    // 递归分析当前臂 body
                    collect_safety_obligations_recursive(
                        &arm.body,
                        var_types,
                        &mut arm_subs,
                        &arm_paths,
                        obligations,
                    )?;

                    // [Spec 1, P0] 遇到首个 catch-all 分支，后续所有分支在物理上均为不可达死代码，立即终止分析
                    if is_catch_all {
                        break;
                    }

                    // 将当前臂的否定条件累积到后续分支的路径前提中
                    if let Some(nc) = neg_cond_opt {
                        accumulated_negations.push(nc);
                    }
                }
            } else {
                for arm in arms {
                    let mut arm_subs = subs.clone();
                    if let Pattern::Ident(ref id) = arm.pattern {
                        arm_subs.remove(id);
                    }
                    let is_catch_all = matches!(arm.pattern, Pattern::Wildcard | Pattern::Ident(_));
                    collect_safety_obligations_recursive(
                        &arm.body,
                        var_types,
                        &mut arm_subs,
                        path_conditions,
                        obligations,
                    )?;
                    if is_catch_all {
                        break;
                    }
                }
            }
        }
        Expr::Call { callee, args } => {
            collect_safety_obligations_recursive(
                callee,
                var_types,
                subs,
                path_conditions,
                obligations,
            )?;
            for arg in args {
                collect_safety_obligations_recursive(
                    arg,
                    var_types,
                    subs,
                    path_conditions,
                    obligations,
                )?;
            }
        }
        Expr::FieldAccess { target, .. } => {
            collect_safety_obligations_recursive(
                target,
                var_types,
                subs,
                path_conditions,
                obligations,
            )?;
        }
        Expr::RecordInit { fields, .. } => {
            for (_, val) in fields {
                collect_safety_obligations_recursive(
                    val,
                    var_types,
                    subs,
                    path_conditions,
                    obligations,
                )?;
            }
        }
        Expr::RecordUpdate { base, updates } => {
            collect_safety_obligations_recursive(
                base,
                var_types,
                subs,
                path_conditions,
                obligations,
            )?;
            for (_, val) in updates {
                collect_safety_obligations_recursive(
                    val,
                    var_types,
                    subs,
                    path_conditions,
                    obligations,
                )?;
            }
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) | Expr::Raw(_) => {}
    }
    Ok(())
}
