//! SMT-LIB2 generator for Axiom functions and contracts.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use axiom_ast::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeDef, UnaryOperator,
};

/// Minimum boundary of signed 64-bit physical machine word integer.
pub const I64_MIN: i64 = -9223372036854775808;
/// Maximum boundary of signed 64-bit physical machine word integer.
pub const I64_MAX: i64 = 9223372036854775807;
/// Minimum boundary of signed 128-bit integer for intermediate arithmetic.
pub const I128_MIN: i128 = -170141183460469231731687303715884105728;
/// Maximum boundary of signed 128-bit integer for intermediate arithmetic.
pub const I128_MAX: i128 = 170141183460469231731687303715884105727;

/// Context containing module records and function definitions.
#[derive(Debug, Clone, Default)]
pub struct SmtContext<'a> {
    pub records: HashMap<String, Vec<(String, Type)>>,
    pub functions: HashMap<String, &'a FunctionDecl>,
    pub current_func: Option<&'a FunctionDecl>,
    pub local_decimals: HashMap<String, bool>,
    pub type_env: Option<axiom_ast::TypeEnv<'a>>,
    pub reserved_names: RefCell<HashSet<String>>,
    pub call_id_counter: Cell<usize>,
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
        let reserved_names = collect_module_reserved_names(module);
        Self {
            records,
            functions,
            current_func: None,
            local_decimals: HashMap::new(),
            type_env: Some(axiom_ast::TypeEnv::new(module, None)),
            reserved_names: RefCell::new(reserved_names),
            call_id_counter: Cell::new(1),
        }
    }

    pub fn next_call_id(&self) -> usize {
        let id = self.call_id_counter.get();
        self.call_id_counter.set(id + 1);
        id
    }

    pub fn alloc_fresh_id(&self, base_name: &str, call_id: usize, seq: usize) -> String {
        let mut reserved = self.reserved_names.borrow_mut();
        let mut candidate = format!("{base_name}__axiom_in_{call_id}_{seq}");
        let mut attempt = 0usize;
        while reserved.contains(&candidate) {
            attempt += 1;
            candidate = format!("{base_name}__axiom_in_{call_id}_{seq}_c{attempt}");
        }
        reserved.insert(candidate.clone());
        candidate
    }

    pub fn infer_type(&self, expr: &Expr) -> Option<Type> {
        if let Some(ref env) = self.type_env {
            return env.infer_type(expr);
        }
        if self.is_decimal(expr) {
            return Some(Type::Decimal);
        }
        None
    }

    pub fn is_decimal(&self, expr: &Expr) -> bool {
        if let Some(ref env) = self.type_env {
            return env.is_decimal(expr);
        }
        match expr {
            Expr::Literal(Literal::Decimal(_)) => true,
            Expr::Ident(name) => {
                if let Some(&is_dec) = self.local_decimals.get(name) {
                    if is_dec {
                        return true;
                    }
                }
                if name == "result" {
                    if let Some(f) = self.current_func {
                        return f.return_type == Type::Decimal;
                    }
                }
                if let Some(f) = self.current_func {
                    for p in &f.params {
                        if &p.name == name && p.ty == Type::Decimal {
                            return true;
                        }
                    }
                }
                false
            }
            Expr::FieldAccess { field, .. } => {
                for fields in self.records.values() {
                    for (f_name, f_ty) in fields {
                        if f_name == field && *f_ty == Type::Decimal {
                            return true;
                        }
                    }
                }
                false
            }
            Expr::BinaryOp {
                left,
                op:
                    BinaryOperator::Add
                    | BinaryOperator::Subtract
                    | BinaryOperator::Multiply
                    | BinaryOperator::Divide
                    | BinaryOperator::Modulo,
                right,
            } => self.is_decimal(left) || self.is_decimal(right),
            Expr::UnaryOp {
                op: UnaryOperator::Negate,
                operand,
            } => self.is_decimal(operand),
            Expr::Call { callee, .. } => {
                if let Expr::Ident(ref name) = **callee {
                    if let Some(target) = self.functions.get(name) {
                        return target.return_type == Type::Decimal;
                    }
                }
                false
            }
            Expr::Block { tail: Some(t), .. } => self.is_decimal(t),
            Expr::Match { arms, .. } => {
                if let Some(first) = arms.first() {
                    self.is_decimal(&first.body)
                } else {
                    false
                }
            }
            _ => false,
        }
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
    if func.contains_raw() {
        return Err(format!(
            "function `{}` contains Expr::Raw which cannot be verified with SMT",
            func.name
        ));
    }

    let dummy_mod = Module {
        types: vec![],
        functions: vec![func.clone()],
        effects: vec![],
    };
    let effective_module = module.unwrap_or(&dummy_mod);
    let mut ctx = SmtContext::from_module(effective_module);
    ctx.current_func = Some(func);
    ctx.type_env = Some(axiom_ast::TypeEnv::new(effective_module, Some(func)));

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
            let req_expr = expr_to_smt(&inlined_req, &var_types, &ctx)?;
            smt.push_str(&format!("(assert {})\n", req_expr));
        }
        smt.push('\n');
    }

    // 4. Bind function body to result
    smt.push_str("; Function Body Definition\n");
    let inlined_body = inline_calls(&func.body, &ctx, 0)?;
    if flat_results.len() == 1 && flat_results[0].0 == "result" {
        let body_expr = expr_to_smt(&inlined_body, &var_types, &ctx)?;
        smt.push_str(&format!("(assert (= result {}))\n\n", body_expr));
    } else {
        // Returned a record: bind individual fields if body is a RecordInit or let block
        bind_record_body_to_results(&inlined_body, &var_types, &mut smt, &ctx)?;
    }

    // 5. Assert negation of postconditions (Ensures) and safety/contract obligations
    smt.push_str(
        "; Goal: Find any counterexample violating Ensures clauses, Call-Site Preconditions, or 64-bit range safety\n",
    );
    let mut obligations = Vec::new();
    for ens in &func.ensures {
        let inlined_ens = inline_calls(ens, &ctx, 0)?;
        let ens_expr = expr_to_smt(&inlined_ens, &var_types, &ctx)?;
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

    // Collect arithmetic safety and callee contract obligations across body, requires, and ensures clauses
    let mut subexpr_obligations = Vec::new();
    let mut safety_subs = HashMap::new();
    collect_safety_obligations_recursive(
        &func.body,
        &var_types,
        &mut safety_subs,
        &[],
        &mut subexpr_obligations,
        &ctx,
        0,
    )?;
    for req in &func.requires {
        let mut req_subs = HashMap::new();
        collect_safety_obligations_recursive(
            req,
            &var_types,
            &mut req_subs,
            &[],
            &mut subexpr_obligations,
            &ctx,
            0,
        )?;
    }
    for ens in &func.ensures {
        let mut ens_subs = HashMap::new();
        collect_safety_obligations_recursive(
            ens,
            &var_types,
            &mut ens_subs,
            &[],
            &mut subexpr_obligations,
            &ctx,
            0,
        )?;
    }
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
        Type::Int | Type::Decimal => {
            out.push((prefix.replace('.', "_"), "Int"));
            Ok(())
        }
        Type::Float => {
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
    ctx: &SmtContext,
) -> Result<(), String> {
    let mut let_bindings = Vec::new();
    let mut current = body;
    let mut child_ctx = ctx.clone();

    while let Expr::Block { statements, tail } = current {
        if let Some(ref mut env) = child_ctx.type_env {
            env.push_scope();
        }
        for s in statements {
            if let Expr::Let { name, value } = s {
                let val_smt = expr_to_smt(value, var_types, &child_ctx)?;
                let is_dec = child_ctx.is_decimal(value);
                child_ctx.local_decimals.insert(name.clone(), is_dec);
                if let Some(ref mut env) = child_ctx.type_env {
                    if let Some(v_ty) = env.infer_type(value) {
                        env.insert(name.clone(), v_ty);
                    }
                }
                let_bindings.push((name.clone(), val_smt));
            }
        }
        if let Some(tail_expr) = tail {
            current = tail_expr.as_ref();
        } else {
            return Err("record block body missing tail expression".into());
        }
    }

    if let Expr::RecordInit { fields, .. } = current {
        for (f_name, f_expr) in fields {
            let inner_val = expr_to_smt(f_expr, var_types, &child_ctx)?;
            let mut full_val = inner_val;
            for (let_name, let_val) in let_bindings.iter().rev() {
                full_val = format!("(let (({} {})) {})", let_name, let_val, full_val);
            }
            smt.push_str(&format!("(assert (= result_{} {}))\n", f_name, full_val));
        }
        smt.push('\n');
        return Ok(());
    }

    Err("record function body must evaluate to a record".into())
}

fn flatten_block(statements: Vec<Expr>, tail: Option<Box<Expr>>) -> Expr {
    let mut flat_stmts = statements;
    match tail {
        Some(tail_box) => match *tail_box {
            Expr::Block {
                statements: inner_stmts,
                tail: inner_tail,
            } => {
                flat_stmts.extend(inner_stmts);
                if flat_stmts.is_empty() {
                    if let Some(it) = inner_tail {
                        *it
                    } else {
                        Expr::Block {
                            statements: flat_stmts,
                            tail: None,
                        }
                    }
                } else {
                    Expr::Block {
                        statements: flat_stmts,
                        tail: inner_tail,
                    }
                }
            }
            other => {
                if flat_stmts.is_empty() {
                    other
                } else {
                    Expr::Block {
                        statements: flat_stmts,
                        tail: Some(Box::new(other)),
                    }
                }
            }
        },
        None => Expr::Block {
            statements: flat_stmts,
            tail: None,
        },
    }
}

/// Collects all identifiers (types, functions, fields, parameters, let variables, patterns)
/// across the entire module to construct a reserved name set for hygienic symbol generation.
pub fn collect_module_reserved_names(module: &Module) -> HashSet<String> {
    let mut names = HashSet::new();
    let keywords = [
        "result",
        "true",
        "false",
        "Int",
        "Real",
        "Bool",
        "String",
        "Unit",
        "Decimal",
        "let",
        "assert",
        "check-sat",
        "get-model",
        "declare-const",
        "define-fun",
        "and",
        "or",
        "not",
        "ite",
        "mod",
        "div",
        "to_real",
        "to_int",
    ];
    for kw in &keywords {
        names.insert((*kw).to_string());
    }

    for ty in &module.types {
        match ty {
            TypeDef::Record { name, fields } => {
                names.insert(name.clone());
                for f in fields {
                    names.insert(f.name.clone());
                }
            }
            TypeDef::Enum { name, variants } => {
                names.insert(name.clone());
                for v in variants {
                    names.insert(v.name.clone());
                    for f in &v.fields {
                        names.insert(f.name.clone());
                    }
                }
            }
        }
    }

    for eff in &module.effects {
        names.insert(eff.name.clone());
        for op in &eff.operations {
            names.insert(op.name.clone());
        }
    }

    for func in &module.functions {
        collect_function_reserved_names(func, &mut names);
    }

    names
}

pub fn collect_function_reserved_names(func: &FunctionDecl, names: &mut HashSet<String>) {
    names.insert(func.name.clone());
    for p in &func.params {
        names.insert(p.name.clone());
    }
    for req in &func.requires {
        collect_expr_identifiers(req, names);
    }
    for ens in &func.ensures {
        collect_expr_identifiers(ens, names);
    }
    collect_expr_identifiers(&func.body, names);
}

fn collect_expr_identifiers(expr: &Expr, names: &mut HashSet<String>) {
    match expr {
        Expr::Ident(name) | Expr::Hole(name) => {
            names.insert(name.clone());
        }
        Expr::Let { name, value } => {
            names.insert(name.clone());
            collect_expr_identifiers(value, names);
        }
        Expr::Block { statements, tail } => {
            for s in statements {
                collect_expr_identifiers(s, names);
            }
            if let Some(t) = tail {
                collect_expr_identifiers(t, names);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_expr_identifiers(left, names);
            collect_expr_identifiers(right, names);
        }
        Expr::UnaryOp { operand, .. } => {
            collect_expr_identifiers(operand, names);
        }
        Expr::Call { callee, args } => {
            collect_expr_identifiers(callee, names);
            for a in args {
                collect_expr_identifiers(a, names);
            }
        }
        Expr::FieldAccess { target, field } => {
            collect_expr_identifiers(target, names);
            names.insert(field.clone());
        }
        Expr::RecordInit { fields, .. } => {
            for (f, v) in fields {
                names.insert(f.clone());
                collect_expr_identifiers(v, names);
            }
        }
        Expr::RecordUpdate { base, updates } => {
            collect_expr_identifiers(base, names);
            for (f, v) in updates {
                names.insert(f.clone());
                collect_expr_identifiers(v, names);
            }
        }
        Expr::Match { target, arms } => {
            collect_expr_identifiers(target, names);
            for arm in arms {
                collect_pattern_identifiers(&arm.pattern, names);
                collect_expr_identifiers(&arm.body, names);
            }
        }
        _ => {}
    }
}

fn collect_pattern_identifiers(pattern: &Pattern, names: &mut HashSet<String>) {
    match pattern {
        Pattern::Ident(name) => {
            names.insert(name.clone());
        }
        Pattern::Variant { fields, .. } => {
            for f in fields {
                collect_pattern_identifiers(f, names);
            }
        }
        _ => {}
    }
}

fn alpha_rename_pattern(
    pattern: &Pattern,
    call_id: usize,
    seq: &mut usize,
    renames: &mut HashMap<String, String>,
    ctx: &SmtContext,
) -> Pattern {
    match pattern {
        Pattern::Wildcard => Pattern::Wildcard,
        Pattern::Literal(lit) => Pattern::Literal(lit.clone()),
        Pattern::Ident(name) => {
            *seq += 1;
            let fresh_name = ctx.alloc_fresh_id(name, call_id, *seq);
            renames.insert(name.clone(), fresh_name.clone());
            Pattern::Ident(fresh_name)
        }
        Pattern::Variant {
            enum_name,
            variant_name,
            fields,
        } => {
            let new_fields = fields
                .iter()
                .map(|f| alpha_rename_pattern(f, call_id, seq, renames, ctx))
                .collect();
            Pattern::Variant {
                enum_name: enum_name.clone(),
                variant_name: variant_name.clone(),
                fields: new_fields,
            }
        }
    }
}

fn alpha_rename_locals(
    expr: &Expr,
    call_id: usize,
    seq: &mut usize,
    renames: &HashMap<String, String>,
    ctx: &SmtContext,
) -> Expr {
    match expr {
        Expr::Ident(name) => {
            if let Some(new_name) = renames.get(name) {
                Expr::Ident(new_name.clone())
            } else {
                Expr::Ident(name.clone())
            }
        }
        Expr::Let { name, value } => {
            let new_value = alpha_rename_locals(value, call_id, seq, renames, ctx);
            *seq += 1;
            let fresh_name = ctx.alloc_fresh_id(name, call_id, *seq);
            Expr::Let {
                name: fresh_name,
                value: Box::new(new_value),
            }
        }
        Expr::Block { statements, tail } => {
            let mut curr_renames = renames.clone();
            let mut new_stmts = Vec::with_capacity(statements.len());
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    let new_val = alpha_rename_locals(value, call_id, seq, &curr_renames, ctx);
                    *seq += 1;
                    let fresh_name = ctx.alloc_fresh_id(name, call_id, *seq);
                    curr_renames.insert(name.clone(), fresh_name.clone());
                    new_stmts.push(Expr::Let {
                        name: fresh_name,
                        value: Box::new(new_val),
                    });
                } else {
                    new_stmts.push(alpha_rename_locals(stmt, call_id, seq, &curr_renames, ctx));
                }
            }
            let new_tail = tail
                .as_ref()
                .map(|t| Box::new(alpha_rename_locals(t, call_id, seq, &curr_renames, ctx)));
            Expr::Block {
                statements: new_stmts,
                tail: new_tail,
            }
        }
        Expr::Match { target, arms } => {
            let new_target = alpha_rename_locals(target, call_id, seq, renames, ctx);
            let new_arms = arms
                .iter()
                .map(|arm| {
                    let mut arm_renames = renames.clone();
                    let new_pattern =
                        alpha_rename_pattern(&arm.pattern, call_id, seq, &mut arm_renames, ctx);
                    let new_body = alpha_rename_locals(&arm.body, call_id, seq, &arm_renames, ctx);
                    axiom_ast::MatchArm {
                        pattern: new_pattern,
                        body: new_body,
                    }
                })
                .collect();
            Expr::Match {
                target: Box::new(new_target),
                arms: new_arms,
            }
        }
        Expr::FieldAccess { target, field } => Expr::FieldAccess {
            target: Box::new(alpha_rename_locals(target, call_id, seq, renames, ctx)),
            field: field.clone(),
        },
        Expr::RecordInit { ty_name, fields } => {
            let new_fields = fields
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        alpha_rename_locals(v, call_id, seq, renames, ctx),
                    )
                })
                .collect();
            Expr::RecordInit {
                ty_name: ty_name.clone(),
                fields: new_fields,
            }
        }
        Expr::RecordUpdate { base, updates } => {
            let new_base = alpha_rename_locals(base, call_id, seq, renames, ctx);
            let new_updates = updates
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        alpha_rename_locals(v, call_id, seq, renames, ctx),
                    )
                })
                .collect();
            Expr::RecordUpdate {
                base: Box::new(new_base),
                updates: new_updates,
            }
        }
        Expr::Call { callee, args } => {
            let new_callee = alpha_rename_locals(callee, call_id, seq, renames, ctx);
            let new_args = args
                .iter()
                .map(|a| alpha_rename_locals(a, call_id, seq, renames, ctx))
                .collect();
            Expr::Call {
                callee: Box::new(new_callee),
                args: new_args,
            }
        }
        Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(alpha_rename_locals(left, call_id, seq, renames, ctx)),
            op: op.clone(),
            right: Box::new(alpha_rename_locals(right, call_id, seq, renames, ctx)),
        },
        Expr::UnaryOp { op, operand } => Expr::UnaryOp {
            op: op.clone(),
            operand: Box::new(alpha_rename_locals(operand, call_id, seq, renames, ctx)),
        },
        _ => expr.clone(),
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

                    if inlined_args.len() != target_func.params.len() {
                        return Err(format!(
                            "call to `{fn_name}` argument count mismatch: expected {}, got {}",
                            target_func.params.len(),
                            inlined_args.len()
                        ));
                    }
                    for (param, arg) in target_func.params.iter().zip(&inlined_args) {
                        let actual_ty = ctx.infer_type(arg);
                        if let Some(ref act) = actual_ty {
                            let effective_ty = match (&param.ty, arg) {
                                (Type::Decimal, Expr::Literal(Literal::Float(_))) => Type::Decimal,
                                _ => act.clone(),
                            };
                            if effective_ty != param.ty {
                                return Err(format!(
                                    "call argument type mismatch in inlining `{fn_name}`: parameter `{}` expects {:?}, found {:?}",
                                    param.name, param.ty, act
                                ));
                            }
                        }
                    }

                    let call_id = ctx.next_call_id();
                    let mut seq = 0usize;
                    let mut arg_bindings = Vec::with_capacity(target_func.params.len());
                    let mut param_subs = HashMap::new();

                    for (param, arg) in target_func.params.iter().zip(inlined_args) {
                        seq += 1;
                        let fresh_param = ctx.alloc_fresh_id(&param.name, call_id, seq);
                        arg_bindings.push(Expr::Let {
                            name: fresh_param.clone(),
                            value: Box::new(arg),
                        });
                        param_subs.insert(param.name.clone(), Expr::Ident(fresh_param));
                    }

                    let renamed_body = alpha_rename_locals(
                        &target_func.body,
                        call_id,
                        &mut seq,
                        &HashMap::new(),
                        ctx,
                    );
                    let replaced_body = substitute_expr(&renamed_body, &param_subs);
                    let inlined_body = inline_calls(&replaced_body, ctx, depth + 1)?;
                    return Ok(flatten_block(arg_bindings, Some(Box::new(inlined_body))));
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

