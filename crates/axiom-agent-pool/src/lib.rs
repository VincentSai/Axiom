//! Coordination of independently produced implementations for holes in an Axiom module.

use std::collections::{HashMap, HashSet};
use std::fmt;

use axiom_ast::{Expr, Module};
use axiom_db::{
    compute_candidate_hash, compute_envelope_hash, compute_expr_hash, compute_function_hash,
    AstDatabase, Candidate, CasError, CommitRecord, NodeHash, ProofLevel, TaskEnvelope,
    VerificationReceipt,
};
use axiom_hole_extractor::extract_holes;
pub use axiom_hole_extractor::HoleTask;
use axiom_parser::{parse_expr, parse_module, ParseError};
use axiom_smt::verify_function_in_module;
pub use axiom_smt::SmtResult;

/// Verification policy for accepting hole candidate solutions into the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PoolVerificationPolicy {
    /// Strict formal mode: contracted functions MUST be formally Proved by SMT solver.
    #[default]
    StrictProved,
    /// Permissive mode: allows discrete grid sampling (Sampled) when solver is unavailable.
    Permissive,
}

/// A pool of hole tasks backed by a module whose accepted solutions are SMT-verified and committed via CAS.
#[derive(Debug, Clone)]
pub struct AgentTaskPool {
    module: Module,
    tasks: Vec<HoleTask>,
    pending_holes: HashSet<String>,
    policy: PoolVerificationPolicy,
    db: AstDatabase,
    envelopes: HashMap<String, TaskEnvelope>,
    commits: Vec<CommitRecord>,
}

impl AgentTaskPool {
    /// Create a task pool from a parsed skeleton module.
    pub fn new(skeleton_module: Module) -> Self {
        let mut seen_holes = HashSet::new();
        let tasks = extract_holes(&skeleton_module)
            .into_iter()
            .filter(|task| seen_holes.insert(task.hole_id.clone()))
            .collect::<Vec<_>>();
        let pending_holes = tasks.iter().map(|task| task.hole_id.clone()).collect();

        let mut db = AstDatabase::in_memory();
        let mod_rec = db
            .insert_module(&skeleton_module)
            .expect("in-memory database module insert cannot fail");
        db.set_root(mod_rec.module_hash.clone())
            .expect("in-memory database set_root cannot fail");

        let mut envelopes = HashMap::new();
        for task in &tasks {
            let func_hash = skeleton_module
                .functions
                .iter()
                .find(|f| f.name == task.target_function)
                .map(compute_function_hash)
                .unwrap_or_default();
            let env = TaskEnvelope {
                base_root_hash: Some(mod_rec.module_hash.clone()),
                target_hole_id: task.hole_id.clone(),
                target_function: task.target_function.clone(),
                contract_hash: func_hash,
                dependency_hashes: Vec::new(),
            };
            envelopes.insert(task.hole_id.clone(), env);
        }

        Self {
            module: skeleton_module,
            tasks,
            pending_holes,
            policy: PoolVerificationPolicy::default(),
            db,
            envelopes,
            commits: Vec::new(),
        }
    }

    /// Access the underlying immutable AST database.
    pub fn database(&self) -> &AstDatabase {
        &self.db
    }

    /// Get current root module hash reference.
    pub fn current_root(&self) -> Option<NodeHash> {
        self.db.current_root()
    }

    /// Access verified commit audit history.
    pub fn commits(&self) -> &[CommitRecord] {
        &self.commits
    }

    /// Access task envelope for a specific hole.
    pub fn envelope_for_hole(&self, hole_id: &str) -> Option<&TaskEnvelope> {
        self.envelopes.get(hole_id)
    }

