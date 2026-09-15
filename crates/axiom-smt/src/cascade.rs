//! Inter-procedural cascading formal verification across dependency closures.
//!
//! When a symbol or function implementation changes, this module computes the
//! transitive downstream closure and verifies all affected contracted functions,
//! ensuring that local proofs do not silently break global contracts.

use std::collections::HashMap;

use axiom_ast::analysis::CallGraph;
use axiom_ast::Module;
use serde::{Deserialize, Serialize};

use crate::{verify_function_in_module, SmtResult};

/// 级联验证报告
#[derive(Debug, Clone, PartialEq)]
pub struct CascadeVerificationReport {
    /// 触发本次重验证的变更符号列表
    pub triggers: Vec<String>,
    /// 经过闭包展开后全部受影响并实际执行 SMT 检验的函数列表
    pub verified_functions: Vec<String>,
    /// 每个受检契约函数的 SMT 证明结果
    pub results: HashMap<String, SmtResult>,
    /// 级联失败/反例列表（若非空则证明级联受阻）
    pub failures: Vec<CascadeFailure>,
}

impl CascadeVerificationReport {
    /// 检查闭包内所有受影响契约函数是否全部证明通过（All Proved）
    pub fn is_all_proved(&self) -> bool {
        self.failures.is_empty() && self.results.values().all(|r| r.is_proved())
    }
}

/// 级联契约破坏诊断信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CascadeFailure {
    /// 验证失败的下游契约函数
    pub function_name: String,
    /// 导致其失败的上游触发符号
    pub trigger_symbol: String,
    /// 失败原因摘要
    pub reason: String,
}

/// 针对变更符号集合，计算下游传递影响闭包并执行级联 SMT 定理证明
pub fn verify_dependent_closure(
    module: &Module,
    changed_symbols: &[&str],
) -> CascadeVerificationReport {
    let graph = CallGraph::from_module(module);
    let closure = graph.transitive_dependents(changed_symbols);

    let mut report = CascadeVerificationReport {
        triggers: changed_symbols.iter().map(|s| s.to_string()).collect(),
        verified_functions: Vec::new(),
        results: HashMap::new(),
        failures: Vec::new(),
    };

    let trigger_sym = changed_symbols.first().copied().unwrap_or("unknown");

    for func in &module.functions {
        if closure.contains(&func.name) && func.has_contracts() {
            report.verified_functions.push(func.name.clone());
            let result = verify_function_in_module(func, module);
            if !result.is_proved() {
                let reason = match &result {
                    SmtResult::CounterExample(ce) => {
                        format!("SMT counterexample found: {ce:?}")
                    }
                    SmtResult::VacuousContract { reason } => {
                        format!("vacuous contract: {reason}")
                    }
                    SmtResult::Unknown(r) => format!("solver unknown: {r}"),
                    SmtResult::Skipped(r) => format!("verification skipped: {r}"),
                    SmtResult::Sampled { points_checked } => {
                        format!("sampled {points_checked} points but not strictly proved")
                    }
                    SmtResult::Proved { .. } => unreachable!(),
                };

                report.failures.push(CascadeFailure {
                    function_name: func.name.clone(),
                    trigger_symbol: trigger_sym.to_string(),
                    reason,
                });
            }
            report.results.insert(func.name.clone(), result);
        }
    }

    report
}
