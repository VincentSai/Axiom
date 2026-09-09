//! Coordination of independently produced implementations for holes in an Axiom module.

use std::collections::{HashMap, HashSet};
use std::fmt;

use axiom_ast::{Expr, Module};
use axiom_db::{
    compute_envelope_hash, compute_expr_hash, compute_function_hash,
    AstDatabase, Candidate, CasError, CommitRecord, NodeHash, ProofLevel, TaskEnvelope,
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
    verified_module: Module,
    provisional_module: Module,
    tasks: Vec<HoleTask>,
    pending_holes: HashSet<String>,
    hole_evidence: HashMap<String, ProofLevel>,
    policy: PoolVerificationPolicy,
    db: AstDatabase,
    envelopes: HashMap<String, TaskEnvelope>,
    commits: Vec<CommitRecord>,
}

impl AgentTaskPool {
    /// Create a task pool from a parsed skeleton module with an in-memory database.
    pub fn new(skeleton_module: Module) -> Self {
        let db = AstDatabase::in_memory();
        Self::open_with_database(skeleton_module, db).expect("in-memory database pool initialization cannot fail")
    }

    /// Open or initialize a task pool backed by a shared AST database instance.
    pub fn open_with_database(skeleton_module: Module, mut db: AstDatabase) -> Result<Self, PoolError> {
        let mut seen_holes = HashSet::new();
        let tasks = extract_holes(&skeleton_module)
            .into_iter()
            .filter(|task| seen_holes.insert(task.hole_id.clone()))
            .collect::<Vec<_>>();
        let pending_holes = tasks.iter().map(|task| task.hole_id.clone()).collect();

        let base_root = if let Some(root) = db.current_root() {
            root
        } else {
            let mod_rec = db
                .insert_module(&skeleton_module)
                .map_err(|e| PoolError::CasFailure(e.to_string()))?;
            db.set_root(mod_rec.module_hash.clone())
                .map_err(|e| PoolError::CasFailure(e.to_string()))?;
            mod_rec.module_hash
        };

        let mut envelopes = HashMap::new();
        for task in &tasks {
            let func_hash = skeleton_module
                .functions
                .iter()
                .find(|f| f.name == task.target_function)
                .map(compute_function_hash)
                .unwrap_or_default();
            let env = TaskEnvelope {
                base_root_hash: Some(base_root.clone()),
                target_hole_id: task.hole_id.clone(),
                target_function: task.target_function.clone(),
                contract_hash: func_hash,
                dependency_hashes: Vec::new(),
            };
            envelopes.insert(task.hole_id.clone(), env);
        }

        Ok(Self {
            verified_module: skeleton_module.clone(),
            provisional_module: skeleton_module,
            tasks,
            pending_holes,
            hole_evidence: HashMap::new(),
            policy: PoolVerificationPolicy::default(),
            db,
            envelopes,
            commits: Vec::new(),
        })
    }

    /// Open or initialize a task pool backed by a persistent file database and skeleton source file.
    pub fn open_persisted(skeleton_path: &std::path::Path, db_path: &std::path::Path) -> Result<Self, PoolError> {
        let source = std::fs::read_to_string(skeleton_path).map_err(|e| PoolError::CasFailure(e.to_string()))?;
        let skeleton_module = parse_module(&source)?;
        let db = AstDatabase::open(db_path).map_err(|e| PoolError::CasFailure(e.to_string()))?;
        Self::open_with_database(skeleton_module, db)
    }

    /// Access the underlying immutable AST database.
    pub fn database(&self) -> &AstDatabase {
        &self.db
    }

    /// Access the underlying mutable AST database.
    pub fn database_mut(&mut self) -> &mut AstDatabase {
        &mut self.db
    }

    /// Access the verified module baseline.
    pub fn verified_module(&self) -> &Module {
        &self.verified_module
    }

    /// Access the provisional module (including draft / sampled solutions).
    pub fn provisional_module(&self) -> &Module {
        &self.provisional_module
    }

    /// Access the primary module (alias for verified_module).
    pub fn module(&self) -> &Module {
        &self.verified_module
    }