    /// Set verification policy for this pool.
    pub fn with_policy(mut self, policy: PoolVerificationPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Set verification policy mutably.
    pub fn set_policy(&mut self, policy: PoolVerificationPolicy) {
        self.policy = policy;
    }

    /// Get current verification policy for this pool.
    pub fn policy(&self) -> PoolVerificationPolicy {
        self.policy
    }

    /// Parse an Axiom skeleton module and create its task pool.
    pub fn from_source(source: &str) -> Result<Self, PoolError> {
        Ok(Self::new(parse_module(source)?))
    }

    /// Return the tasks that still need verified implementations, in source order.
    pub fn dispatch_tasks(&self) -> Vec<HoleTask> {
        self.tasks
            .iter()
            .filter(|task| self.pending_holes.contains(&task.hole_id))
            .cloned()
            .collect()
    }

    /// Return the pending hole identifiers in source order.
    pub fn pending_hole_ids(&self) -> Vec<String> {
        self.dispatch_tasks()
            .into_iter()
            .map(|task| task.hole_id)
            .collect()
    }

    /// Parse, verify, and conditionally commit a candidate hole implementation via CAS.
    pub fn submit_solution(
        &mut self,
        hole_id: &str,
        solution_source: &str,
    ) -> Result<SmtResult, PoolError> {
        self.submit_solution_with_agent(hole_id, solution_source, "axiom-worker")
    }

    /// Parse, verify, and conditionally commit a candidate hole implementation with an explicit agent ID.
    pub fn submit_solution_with_agent(
        &mut self,
        hole_id: &str,
        solution_source: &str,
        agent_id: &str,
    ) -> Result<SmtResult, PoolError> {
        if !self.tasks.iter().any(|task| task.hole_id == hole_id) {
            return Err(PoolError::UnknownHole(hole_id.to_owned()));
        }
        if !self.pending_holes.contains(hole_id) {
            return Err(PoolError::AlreadyResolved(hole_id.to_owned()));
        }

        let solution = parse_expr(solution_source)?;
        let mut candidate_module = self.module.clone();
        let mut affected_functions = Vec::new();

        for (index, function) in candidate_module.functions.iter_mut().enumerate() {
            if replace_hole(&mut function.body, hole_id, &solution) {
                affected_functions.push(index);
            }
        }

        if affected_functions.is_empty() {
            return Err(PoolError::MissingHole(hole_id.to_owned()));
        }

        let cand_expr_hash = compute_expr_hash(&solution);
        let contract_hash = compute_function_hash(&candidate_module.functions[affected_functions[0]]);
        let envelope = self.envelopes.get(hole_id).cloned().unwrap_or_else(|| TaskEnvelope {
            base_root_hash: self.db.current_root(),
            target_hole_id: hole_id.to_owned(),
            target_function: candidate_module.functions[affected_functions[0]].name.clone(),
            contract_hash,
            dependency_hashes: Vec::new(),
        });

        let candidate = Candidate {
            envelope_hash: compute_envelope_hash(&envelope),
            expression_source: solution_source.to_string(),
            expression_hash: cand_expr_hash.clone(),
            author_agent_id: agent_id.to_string(),
            timestamp: 0,
        };

        // 1. 首次验证通过单点函数执行
        let (final_result, receipt) = match verify_candidate_and_mint_receipt(
            &candidate_module,
            &affected_functions,
            &candidate,
            self.policy,
        ) {
            Ok(pair) => pair,
            Err(failure_result) => return Ok(failure_result),
        };

        let expected_root = envelope.base_root_hash.clone();
        match self.db.try_commit_candidate(
            &expected_root,
            envelope.clone(),
            candidate.clone(),
            receipt.clone(),
            &candidate_module,
        ) {
            Ok(commit_record) => {
                self.commits.push(commit_record);
                self.module = candidate_module;
                self.pending_holes.remove(hole_id);
                let latest_root = self.db.current_root();
                for env in self.envelopes.values_mut() {
                    env.base_root_hash = latest_root.clone();
                }
                Ok(final_result)
            }
            Err(CasError::StaleRoot { current, .. }) => {
                // Stale Root 冲突处理：并发冲突检测成功
                if !self.pending_holes.contains(hole_id) {
                    return Err(PoolError::AlreadyResolved(hole_id.to_owned()));
                }

                let mut rebased_module = self.module.clone();
                let mut rebased_affected = Vec::new();
                for (idx, function) in rebased_module.functions.iter_mut().enumerate() {
                    if replace_hole(&mut function.body, hole_id, &solution) {
                        rebased_affected.push(idx);
                    }
                }
                if rebased_affected.is_empty() {
                    return Err(PoolError::MissingHole(hole_id.to_owned()));
                }

                let rebased_func_hash = compute_function_hash(&rebased_module.functions[rebased_affected[0]]);
                let rebased_envelope = TaskEnvelope {
                    base_root_hash: current.clone(),
                    target_hole_id: hole_id.to_owned(),
                    target_function: rebased_module.functions[rebased_affected[0]].name.clone(),
                    contract_hash: rebased_func_hash,
                    dependency_hashes: Vec::new(),
                };
                let rebased_candidate = Candidate {
                    envelope_hash: compute_envelope_hash(&rebased_envelope),
                    expression_source: solution_source.to_string(),
                    expression_hash: cand_expr_hash,
                    author_agent_id: agent_id.to_string(),
                    timestamp: candidate.timestamp + 1,
                };

                // 2. 关键：Fast-Forward 重验 100% 走同一个纯函数，铸造全新收据，严禁继承旧收据！
                let (recheck_result, rebased_receipt) = match verify_candidate_and_mint_receipt(
                    &rebased_module,
                    &rebased_affected,
                    &rebased_candidate,
                    self.policy,
                ) {
                    Ok(pair) => pair,
                    Err(failure_result) => return Ok(failure_result),
                };

                let commit_record = self
                    .db
                    .try_commit_candidate(
                        &current,
                        rebased_envelope,
                        rebased_candidate,
                        rebased_receipt,
                        &rebased_module,
                    )
                    .map_err(|e| PoolError::CasFailure(e.to_string()))?;

                self.commits.push(commit_record);
                self.module = rebased_module;
                self.pending_holes.remove(hole_id);
                let latest_root = self.db.current_root();
                for env in self.envelopes.values_mut() {
                    env.base_root_hash = latest_root.clone();
                }
                Ok(recheck_result)
            }
            Err(CasError::UnverifiedCandidate(_)) => {
                if self.policy == PoolVerificationPolicy::Permissive {
                    let _ = self.db.record_receipt(receipt);
                    self.module = candidate_module;
                    self.pending_holes.remove(hole_id);
                    Ok(final_result)
                } else {
                    Ok(final_result)
                }
            }
            Err(other) => Err(PoolError::CasFailure(other.to_string())),
        }
    }

    /// Return whether every dispatched hole has an accepted implementation.
    pub fn is_complete(&self) -> bool {
        self.pending_holes.is_empty()
    }

    /// Assemble a clone of the verified, hole-free module.
    ///
    /// 严禁妥协：
    /// 1. 所有已分发的洞必须已被解决；
    /// 2. 深入 AST 扫描，若有残留的 Expr::Hole，拒绝导出；
    pub fn assemble_verified_module(&self) -> Result<Module, PoolError> {
        if !self.is_complete() {
            return Err(PoolError::Incomplete(self.pending_hole_ids()));
        }
        if ast_contains_any_hole(&self.module) {
            return Err(PoolError::Incomplete(vec!["unresolved_ast_hole".to_string()]));
        }
        Ok(self.module.clone())
    }

    /// Assemble a provisional module draft (for inspection, review, or debugging).
    /// May contain unresolved holes or provisional implementations.
    pub fn assemble_provisional_module(&self) -> Module {
        self.module.clone()
    }
}

/// 单点验证与收据铸造纯函数：
/// 对给定模块中被填洞替换影响的所有函数执行 SMT 定理证明，
/// 严格保证无论首次提交还是并发冲突后的 Fast-Forward 重验，100% 走完全一致的逻辑，
/// 杜绝分支逻辑漂移，坚决禁止继承旧收据字段。
pub fn verify_candidate_and_mint_receipt(
    module: &Module,
    affected_functions: &[usize],
    candidate: &Candidate,
    policy: PoolVerificationPolicy,
) -> Result<(SmtResult, VerificationReceipt), SmtResult> {
    let mut final_result = SmtResult::Sampled { points_checked: 0 };

    for &function_index in affected_functions {
        let func = &module.functions[function_index];
        let result = verify_function_in_module(func, module);
        match &result {
            SmtResult::CounterExample { .. }
            | SmtResult::VacuousContract { .. }
            | SmtResult::Unknown(_) => {
                return Err(result);
            }
            SmtResult::Skipped(_) => {
                if !func.ensures.is_empty() {
                    return Err(result);
                }
                final_result = result;
            }
            SmtResult::Sampled { .. } => {
                if policy == PoolVerificationPolicy::StrictProved && !func.ensures.is_empty() {
                    return Err(result);
                }
                final_result = result;
            }
            SmtResult::Proved { .. } => {
                final_result = result;
            }
        }
    }

    let cand_hash = compute_candidate_hash(candidate);

    let (proof_level, solver, solver_version, obligations_checked) = match &final_result {
        SmtResult::Proved {
            solver,
            version,
            obligations_checked,
        } => (
            ProofLevel::Proved,
            solver.clone(),
            version.clone(),
            *obligations_checked,
        ),
        SmtResult::Sampled { points_checked } => (
            ProofLevel::Sampled,
            "discrete-grid-evaluator".to_string(),
            "0.1.0".to_string(),
            *points_checked,
        ),
        _ => (ProofLevel::Unverified, "none".to_string(), "0".to_string(), 0),
    };

    let receipt = VerificationReceipt {
        candidate_hash: cand_hash.clone(),
        proof_level,
        solver,
        solver_version,
        obligations_checked,
        proof_obligation_hash: cand_hash,
        verified_at: candidate.timestamp,
    };

    Ok((final_result, receipt))
}

/// Errors that prevent a worker submission or final module assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    Parse(ParseError),
    UnknownHole(String),
    AlreadyResolved(String),
    MissingHole(String),
    Incomplete(Vec<String>),
    CasFailure(String),
}

