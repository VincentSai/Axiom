//! SMT-LIB2 & Z3 Formal Verification Engine for Axiom.
//!
//! Provides two verification pathways:
//!   1. First-order full domain SMT theorem proving via Microsoft Z3 (or any SMT-LIB2 solver).
//!   2. Built-in standalone symbolic interval and boundary grid verification.

pub mod internal_solver;
pub mod smtlib2;
pub mod z3_solver;

use std::collections::HashMap;

use axiom_ast::{FunctionDecl, Module};
pub use smtlib2::generate_smtlib2;
pub use z3_solver::{find_z3_executable, run_z3_query, Z3Result};

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

/// The result of checking one function's contracts.
#[derive(Debug, Clone, PartialEq)]
pub enum SmtResult {
    /// Formula is proved globally by an external SMT solver (e.g. Microsoft Z3 Unsat).
    Proved { solver: String },
    /// Verified only on a bounded discrete grid without counterexamples (heuristic test, NOT a theorem proof).
    Sampled { points_checked: usize },
    /// A concrete input satisfying preconditions violates a postcondition.
    CounterExample(CounterExample),
    /// Verification could not be performed for the given function.
    Skipped(String),
    /// Solver returned unknown or timed out.
    Unknown(String),
}

/// Translates an Axiom function to standard SMT-LIB2 format.
pub fn to_smtlib2(func: &FunctionDecl) -> Result<String, String> {
    generate_smtlib2(func)
}

/// Verifies that a function body satisfies every `ensures` clause whenever all `requires` clauses hold.
/// Automatically probes for external Z3 executable; falls back to internal symbolic engine if Z3 is absent.
pub fn verify_function(func: &FunctionDecl) -> SmtResult {
    if let Some(z3_path) = find_z3_executable() {
        if let Ok(smt2_query) = generate_smtlib2(func) {
            match run_z3_query(&z3_path, &smt2_query) {
                Ok(Z3Result::Unsat) => {
                    return SmtResult::Proved {
                        solver: "Microsoft Z3 (SMT-LIB2)".into(),
                    }
                }
                Ok(Z3Result::Sat(ce)) => return SmtResult::CounterExample(ce),
                Ok(Z3Result::Unknown(reason)) => return SmtResult::Unknown(reason),
                Err(_) => {
                    // Fall through to internal solver on invocation error
                }
            }
        }
    }

    // Standalone fallback solver (discrete grid sampling)
    internal_solver::verify_with_internal_solver(func)
}

/// Verifies a function explicitly using the Z3 solver via SMT-LIB2 pipe.
pub fn verify_function_with_z3(func: &FunctionDecl) -> Result<SmtResult, String> {
    let z3_path = find_z3_executable()
        .ok_or_else(|| "Z3 executable not found in PATH or standard install directories".to_string())?;
    let smt2_query = generate_smtlib2(func)?;
    
    match run_z3_query(&z3_path, &smt2_query)? {
        Z3Result::Unsat => Ok(SmtResult::Proved {
            solver: "Microsoft Z3 (SMT-LIB2)".into(),
        }),
        Z3Result::Sat(ce) => Ok(SmtResult::CounterExample(ce)),
        Z3Result::Unknown(r) => Ok(SmtResult::Unknown(r)),
    }
}

/// Verify all functions in a module, keyed by function name.
pub fn verify_module(module: &Module) -> HashMap<String, SmtResult> {
    module
        .functions
        .iter()
        .map(|function| (function.name.clone(), verify_function(function)))
        .collect()
}