    /// Access the explicit hole evidence ledger.
    pub fn hole_evidence(&self) -> &HashMap<String, ProofLevel> {
        &self.hole_evidence
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
        let mut candidate_module = self.provisional_module.clone();
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
        let current_root = self.db.current_root();
        let envelope = self.envelopes.get(hole_id).cloned().unwrap_or_else(|| TaskEnvelope {
            base_root_hash: current_root.clone(),
            target_hole_id: hole_id.to_owned(),
            target_function: candidate_module.functions[affected_functions[0]].name.clone(),
            contract_hash,
            dependency_hashes: Vec::new(),
        });

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let candidate = Candidate {
            envelope_hash: compute_envelope_hash(&envelope),
            expression_source: solution_source.to_string(),
            expression_hash: cand_expr_hash.clone(),
            author_agent_id: agent_id.to_string(),
            timestamp: now,
        };

        // 1. 预先单点验证，支持向 Worker 报告反例以进行自愈
        let precheck_result = verify_candidate_precheck(&candidate_module, &affected_functions);
        match &precheck_result {
            SmtResult::CounterExample(_) | SmtResult::VacuousContract { .. } | SmtResult::Unknown(_) => {
                return Ok(precheck_result);
            }
            SmtResult::Skipped(_) => {
                let func = &candidate_module.functions[affected_functions[0]];
                if func.ensures.is_empty() {
                    if self.policy == PoolVerificationPolicy::Permissive {
                        self.provisional_module = candidate_module;
                        self.pending_holes.remove(hole_id);
                        self.hole_evidence.insert(hole_id.to_string(), ProofLevel::Sampled);
                        return Ok(precheck_result);
                    }
                    return Ok(precheck_result);
                }
            }
            SmtResult::Sampled { .. } => {
                if self.policy == PoolVerificationPolicy::Permissive {
                    // Permissive 模式：允许作为草稿更新 provisional_module，但不推进 verified_module
                    self.provisional_module = candidate_module;
                    self.pending_holes.remove(hole_id);
                    self.hole_evidence.insert(hole_id.to_string(), ProofLevel::Sampled);
                    return Ok(precheck_result);
                } else {
                    return Ok(precheck_result);
                }
            }
            SmtResult::Proved { .. } => {}
        }

        // 2. 核心提交：直接交由数据库内核验证并铸造收据！
        let expected_root = envelope.base_root_hash.clone();
        match self.db.try_commit_candidate(
            &expected_root,
            envelope.clone(),
            candidate.clone(),
            Some(&candidate_module),
        ) {
            Ok(commit_record) => {
                self.commits.push(commit_record);
                self.verified_module = candidate_module.clone();
                self.provisional_module = candidate_module;
                self.pending_holes.remove(hole_id);
                self.hole_evidence.insert(hole_id.to_string(), ProofLevel::Proved);
                let latest_root = self.db.current_root();
                for env in self.envelopes.values_mut() {
                    env.base_root_hash = latest_root.clone();
                }
                Ok(precheck_result)
            }
            Err(CasError::StaleRoot { current, .. }) => {
                if !self.pending_holes.contains(hole_id) {
                    return Err(PoolError::AlreadyResolved(hole_id.to_owned()));
                }

                // 核心决议 3：从数据库 CAS 物理加载最新真实 AST 实施 Fast-Forward 重基！
                let latest_root = current.clone().ok_or_else(|| {
                    PoolError::CasFailure("stale root returned None current root".to_string())
                })?;
                let mut rebased_module = self
                    .db
                    .get_module(&latest_root)
                    .map_err(|e| PoolError::CasFailure(e.to_string()))?
                    .ok_or_else(|| {
                        PoolError::CasFailure(format!(
                            "latest module for root `{latest_root}` not found in database"
                        ))
                    })?;

                // 关键：在被填洞替换前获取基线目标函数的契约哈希
                let base_func = rebased_module
                    .functions
                    .iter()
                    .find(|f| f.name == envelope.target_function)
                    .cloned()
                    .ok_or_else(|| PoolError::MissingHole(hole_id.to_owned()))?;
                let rebased_func_hash = compute_function_hash(&base_func);

                let mut rebased_affected = Vec::new();
                for (idx, function) in rebased_module.functions.iter_mut().enumerate() {
                    if replace_hole(&mut function.body, hole_id, &solution) {
                        rebased_affected.push(idx);
                    }
                }
                if rebased_affected.is_empty() {
                    return Err(PoolError::MissingHole(hole_id.to_owned()));
                }

                let rebased_envelope = TaskEnvelope {
                    base_root_hash: Some(latest_root.clone()),
                    target_hole_id: hole_id.to_owned(),
                    target_function: envelope.target_function.clone(),
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

                let recheck_result =
                    verify_candidate_precheck(&rebased_module, &rebased_affected);
                if !recheck_result.is_proved() {
                    return Ok(recheck_result);
                }

                let commit_record = self
                    .db
                    .try_commit_candidate(
                        &Some(latest_root),
                        rebased_envelope,
                        rebased_candidate,
                        Some(&rebased_module),
                    )
                    .map_err(|e| PoolError::CasFailure(e.to_string()))?;

                self.commits.push(commit_record);
                self.verified_module = rebased_module.clone();
                self.provisional_module = rebased_module;
                self.pending_holes.remove(hole_id);
                self.hole_evidence.insert(hole_id.to_string(), ProofLevel::Proved);
                let current_root = self.db.current_root();
                for env in self.envelopes.values_mut() {
                    env.base_root_hash = current_root.clone();
                }
                Ok(recheck_result)
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
    /// 严禁妥协强门禁：
    /// 1. 所有已分发的洞必须已被解决；
    /// 2. 深入 AST 扫描，若有残留的 Expr::Hole，拒绝导出；
    /// 3. 证据账本强门禁：所有已分发洞 100% 具备 ProofLevel::Proved 证据；
    ///    若有任何洞仅为 Sampled 或 Unverified，坚决拒绝，返回 PoolError::InsufficientEvidence！
    pub fn assemble_verified_module(&self) -> Result<Module, PoolError> {
        if !self.is_complete() {
            return Err(PoolError::Incomplete(self.pending_hole_ids()));
        }
        for task in &self.tasks {
            let evidence = self.hole_evidence.get(&task.hole_id);
            if evidence != Some(&ProofLevel::Proved) {
                return Err(PoolError::InsufficientEvidence {
                    hole_id: task.hole_id.clone(),
                    level: evidence.copied().unwrap_or(ProofLevel::Unverified),
                });
            }
        }
        if ast_contains_any_hole(&self.verified_module) {
            return Err(PoolError::Incomplete(vec!["unresolved_ast_hole".to_string()]));
        }
        Ok(self.verified_module.clone())
    }

    /// Assemble a provisional module draft (for inspection, review, or debugging).
    /// May contain unresolved holes or provisional implementations.
    pub fn assemble_provisional_module(&self) -> Module {
        self.provisional_module.clone()
    }
}

/// 预检给定模块中被填洞替换影响的所有函数的 SMT 证明结果
pub fn verify_candidate_precheck(
    module: &Module,
    affected_functions: &[usize],
) -> SmtResult {
    let mut final_result = SmtResult::Sampled { points_checked: 0 };
    for &function_index in affected_functions {
        let func = &module.functions[function_index];
        let result = verify_function_in_module(func, module);
        match &result {
            SmtResult::CounterExample { .. }
            | SmtResult::VacuousContract { .. }
            | SmtResult::Unknown(_) => {
                return result;
            }
            SmtResult::Skipped(_) => {
                if !func.ensures.is_empty() {
                    return result;
                }
                final_result = result;
            }
            SmtResult::Sampled { .. } => {
                final_result = result;
            }
            SmtResult::Proved { .. } => {
                final_result = result;
            }
        }
    }
    final_result
}

/// Errors that prevent a worker submission or final module assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    Parse(ParseError),
    UnknownHole(String),
    AlreadyResolved(String),
    MissingHole(String),
    Incomplete(Vec<String>),
    InsufficientEvidence {
        hole_id: String,
        level: ProofLevel,
    },
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
            Self::InsufficientEvidence { hole_id, level } => write!(
                formatter,
                "hole `{hole_id}` does not meet formal verification bar: evidence level is `{level:?}`, required `Proved`"
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
            | Self::InsufficientEvidence { .. }
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
