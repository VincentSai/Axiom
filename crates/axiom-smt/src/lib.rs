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

const IEEE754_MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0; // 2^53 - 1

fn extract_path_string(expr: &axiom_ast::Expr) -> Option<String> {
    match expr {
        axiom_ast::Expr::Ident(name) => Some(name.clone()),
        axiom_ast::Expr::FieldAccess { target, field } => {
            extract_path_string(target).map(|t| format!("{t}.{field}"))
        }
        _ => None,
    }
}

fn extract_numeric_constant(expr: &axiom_ast::Expr) -> Option<f64> {
    match expr {
        axiom_ast::Expr::Literal(axiom_ast::Literal::Float(f)) => Some(*f),
        axiom_ast::Expr::Literal(axiom_ast::Literal::Int(i)) => Some(*i as f64),
        axiom_ast::Expr::UnaryOp {
            op: axiom_ast::UnaryOperator::Negate,
            operand,
        } => extract_numeric_constant(operand).map(|v| -v),
        _ => None,
    }
}

fn record_bounds(
    expr: &axiom_ast::Expr,
    has_lower: &mut HashMap<String, bool>,
    has_upper: &mut HashMap<String, bool>,
) {
    if let axiom_ast::Expr::BinaryOp { left, op, right } = expr {
        match op {
            axiom_ast::BinaryOperator::And => {
                record_bounds(left, has_lower, has_upper);
                record_bounds(right, has_lower, has_upper);
            }
            axiom_ast::BinaryOperator::GreaterThan
            | axiom_ast::BinaryOperator::GreaterThanOrEqual => {
                if let Some(path) = extract_path_string(left) {
                    if let Some(c) = extract_numeric_constant(right) {
                        if c.abs() <= IEEE754_MAX_SAFE_INTEGER {
                            has_lower.insert(path, true);
                        }
                    }
                } else if let Some(path) = extract_path_string(right) {
                    if let Some(c) = extract_numeric_constant(left) {
                        if c.abs() <= IEEE754_MAX_SAFE_INTEGER {
                            has_upper.insert(path, true);
                        }
                    }
                }
            }
            axiom_ast::BinaryOperator::LessThan | axiom_ast::BinaryOperator::LessThanOrEqual => {
                if let Some(path) = extract_path_string(left) {
                    if let Some(c) = extract_numeric_constant(right) {
                        if c.abs() <= IEEE754_MAX_SAFE_INTEGER {
                            has_upper.insert(path, true);
                        }
                    }
                } else if let Some(path) = extract_path_string(right) {
                    if let Some(c) = extract_numeric_constant(left) {
                        if c.abs() <= IEEE754_MAX_SAFE_INTEGER {
                            has_lower.insert(path, true);
                        }
                    }
                }
            }
            axiom_ast::BinaryOperator::Equal => {
                if let Some(path) = extract_path_string(left) {
                    if let Some(c) = extract_numeric_constant(right) {
                        if c.abs() <= IEEE754_MAX_SAFE_INTEGER {
                            has_lower.insert(path.clone(), true);
                            has_upper.insert(path, true);
                        }
                    }
                } else if let Some(path) = extract_path_string(right) {
                    if let Some(c) = extract_numeric_constant(left) {
                        if c.abs() <= IEEE754_MAX_SAFE_INTEGER {
                            has_lower.insert(path.clone(), true);
                            has_upper.insert(path, true);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// 判定一个函数是否包含在实数理论下不健全的浮点数契约 (Unsound/Unbounded Float Contract)。
///
/// 当浮点参数未在 `requires` 中严格限定在 IEEE-754 安全有效位区间 ([-2^53 + 1, 2^53 - 1]) 时，
/// 实数理论 (Real) 的数学证明可能在机器运行时因精度截断发生分歧（如 2^53 + 1.0 == 2^53）。
/// 此时必须 Fail-Closed 拦截并返回 Skipped，严禁给出 Proved 误导。
///
/// 注意：仅对函数体或契约中实际被访问/引用的浮点路径进行有界性检查，未被引用的字段不施加虚假约束。
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

    let type_env = axiom_ast::TypeEnv::new(mod_ref, Some(func));

    let mut referenced_float_paths = std::collections::HashSet::new();
    let mut visit_for_floats = |expr: &axiom_ast::Expr| {
        if let Some(path) = extract_path_string(expr) {
            let root = path.split('.').next().unwrap_or("");
            if func.params.iter().any(|p| p.name == root)
                && type_env.infer_type(expr) == Some(axiom_ast::Type::Float)
            {
                referenced_float_paths.insert(path);
            }
        }
    };

    axiom_ast::visit_expr(&func.body, &mut visit_for_floats);
    for req in &func.requires {
        axiom_ast::visit_expr(req, &mut visit_for_floats);
    }
    for ens in &func.ensures {
        axiom_ast::visit_expr(ens, &mut visit_for_floats);
    }

    let returns_float = func.return_type == axiom_ast::Type::Float;
    if referenced_float_paths.is_empty() && !returns_float {
        return false;
    }

    if referenced_float_paths.is_empty() && returns_float {
        return true;
    }

    let mut has_lower = HashMap::new();
    let mut has_upper = HashMap::new();
    for req in &func.requires {
        record_bounds(req, &mut has_lower, &mut has_upper);
    }

    for path in &referenced_float_paths {
        let lower_ok = has_lower.get(path).copied().unwrap_or(false);
        let upper_ok = has_upper.get(path).copied().unwrap_or(false);
        if !lower_ok || !upper_ok {
            return true;
        }
    }

    false
}

/// Verifies a function using full module context for record flattening and pure function calls.
pub fn verify_function_in_module(func: &FunctionDecl, module: &Module) -> SmtResult {
    if function_has_unsound_float_contract(func, Some(module)) {
        return SmtResult::Skipped(
            "unbounded floating-point arithmetic is not verified soundly under real theory (IEEE-754 divergence prevented)".into(),
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
            "unbounded floating-point arithmetic is not verified soundly under real theory (IEEE-754 divergence prevented)".into()
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