fn expr_to_smt(
    expr: &Expr,
    var_types: &HashMap<String, &'static str>,
    ctx: &SmtContext,
) -> Result<String, String> {
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
        Expr::Literal(Literal::Decimal(n)) => {
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
            let inner = expr_to_smt(operand, var_types, ctx)?;
            match op {
                UnaryOperator::Negate => Ok(format!("(- {})", inner)),
                UnaryOperator::Not => Ok(format!("(not {})", inner)),
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let mut lt = ctx.infer_type(left);
            let mut rt = ctx.infer_type(right);

            // 上下文精化：若一方为 Decimal，且另一方为 FloatLiteral，则精化为 Decimal
            if lt == Some(Type::Decimal)
                && matches!(right.as_ref(), Expr::Literal(Literal::Float(_)))
            {
                rt = Some(Type::Decimal);
            } else if rt == Some(Type::Decimal)
                && matches!(left.as_ref(), Expr::Literal(Literal::Float(_)))
            {
                lt = Some(Type::Decimal);
            }

            let l = if (rt == Some(Type::Decimal) || rt == Some(Type::Int))
                && matches!(left.as_ref(), Expr::Literal(Literal::Float(_)))
            {
                if let Expr::Literal(Literal::Float(f)) = left.as_ref() {
                    let micros = axiom_ast::float_to_decimal_micros(*f);
                    if micros < 0 {
                        format!("(- {})", micros.abs())
                    } else {
                        micros.to_string()
                    }
                } else {
                    expr_to_smt(left, var_types, ctx)?
                }
            } else {
                expr_to_smt(left, var_types, ctx)?
            };

            let r = if (lt == Some(Type::Decimal) || lt == Some(Type::Int))
                && matches!(right.as_ref(), Expr::Literal(Literal::Float(_)))
            {
                if let Expr::Literal(Literal::Float(f)) = right.as_ref() {
                    let micros = axiom_ast::float_to_decimal_micros(*f);
                    if micros < 0 {
                        format!("(- {})", micros.abs())
                    } else {
                        micros.to_string()
                    }
                } else {
                    expr_to_smt(right, var_types, ctx)?
                }
            } else {
                expr_to_smt(right, var_types, ctx)?
            };

            match (lt, rt) {
                (Some(Type::Decimal), Some(Type::Decimal)) => match op {
                    BinaryOperator::Multiply => {
                        return Ok(emit_trunc_div_smt(&format!("(* {} {})", l, r), "1000000"));
                    }
                    BinaryOperator::Divide => {
                        return Ok(emit_trunc_div_smt(&format!("(* {} 1000000)", l), &r));
                    }
                    _ => {}
                },
                (Some(Type::Decimal), Some(Type::Int)) => match op {
                    BinaryOperator::Multiply => {
                        return Ok(format!("(* {} {})", l, r));
                    }
                    BinaryOperator::Divide => {
                        return Ok(emit_trunc_div_smt(&l, &r));
                    }
                    _ => {}
                },
                (Some(Type::Int), Some(Type::Decimal)) => match op {
                    BinaryOperator::Multiply => {
                        return Ok(format!("(* {} {})", l, r));
                    }
                    BinaryOperator::Divide => {
                        return Ok(emit_trunc_div_smt(&format!("(* {} 1000000000000)", l), &r));
                    }
                    _ => {}
                },
                _ => {
                    if ctx.is_decimal(left) && ctx.is_decimal(right) {
                        match op {
                            BinaryOperator::Multiply => {
                                return Ok(emit_trunc_div_smt(
                                    &format!("(* {} {})", l, r),
                                    "1000000",
                                ));
                            }
                            BinaryOperator::Divide => {
                                return Ok(emit_trunc_div_smt(&format!("(* {} 1000000)", l), &r));
                            }
                            _ => {}
                        }
                    }
                }
            }

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
            let val = expr_to_smt(value, var_types, ctx)?;
            Ok(format!("(let (({} {})))", name, val))
        }
        Expr::Block { statements, tail } => {
            if statements.is_empty() {
                if let Some(t) = tail {
                    expr_to_smt(t, var_types, ctx)
                } else {
                    Ok("0".into())
                }
            } else {
                build_block_smt(statements, tail.as_deref(), var_types, ctx)
            }
        }
        Expr::Call { callee, args } => {
            let callee_str = expr_to_smt(callee, var_types, ctx)?;
            let mut s = format!("({}", callee_str);
            for arg in args {
                s.push(' ');
                s.push_str(&expr_to_smt(arg, var_types, ctx)?);
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

            let cond_str = expr_to_smt(target, var_types, ctx)?;

            // 2. 若首臂即为不可反驳 catch-all，直接翻译该臂 body（若是 Ident 则建立词法绑定）
            if reachable_arms.len() == 1 {
                if let Pattern::Ident(ref id) = reachable_arms[0].pattern {
                    let mut arm_var_types = var_types.clone();
                    let empty_subs = HashMap::new();
                    let target_ty = infer_expr_type(target, var_types, &empty_subs);
                    arm_var_types.insert(id.clone(), target_ty);
                    let mut arm_ctx = ctx.clone();
                    let target_is_dec = ctx.is_decimal(target);
                    arm_ctx.local_decimals.insert(id.clone(), target_is_dec);
                    let inferred_target_ty = ctx.infer_type(target);
                    if let Some(ref mut env) = arm_ctx.type_env {
                        if let Some(ref ty) = inferred_target_ty {
                            env.insert(id.clone(), ty.clone());
                        }
                    }
                    let body_smt = expr_to_smt(&reachable_arms[0].body, &arm_var_types, &arm_ctx)?;
                    return Ok(format!("(let (({} {})) {})", id, cond_str, body_smt));
                } else if matches!(reachable_arms[0].pattern, Pattern::Wildcard) {
                    return expr_to_smt(&reachable_arms[0].body, var_types, ctx);
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
                    let mut arm_ctx = ctx.clone();
                    let target_is_dec = ctx.is_decimal(target);
                    arm_ctx.local_decimals.insert(id.clone(), target_is_dec);
                    let inferred_target_ty = ctx.infer_type(target);
                    if let Some(ref mut env) = arm_ctx.type_env {
                        if let Some(ref ty) = inferred_target_ty {
                            env.insert(id.clone(), ty.clone());
                        }
                    }
                    let body_smt = expr_to_smt(&last_arm.body, &arm_var_types, &arm_ctx)?;
                    format!("(let (({} {})) {})", id, cond_str, body_smt)
                } else {
                    expr_to_smt(&last_arm.body, var_types, ctx)?
                };

                for arm in reachable_arms[..last_idx].iter().rev() {
                    let (pat_cond_opt, _) = pattern_condition_smt(&arm.pattern, &cond_str)?;
                    let pat_cond = pat_cond_opt.ok_or_else(|| {
                        "unexpected catch-all before last reachable arm".to_string()
                    })?;
                    let body_smt = expr_to_smt(&arm.body, var_types, ctx)?;
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
                        let true_body = expr_to_smt(&reachable_arms[0].body, var_types, ctx)?;
                        let false_body = expr_to_smt(&reachable_arms[1].body, var_types, ctx)?;
                        return Ok(format!("(ite {} {} {})", cond_str, true_body, false_body));
                    }
                    (
                        Pattern::Literal(Literal::Bool(false)),
                        Pattern::Literal(Literal::Bool(true)),
                    ) => {
                        let false_body = expr_to_smt(&reachable_arms[0].body, var_types, ctx)?;
                        let true_body = expr_to_smt(&reachable_arms[1].body, var_types, ctx)?;
                        return Ok(format!("(ite {} {} {})", cond_str, true_body, false_body));
                    }
                    _ => {}
                }
            }

            Err("complex match pattern translation requires full ADT solver encoding or catch-all default".into())
        }
        Expr::Raw(_) => Err("Expr::Raw cannot be converted to SMT-LIB2".into()),
        _ => Err("unsupported expression in SMT translation".into()),
    }
}

