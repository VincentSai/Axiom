//! SMT-LIB2 & Z3 Formal Verification Engine for Axiom.
//!
//! Provides two verification pathways:
//!   1. First-order full domain SMT theorem proving via Microsoft Z3 (or any SMT-LIB2 solver).
//!   2. Built-in standalone symbolic interval and boundary grid verification.

pub mod cascade;
pub mod internal_solver;
pub mod process_supervisor;
pub mod smtlib2;
pub mod z3_solver;

use std::collections::HashMap;

use axiom_ast::{FunctionDecl, Module};
pub use process_supervisor::{
    execute_process_until_deadline, execute_supervised_command, ProcessExecutionOutput,
    ProcessSupervisor, TimeoutPolicy,
};
pub use smtlib2::{
    generate_precondition_check_smtlib2, generate_smtlib2, generate_smtlib2_and_obligations,
    generate_smtlib2_with_module, generate_smtlib2_with_module_and_obligations,
};
pub use z3_solver::{
    compute_file_sha256, find_z3_executable, get_z3_version, resolve_solver_identity, run_z3_query,
    SealedSolverBinary, SolverIdentity, Z3Result,
};

/// Concrete numeric parameter assignments that demonstrate a contract violation.
pub type CounterExample = HashMap<String, f64>;

/// The solver backend that performed the verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverBackend {
    /// Microsoft Z3 SMT Solver (Full First-Order Mathematical Theorem Proved)
    Z3,
    /// Built-in Standalone Symbolic Engine
    InternalSymbolic,
}

/// The result of checking one function's contracts with strict evidence grading.
#[derive(Debug, Clone, PartialEq)]
pub enum SmtResult {
    /// Formula is proved globally by an external SMT solver (e.g. Microsoft Z3 Unsat).
    Proved {
        solver: String,
        solver_path: Option<String>,
        solver_sha256: Option<String>,
        version: String,
        obligations_checked: usize,
    },
    /// Verified only on a bounded discrete grid without counterexamples (heuristic test, NOT a theorem proof).
    Sampled { points_checked: usize },
    /// A concrete input satisfying preconditions violates a postcondition.
    CounterExample(CounterExample),
    /// Preconditions are logically unsatisfiable (unsat), creating a vacuous pseudo-proof.
    VacuousContract { reason: String },
    /// Verification could not be performed for the given function.
    Skipped(String),
    /// Solver returned unknown or timed out.
    Unknown(String),
}

impl SmtResult {
    /// Returns true if this function is strictly mathematically proved.
    pub fn is_proved(&self) -> bool {
        matches!(self, Self::Proved { .. })
    }

    /// Returns true if the contract was rejected due to vacuous (unsatisfiable) preconditions.
    pub fn is_vacuous(&self) -> bool {
        matches!(self, Self::VacuousContract { .. })
    }
}

/// Checks precondition satisfiability (vacuity prevention) using fail-closed policy.
///
/// If preconditions are unsatisfiable (unsat), returns `Err(SmtResult::VacuousContract)`.
/// If solver returns unknown or encounters an error, returns `Err(SmtResult::Unknown)`.
/// Only returns `Ok(())` if preconditions are proven satisfiable (`Sat`).
pub fn check_precondition_vacuity_fail_closed(
    z3_path: &std::path::Path,
    func: &FunctionDecl,
    module: Option<&Module>,
) -> Result<(), SmtResult> {
    if func.requires.is_empty() {
        return Ok(());
    }

    let vacuity_query = match generate_precondition_check_smtlib2(func, module) {
        Ok(q) if !q.is_empty() => q,
        Ok(_) => return Ok(()),
        Err(err) => {
            return Err(SmtResult::Unknown(format!(
                "Failed to generate precondition check query for `{}`: {}. Aborting to prevent vacuous proof.",
                func.name, err
            )));
        }
    };

    match run_z3_query(z3_path, &vacuity_query) {
        Ok(Z3Result::Sat(_)) => Ok(()),
        Ok(Z3Result::Unsat) => Err(SmtResult::VacuousContract {
            reason: format!(
                "Preconditions of function `{}` are logically unsatisfiable (unsat). This creates a vacuous proof where arbitrary implementations would trivially pass.",
                func.name
            ),
        }),
        Ok(Z3Result::Unknown(reason)) => Err(SmtResult::Unknown(format!(
            "Precondition satisfiability for `{}` is unknown ({}). Aborting to prevent vacuous proof.",
            func.name, reason
        ))),
        Err(err) => Err(SmtResult::Unknown(format!(
            "Vacuity check solver error for `{}`: {}. Aborting to prevent vacuous proof.",
            func.name, err
        ))),
    }
}