impl fmt::Display for PoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "could not parse Axiom source: {error}"),
            Self::UnknownHole(hole_id) => write!(formatter, "unknown hole `{hole_id}`"),
            Self::AlreadyResolved(hole_id) => {
                write!(
                    formatter,
                    "hole `{hole_id}` already has a verified solution"
                )
            }
            Self::MissingHole(hole_id) => write!(
                formatter,
                "pending hole `{hole_id}` is missing from the working module"
            ),
            Self::Incomplete(hole_ids) => write!(
                formatter,
                "module still has pending holes: {}",
                hole_ids.join(", ")
            ),
            Self::CasFailure(msg) => write!(formatter, "CAS commit failed: {msg}"),
        }
    }
}

impl std::error::Error for PoolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::UnknownHole(_)
            | Self::AlreadyResolved(_)
            | Self::MissingHole(_)
            | Self::Incomplete(_)
            | Self::CasFailure(_) => None,
        }
    }
}

impl From<ParseError> for PoolError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

fn replace_hole(expression: &mut Expr, hole_id: &str, solution: &Expr) -> bool {
    match expression {
        Expr::Hole(current_hole_id) if current_hole_id == hole_id => {
            *expression = solution.clone();
            true
        }
        Expr::BinaryOp { left, right, .. } => {
            replace_hole(left, hole_id, solution) | replace_hole(right, hole_id, solution)
        }
        Expr::UnaryOp { operand, .. } | Expr::Let { value: operand, .. } => {
            replace_hole(operand, hole_id, solution)
        }
        Expr::Block { statements, tail } => {
            let statements_changed = statements.iter_mut().fold(false, |changed, statement| {
                replace_hole(statement, hole_id, solution) | changed
            });
            let tail_changed = tail
                .as_mut()
                .is_some_and(|tail| replace_hole(tail, hole_id, solution));
            statements_changed | tail_changed
        }
        Expr::Call { callee, args } => {
            let callee_changed = replace_hole(callee, hole_id, solution);
            let args_changed = args.iter_mut().fold(false, |changed, argument| {
                replace_hole(argument, hole_id, solution) | changed
            });
            callee_changed | args_changed
        }
        Expr::FieldAccess { target, .. } => replace_hole(target, hole_id, solution),
        Expr::RecordInit { fields, .. } => fields.iter_mut().fold(false, |changed, (_, value)| {
            replace_hole(value, hole_id, solution) | changed
        }),
        Expr::RecordUpdate { base, updates } => {
            let base_changed = replace_hole(base, hole_id, solution);
            let updates_changed = updates.iter_mut().fold(false, |changed, (_, value)| {
                replace_hole(value, hole_id, solution) | changed
            });
            base_changed | updates_changed
        }
        Expr::Match { target, arms } => {
            let target_changed = replace_hole(target, hole_id, solution);
            let arms_changed = arms.iter_mut().fold(false, |changed, arm| {
                replace_hole(&mut arm.body, hole_id, solution) | changed
            });
            target_changed | arms_changed
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) | Expr::Raw(_) => false,
    }
}