/// 提取统一的模式匹配 SMT 条件生成器 (match_cond, neg_cond)
fn pattern_condition_smt(
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
        Pattern::Literal(Literal::Int(n)) | Pattern::Literal(Literal::Decimal(n)) => {
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
    ctx: &SmtContext,
) -> Result<String, String> {
    if statements.is_empty() {
        if let Some(t) = tail {
            return expr_to_smt(t, var_types, ctx);
        }
        return Ok("0".into());
    }

    let first = &statements[0];
    let rest = &statements[1..];

    if let Expr::Let { name, value } = first {
        let val_smt = expr_to_smt(value, var_types, ctx)?;
        let empty_subs = HashMap::new();
        let val_ty = infer_expr_type(value, var_types, &empty_subs);
        let mut child_var_types = var_types.clone();
        child_var_types.insert(name.clone(), val_ty);
        let mut child_ctx = ctx.clone();
        let is_dec = ctx.is_decimal(value);
        child_ctx.local_decimals.insert(name.clone(), is_dec);
        if let Some(ref mut env) = child_ctx.type_env {
            if let Some(v_ty) = env.infer_type(value) {
                env.insert(name.clone(), v_ty);
            }
        }
        let rest_smt = build_block_smt(rest, tail, &child_var_types, &child_ctx)?;
        Ok(format!("(let (({} {})) {})", name, val_smt, rest_smt))
    } else {
        build_block_smt(rest, tail, var_types, ctx)
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

    let dummy_mod = Module {
        types: vec![],
        functions: vec![func.clone()],
        effects: vec![],
    };
    let effective_module = module.unwrap_or(&dummy_mod);
    let mut ctx = SmtContext::from_module(effective_module);
    ctx.current_func = Some(func);
    ctx.type_env = Some(axiom_ast::TypeEnv::new(effective_module, Some(func)));

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
        let req_expr = expr_to_smt(&inlined_req, &var_types, &ctx)?;
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
        Expr::Literal(Literal::Int(_)) | Expr::Literal(Literal::Decimal(_)) => "Int",
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
    ctx: &SmtContext,
    depth: usize,
) -> Result<(), String> {
    if depth > 16 {
        return Err("recursion limit exceeded while collecting SMT obligations".into());
    }
    match expr {
        Expr::BinaryOp { left, op, right } => {
            collect_safety_obligations_recursive(
                left,
                var_types,
                subs,
                path_conditions,
                obligations,
                ctx,
                depth,
            )?;
            if *op == BinaryOperator::And {
                let left_expanded = substitute_expr_with_subs(left, subs);
                if let Ok(l_smt) = expr_to_smt(&left_expanded, var_types, ctx) {
                    let mut right_paths = path_conditions.to_vec();
                    right_paths.push(l_smt);
                    collect_safety_obligations_recursive(
                        right,
                        var_types,
                        subs,
                        &right_paths,
                        obligations,
                        ctx,
                        depth,
                    )?;
                } else {
                    collect_safety_obligations_recursive(
                        right,
                        var_types,
                        subs,
                        path_conditions,
                        obligations,
                        ctx,
                        depth,
                    )?;
                }
            } else if *op == BinaryOperator::Or {
                let left_expanded = substitute_expr_with_subs(left, subs);
                if let Ok(l_smt) = expr_to_smt(&left_expanded, var_types, ctx) {
                    let mut right_paths = path_conditions.to_vec();
                    right_paths.push(format!("(not {})", l_smt));
                    collect_safety_obligations_recursive(
                        right,
                        var_types,
                        subs,
                        &right_paths,
                        obligations,
                        ctx,
                        depth,
                    )?;
                } else {
                    collect_safety_obligations_recursive(
                        right,
                        var_types,
                        subs,
                        path_conditions,
                        obligations,
                        ctx,
                        depth,
                    )?;
                }
            } else {
                collect_safety_obligations_recursive(
                    right,
                    var_types,
                    subs,
                    path_conditions,
                    obligations,
                    ctx,
                    depth,
                )?;
            }

            let left_expanded = substitute_expr_with_subs(left, subs);
            let right_expanded = substitute_expr_with_subs(right, subs);

            let mut lt = ctx.infer_type(left);
            let mut rt = ctx.infer_type(right);

            // 上下文精化：若一方为 Decimal，且另一方为 FloatLiteral，则精化为 Decimal
            if lt == Some(Type::Decimal)
                && matches!(right.as_ref(), Expr::Literal(Literal::Float(_)))
            {
                rt = Some(Type::Decimal);
            } else if rt == Some(Type::Decimal)
                && matches!(left.as_ref(), Expr::Literal(Literal::Float(_)))
            {
                lt = Some(Type::Decimal);
            }

            let is_real = if lt == Some(Type::Decimal) || rt == Some(Type::Decimal) {
                false
            } else {
                infer_expr_type(left, var_types, subs) == "Real"
                    || infer_expr_type(right, var_types, subs) == "Real"
            };

            if !is_real {
                let l_smt = if (rt == Some(Type::Decimal) || rt == Some(Type::Int))
                    && matches!(left_expanded, Expr::Literal(Literal::Float(_)))
                {
                    if let Expr::Literal(Literal::Float(f)) = left_expanded {
                        let micros = axiom_ast::float_to_decimal_micros(f);
                        if micros < 0 {
                            format!("(- {})", micros.abs())
                        } else {
                            micros.to_string()
                        }
                    } else {
                        expr_to_smt(&left_expanded, var_types, ctx)?
                    }
                } else {
                    expr_to_smt(&left_expanded, var_types, ctx)?
                };

                let r_smt = if (lt == Some(Type::Decimal) || lt == Some(Type::Int))
                    && matches!(right_expanded, Expr::Literal(Literal::Float(_)))
                {
                    if let Expr::Literal(Literal::Float(f)) = right_expanded {
                        let micros = axiom_ast::float_to_decimal_micros(f);
                        if micros < 0 {
                            format!("(- {})", micros.abs())
                        } else {
                            micros.to_string()
                        }
                    } else {
                        expr_to_smt(&right_expanded, var_types, ctx)?
                    }
                } else {
                    expr_to_smt(&right_expanded, var_types, ctx)?
                };

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
                        if lt == Some(Type::Decimal) && rt == Some(Type::Decimal) {
                            // 1. i128 intermediate safety obligation
                            let intermediate = format!("(* {} {})", l_smt, r_smt);
                            let intermediate_ob = format!(
                                "(and (>= {} (- 170141183460469231731687303715884105728)) (<= {} 170141183460469231731687303715884105727))",
                                intermediate, intermediate
                            );
                            emit_safety_obligation(intermediate_ob, path_conditions, obligations);

                            // 2. i64 result safety obligation after fixed-point scaling recovery
                            let scaled = emit_trunc_div_smt(&intermediate, "1000000");
                            let result_ob = format!(
                                "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                                scaled, scaled
                            );
                            emit_safety_obligation(result_ob, path_conditions, obligations);
                        } else if (lt == Some(Type::Decimal) && rt == Some(Type::Int))
                            || (lt == Some(Type::Int) && rt == Some(Type::Decimal))
                        {
                            let intermediate = format!("(* {} {})", l_smt, r_smt);
                            let intermediate_ob = format!(
                                "(and (>= {} (- 170141183460469231731687303715884105728)) (<= {} 170141183460469231731687303715884105727))",
                                intermediate, intermediate
                            );
                            emit_safety_obligation(intermediate_ob, path_conditions, obligations);

                            let result_ob = format!(
                                "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                                intermediate, intermediate
                            );
                            emit_safety_obligation(result_ob, path_conditions, obligations);
                        } else {
                            let op_smt = format!("(* {} {})", l_smt, r_smt);
                            let ob = format!(
                                "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                                op_smt, op_smt
                            );
                            emit_safety_obligation(ob, path_conditions, obligations);
                        }
                    }
                    BinaryOperator::Divide => {
                        emit_safety_obligation(
                            format!("(not (= {} 0))", r_smt),
                            path_conditions,
                            obligations,
                        );
                        if lt == Some(Type::Decimal) && rt == Some(Type::Decimal) {
                            // 1. i128 intermediate safety obligation for left * 1_000_000
                            let intermediate = format!("(* {} 1000000)", l_smt);
                            let intermediate_ob = format!(
                                "(and (>= {} (- 170141183460469231731687303715884105728)) (<= {} 170141183460469231731687303715884105727))",
                                intermediate, intermediate
                            );
                            emit_safety_obligation(intermediate_ob, path_conditions, obligations);

                            // 2. i64 result safety obligation after fixed-point scaling recovery
                            let scaled = emit_trunc_div_smt(&intermediate, &r_smt);
                            let result_ob = format!(
                                "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                                scaled, scaled
                            );
                            emit_safety_obligation(result_ob, path_conditions, obligations);
                        } else if lt == Some(Type::Decimal) && rt == Some(Type::Int) {
                            let scaled = emit_trunc_div_smt(&l_smt, &r_smt);
                            let result_ob = format!(
                                "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                                scaled, scaled
                            );
                            emit_safety_obligation(result_ob, path_conditions, obligations);
                        } else if lt == Some(Type::Int) && rt == Some(Type::Decimal) {
                            // 1. i128 intermediate safety obligation for left * 1_000_000_000_000
                            let intermediate = format!("(* {} 1000000000000)", l_smt);
                            let intermediate_ob = format!(
                                "(and (>= {} (- 170141183460469231731687303715884105728)) (<= {} 170141183460469231731687303715884105727))",
                                intermediate, intermediate
                            );
                            emit_safety_obligation(intermediate_ob, path_conditions, obligations);

                            // 2. i64 result safety obligation after fixed-point scaling recovery
                            let scaled = emit_trunc_div_smt(&intermediate, &r_smt);
                            let result_ob = format!(
                                "(and (>= {} (- 9223372036854775808)) (<= {} 9223372036854775807))",
                                scaled, scaled
                            );
                            emit_safety_obligation(result_ob, path_conditions, obligations);
                        } else {
                            emit_safety_obligation(
                                format!(
                                    "(not (and (= {} (- 9223372036854775808)) (= {} (- 1))))",
                                    l_smt, r_smt
                                ),
                                path_conditions,
                                obligations,
                            );
                        }
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
                ctx,
                depth,
            )?;
            if *op == UnaryOperator::Negate {
                let is_real = infer_expr_type(operand, var_types, subs) == "Real";
                if !is_real {
                    let operand_expanded = substitute_expr_with_subs(operand, subs);
                    let inner_smt = expr_to_smt(&operand_expanded, var_types, ctx)?;
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
                ctx,
                depth,
            )?;
            let expanded_val = substitute_expr_with_subs(value, subs);
            subs.insert(name.clone(), expanded_val);
        }
        Expr::Block { statements, tail } => {
            let mut curr_subs = subs.clone();
            let mut curr_ctx = ctx.clone();
            if let Some(ref mut env) = curr_ctx.type_env {
                env.push_scope();
            }
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    collect_safety_obligations_recursive(
                        value,
                        var_types,
                        &mut curr_subs,
                        path_conditions,
                        obligations,
                        &curr_ctx,
                        depth,
                    )?;
                    let expanded_val = substitute_expr_with_subs(value, &curr_subs);
                    let is_dec = curr_ctx.is_decimal(value);
                    curr_ctx.local_decimals.insert(name.clone(), is_dec);
                    let val_ty = curr_ctx.infer_type(value);
                    if let Some(ref mut env) = curr_ctx.type_env {
                        if let Some(ref ty) = val_ty {
                            env.insert(name.clone(), ty.clone());
                        }
                    }
                    curr_subs.insert(name.clone(), expanded_val);
                } else {
                    collect_safety_obligations_recursive(
                        stmt,
                        var_types,
                        &mut curr_subs,
                        path_conditions,
                        obligations,
                        &curr_ctx,
                        depth,
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
                    &curr_ctx,
                    depth,
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
                ctx,
                depth,
            )?;
            let target_expanded = substitute_expr_with_subs(target, subs);
            let target_smt_res = expr_to_smt(&target_expanded, var_types, ctx);
            let target_ty = ctx.infer_type(target);

            if let Ok(target_smt) = target_smt_res {
                let mut accumulated_negations: Vec<String> = path_conditions.to_vec();
                for arm in arms {
                    let mut arm_subs = subs.clone();
                    let mut arm_ctx = ctx.clone();
                    if let Pattern::Ident(ref id) = arm.pattern {
                        arm_subs.insert(id.clone(), target_expanded.clone());
                        let is_dec = ctx.is_decimal(target);
                        arm_ctx.local_decimals.insert(id.clone(), is_dec);
                        if let Some(ref mut env) = arm_ctx.type_env {
                            if let Some(ref ty) = target_ty {
                                env.insert(id.clone(), ty.clone());
                            }
                        }
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
                        &arm_ctx,
                        depth,
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
                    let mut arm_ctx = ctx.clone();
                    if let Pattern::Ident(ref id) = arm.pattern {
                        arm_subs.remove(id);
                        let is_dec = ctx.is_decimal(target);
                        arm_ctx.local_decimals.insert(id.clone(), is_dec);
                        if let Some(ref mut env) = arm_ctx.type_env {
                            if let Some(ref ty) = target_ty {
                                env.insert(id.clone(), ty.clone());
                            }
                        }
                    }
                    let is_catch_all = matches!(arm.pattern, Pattern::Wildcard | Pattern::Ident(_));
                    collect_safety_obligations_recursive(
                        &arm.body,
                        var_types,
                        &mut arm_subs,
                        path_conditions,
                        obligations,
                        &arm_ctx,
                        depth,
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
                ctx,
                depth,
            )?;
            for arg in args {
                collect_safety_obligations_recursive(
                    arg,
                    var_types,
                    subs,
                    path_conditions,
                    obligations,
                    ctx,
                    depth,
                )?;
            }

            if let Expr::Ident(ref fn_name) = **callee {
                if let Some(target_func) = ctx.functions.get(fn_name) {
                    if !target_func.effects.is_empty() {
                        return Err(format!(
                            "cannot verify function calling effectful `{fn_name}`"
                        ));
                    }
                    let mut param_subs = HashMap::new();
                    for (param, arg) in target_func.params.iter().zip(args) {
                        let arg_expanded = substitute_expr_with_subs(arg, subs);
                        param_subs.insert(param.name.clone(), arg_expanded);
                    }
                    if !target_func.requires.is_empty() {
                        for req in &target_func.requires {
                            let substituted_req = substitute_expr(req, &param_subs);
                            let fully_expanded_req =
                                substitute_expr_with_subs(&substituted_req, subs);
                            let inlined_req = inline_calls(&fully_expanded_req, ctx, 0)?;
                            let req_smt = expr_to_smt(&inlined_req, var_types, ctx)?;
                            emit_safety_obligation(req_smt, path_conditions, obligations);
                        }
                    }
                }
            }
        }
        Expr::FieldAccess { target, .. } => {
            collect_safety_obligations_recursive(
                target,
                var_types,
                subs,
                path_conditions,
                obligations,
                ctx,
                depth,
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
                    ctx,
                    depth,
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
                ctx,
                depth,
            )?;
            for (_, val) in updates {
                collect_safety_obligations_recursive(
                    val,
                    var_types,
                    subs,
                    path_conditions,
                    obligations,
                    ctx,
                    depth,
                )?;
            }
        }
        Expr::Raw(_) => {
            return Err("Expr::Raw is forbidden in formal safety verification".into());
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) => {}
    }
    Ok(())
}