/// Translates an Axiom function to standard SMT-LIB2 format.
pub fn to_smtlib2(func: &FunctionDecl) -> Result<String, String> {
    generate_smtlib2(func)
}

/// Verifies a single function in isolation without module context.
pub fn verify_function(func: &FunctionDecl) -> SmtResult {
    let dummy_module = Module {
        types: vec![],
        functions: vec![func.clone()],
        effects: vec![],
    };
    verify_function_in_module(func, &dummy_module)
}

fn expr_contains_float_operation(
    expr: &axiom_ast::Expr,
    type_env: &mut axiom_ast::TypeEnv,
) -> bool {
    match expr {
        axiom_ast::Expr::Ident(name) => type_env.lookup(name) == Some(axiom_ast::Type::Float),
        axiom_ast::Expr::Call { callee, args } => {
            if let axiom_ast::Expr::Ident(fn_name) = callee.as_ref() {
                if let Some(f) = type_env
                    .module()
                    .functions
                    .iter()
                    .find(|f| &f.name == fn_name)
                {
                    if f.return_type == axiom_ast::Type::Float {
                        return true;
                    }
                }
            }
            args.iter()
                .any(|a| expr_contains_float_operation(a, type_env))
        }
        axiom_ast::Expr::BinaryOp { left, right, .. } => {
            let lt = type_env.infer_type(left);
            let rt = type_env.infer_type(right);
            // 仅当涉及 Float 且未被 Decimal 规则提升覆盖时判定为 Float 操作
            if (lt == Some(axiom_ast::Type::Float) || rt == Some(axiom_ast::Type::Float))
                && lt != Some(axiom_ast::Type::Decimal)
                && rt != Some(axiom_ast::Type::Decimal)
            {
                return true;
            }
            expr_contains_float_operation(left, type_env)
                || expr_contains_float_operation(right, type_env)
        }
        axiom_ast::Expr::UnaryOp { op, operand } => {
            if *op == axiom_ast::UnaryOperator::Negate
                && type_env.infer_type(operand) == Some(axiom_ast::Type::Float)
            {
                return true;
            }
            expr_contains_float_operation(operand, type_env)
        }
        axiom_ast::Expr::Block { statements, tail } => {
            let mut child_env = type_env.clone();
            child_env.push_scope();
            for stmt in statements {
                if let axiom_ast::Expr::Let { name, value } = stmt {
                    if expr_contains_float_operation(value, &mut child_env) {
                        return true;
                    }
                    if let Some(ty) = child_env.infer_type(value) {
                        child_env.insert(name.clone(), ty);
                    }
                } else if expr_contains_float_operation(stmt, &mut child_env) {
                    return true;
                }
            }
            if let Some(t) = tail {
                expr_contains_float_operation(t, &mut child_env)
            } else {
                false
            }
        }
        axiom_ast::Expr::Match { target, arms } => {
            if expr_contains_float_operation(target, type_env) {
                return true;
            }
            let target_ty = type_env.infer_type(target);
            for arm in arms {
                let mut child_env = type_env.clone();
                child_env.push_scope();
                if let axiom_ast::Pattern::Ident(ref id) = arm.pattern {
                    if let Some(ref ty) = target_ty {
                        child_env.insert(id.clone(), ty.clone());
                    }
                }
                if expr_contains_float_operation(&arm.body, &mut child_env) {
                    return true;
                }
            }
            false
        }
        axiom_ast::Expr::Let { name, value } => {
            if expr_contains_float_operation(value, type_env) {
                return true;
            }
            if let Some(ty) = type_env.infer_type(value) {
                type_env.insert(name.clone(), ty);
            }
            false
        }
        axiom_ast::Expr::RecordInit { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_contains_float_operation(v, type_env)),
        axiom_ast::Expr::RecordUpdate { base, updates } => {
            expr_contains_float_operation(base, type_env)
                || updates
                    .iter()
                    .any(|(_, v)| expr_contains_float_operation(v, type_env))
        }
        axiom_ast::Expr::FieldAccess { target, .. } => {
            expr_contains_float_operation(target, type_env)
        }
        _ => false,
    }
}