fn ast_contains_any_hole_in_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Hole(_) => true,
        Expr::BinaryOp { left, right, .. } => {
            ast_contains_any_hole_in_expr(left) || ast_contains_any_hole_in_expr(right)
        }
        Expr::UnaryOp { operand, .. } | Expr::Let { value: operand, .. } => {
            ast_contains_any_hole_in_expr(operand)
        }
        Expr::Block { statements, tail } => {
            statements.iter().any(ast_contains_any_hole_in_expr)
                || tail.as_deref().map_or(false, ast_contains_any_hole_in_expr)
        }
        Expr::Call { callee, args } => {
            ast_contains_any_hole_in_expr(callee) || args.iter().any(ast_contains_any_hole_in_expr)
        }
        Expr::FieldAccess { target, .. } => ast_contains_any_hole_in_expr(target),
        Expr::RecordInit { fields, .. } => fields.iter().any(|(_, v)| ast_contains_any_hole_in_expr(v)),
        Expr::RecordUpdate { base, updates } => {
            ast_contains_any_hole_in_expr(base)
                || updates.iter().any(|(_, v)| ast_contains_any_hole_in_expr(v))
        }
        Expr::Match { target, arms } => {
            ast_contains_any_hole_in_expr(target)
                || arms.iter().any(|arm| ast_contains_any_hole_in_expr(&arm.body))
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::Raw(_) => false,
    }
}

pub fn ast_contains_any_hole(module: &Module) -> bool {
    module.functions.iter().any(|f| ast_contains_any_hole_in_expr(&f.body))
}