/// 判定一个函数本地是否直接包含未受支持的浮点特性（Float 参数、Float 返回值、直接浮点算术/比较/字面量）
fn function_has_local_unsupported_float(func: &FunctionDecl, module: &Module) -> bool {
    if func.return_type == axiom_ast::Type::Float {
        return true;
    }
    if func.params.iter().any(|p| p.ty == axiom_ast::Type::Float) {
        return true;
    }
    let mut type_env = axiom_ast::TypeEnv::new(module, Some(func));
    if expr_contains_float_operation(&func.body, &mut type_env) {
        return true;
    }
    for req in &func.requires {
        if expr_contains_float_operation(req, &mut type_env) {
            return true;
        }
    }
    for ens in &func.ensures {
        if expr_contains_float_operation(ens, &mut type_env) {
            return true;
        }
    }
    false
}

/// 判定一个函数是否包含在实数理论下不健全的浮点数契约 (Unsound Float Contract)。
///
/// 在未接入严格的 IEEE-754 浮点理论 (Q_FPA) 与形式化误差界之前，
/// 实数理论 (Real) 的数学证明可能在机器运行时因精度舍入发生分歧（如 0.1 + 0.2 != 0.3，或 2^53 + 1.0 == 2^53）。
/// 即使参数限定在有限区间内，有限字长浮点算术依然存在舍入误差。
///
/// 特别地，任何直接或【间接】调用了涉及浮点运算辅助函数的契约函数，
/// 其数值语义均受到非健全浮点理论的污染。
/// 因此，带契约函数若自身或其传递调用闭包中涉及任何浮点运算/参数/返回，
/// 必须实施严格 Fail-Closed 拦截并返回 Skipped，严禁给出 Proved 误导。
fn function_has_unsound_float_contract(func: &FunctionDecl, module: Option<&Module>) -> bool {
    if !func.has_contracts() {
        return false;
    }

    let dummy_module;
    let mod_ref = match module {
        Some(m) => m,
        None => {
            dummy_module = Module {
                types: vec![],
                functions: vec![func.clone()],
                effects: vec![],
            };
            &dummy_module
        }
    };

    // 1. 查找模块中所有本地包含浮点的函数集合（Float Seeds）
    let float_seeds: Vec<&str> = mod_ref
        .functions
        .iter()
        .filter(|f| function_has_local_unsupported_float(f, mod_ref))
        .map(|f| f.name.as_str())
        .collect();

    if float_seeds.is_empty() {
        return false;
    }

    // 2. 若 func 自身就在 float_seeds 中，直接拦截
    if float_seeds.contains(&func.name.as_str()) {
        return true;
    }

    // 3. 构建调用图，计算受到 float_seeds 污染的调用者闭包 (Callers)
    let graph = axiom_ast::analysis::CallGraph::from_module(mod_ref);
    let tainted_callers = graph.transitive_dependents(&float_seeds);
    if tainted_callers.contains(&func.name) {
        return true;
    }

    // 4. 双向核验：计算当前函数向下的被调用依赖闭包 (Callees)
    let my_dependencies = graph.transitive_dependencies(&[&func.name]);
    for seed in &float_seeds {
        if my_dependencies.contains(*seed) {
            return true;
        }
    }

    false
}

/// Verifies a function using full module context for record flattening and pure function calls.
pub fn verify_function_in_module(func: &FunctionDecl, module: &Module) -> SmtResult {
    if function_has_unsound_float_contract(func, Some(module)) {
        return SmtResult::Skipped(
            "floating-point arithmetic is not verified soundly under real theory (IEEE-754 divergence prevented)".into(),
        );
    }

    if let Some(z3_path) = find_z3_executable() {
        // [Round 20 P1 Spec 3]: 构造只读内容寻址密封沙箱副本，彻底消除 TOCTOU
        let sealed_solver = match SealedSolverBinary::seal_from_path(&z3_path) {
            Ok(sealed) => sealed,
            Err(e) => return SmtResult::Unknown(format!("failed to seal solver binary: {e}")),
        };

        if let Ok((smt2_query, num_obligations)) =
            generate_smtlib2_with_module_and_obligations(func, Some(module))
        {
            // P0: Precondition Satisfiability Check (Vacuity Prevention)
            // 绑定密封沙箱副本执行
            if let Err(vacuity_err) = check_precondition_vacuity_fail_closed(
                &sealed_solver.sealed_path,
                func,
                Some(module),
            ) {
                return vacuity_err;
            }

            match run_z3_query(&sealed_solver.sealed_path, &smt2_query) {
                Ok(Z3Result::Unsat) => {
                    // 复核密封沙箱副本完整性
                    if !sealed_solver.verify_post_integrity() {
                        return SmtResult::Unknown(
                            "TOCTOU integrity check failed: sealed solver binary modified during verification".into(),
                        );
                    }
                    return SmtResult::Proved {
                        solver: "Microsoft Z3 (SMT-LIB2)".into(),
                        solver_path: Some(sealed_solver.source_path.to_string_lossy().to_string()),
                        solver_sha256: Some(sealed_solver.sha256),
                        version: sealed_solver.version,
                        obligations_checked: num_obligations,
                    };
                }
                Ok(Z3Result::Sat(ce)) => return SmtResult::CounterExample(ce),
                Ok(Z3Result::Unknown(reason)) => return SmtResult::Unknown(reason),
                Err(_) => {
                    // Fall through to internal solver on invocation error
                }
            }
        }
    }

    let ctx = internal_solver::SolverContext::from_module(module);
    internal_solver::verify_with_internal_solver_context(func, Some(&ctx))
}

/// Verifies a function explicitly using the Z3 solver via SMT-LIB2 pipe.
pub fn verify_function_with_z3(func: &FunctionDecl) -> Result<SmtResult, String> {
    if function_has_unsound_float_contract(func, None) {
        return Ok(SmtResult::Skipped(
            "floating-point arithmetic is not verified soundly under real theory (IEEE-754 divergence prevented)".into(),
        ));
    }

    let z3_path = find_z3_executable().ok_or_else(|| {
        "Z3 executable not found in PATH or standard install directories".to_string()
    })?;

    // [Round 20 P1 Spec 3]: 构造只读内容寻址密封沙箱副本，彻底消除 TOCTOU
    let sealed_solver = SealedSolverBinary::seal_from_path(&z3_path)
        .map_err(|e| format!("failed to seal solver binary: {e}"))?;

    // P0: Precondition Satisfiability Check (Vacuity Prevention)
    if let Err(vacuity_err) =
        check_precondition_vacuity_fail_closed(&sealed_solver.sealed_path, func, None)
    {
        return Ok(vacuity_err);
    }

    let (smt2_query, num_obligations) = generate_smtlib2_and_obligations(func)?;

    match run_z3_query(&sealed_solver.sealed_path, &smt2_query)? {
        Z3Result::Unsat => {
            if !sealed_solver.verify_post_integrity() {
                return Ok(SmtResult::Unknown(
                    "TOCTOU integrity check failed: sealed solver binary modified during verification"
                        .into(),
                ));
            }
            Ok(SmtResult::Proved {
                solver: "Microsoft Z3 (SMT-LIB2)".into(),
                solver_path: Some(sealed_solver.source_path.to_string_lossy().to_string()),
                solver_sha256: Some(sealed_solver.sha256),
                version: sealed_solver.version,
                obligations_checked: num_obligations,
            })
        }
        Z3Result::Sat(ce) => Ok(SmtResult::CounterExample(ce)),
        Z3Result::Unknown(r) => Ok(SmtResult::Unknown(r)),
    }
}

/// Verify all functions in a module, keyed by function name, with full module context.
pub fn verify_module(module: &Module) -> HashMap<String, SmtResult> {
    module
        .functions
        .iter()
        .map(|function| {
            (
                function.name.clone(),
                verify_function_in_module(function, module),
            )
        })
        .collect()
}
