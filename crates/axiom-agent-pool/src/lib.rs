//! Coordination of independently produced implementations for holes in an Axiom module.

use std::collections::{HashMap, HashSet};
use std::fmt;

use axiom_ast::{Expr, Module};
use axiom_db::{
    compute_contract_hash, compute_envelope_hash, compute_expr_hash, compute_function_hash,
    AstDatabase, Candidate, CasError, CommitRecord, DraftCasExpectation, NodeHash, ProofLevel,
    TaskEnvelope,
};
use axiom_hole_extractor::extract_holes;
pub use axiom_hole_extractor::HoleTask;
use axiom_parser::{parse_expr, parse_module, ParseError};
use axiom_smt::verify_function_in_module;
pub use axiom_smt::SmtResult;
use serde::{Deserialize, Serialize};

use axiom_ast::merge::{merge_modules_3way, AstConflictReport};
use axiom_smt::cascade::verify_dependent_closure;

pub mod auto_heal;

/// Detailed status tag of submitting a hole solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubmitStatus {
    Accepted,
    Provisional,
    Rejected,
    Unknown,
    Skipped,
}

impl fmt::Display for SubmitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accepted => write!(f, "ACCEPTED"),
            Self::Provisional => write!(f, "PROVISIONAL"),
            Self::Rejected => write!(f, "REJECTED"),
            Self::Unknown => write!(f, "UNKNOWN"),
            Self::Skipped => write!(f, "SKIPPED"),
        }
    }
}

/// Detailed outcome of submitting a hole solution, including SMT verification and CAS events.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmitOutcome {
    pub smt_result: SmtResult,
    pub cas_events: Vec<CasEvent>,
    pub upgraded_to_verified: bool,
    pub conflict_report: Option<AstConflictReport>,
}

/// Serializable Data Transfer Object for SubmitOutcome, decoupling clients/CLI from internal domain types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmitOutcomeDto {
    pub hole: String,
    pub status: SubmitStatus,
    pub proved: bool,
    pub upgraded_to_verified: bool,
    pub solver: Option<String>,
    pub solver_version: Option<String>,
    pub obligations_checked: Option<usize>,
    pub cas_events: Vec<CasEvent>,
    pub counter_example: Option<String>,
    pub reason: Option<String>,
    pub conflict_report: Option<AstConflictReport>,
}

impl SubmitOutcome {
    pub fn to_dto(&self, hole_id: &str) -> SubmitOutcomeDto {
        let (status, proved, solver, version, obligations_checked, counter_example, reason) =
            match &self.smt_result {
                SmtResult::Proved {
                    solver,
                    version,
                    obligations_checked,
                    ..
                } => (
                    SubmitStatus::Accepted,
                    true,
                    Some(solver.clone()),
                    Some(version.clone()),
                    Some(*obligations_checked),
                    None,
                    None,
                ),
                SmtResult::CounterExample(ce) => (
                    SubmitStatus::Rejected,
                    false,
                    None,
                    None,
                    None,
                    Some(format!("{ce:?}")),
                    Some("SMT counterexample found violating contract".to_string()),
                ),
                SmtResult::Sampled { points_checked } => (
                    SubmitStatus::Provisional,
                    false,
                    None,
                    None,
                    Some(*points_checked),
                    None,
                    Some("candidate sampled without strict proof".to_string()),
                ),
                SmtResult::VacuousContract { reason } => (
                    SubmitStatus::Rejected,
                    false,
                    None,
                    None,
                    None,
                    None,
                    Some(format!("vacuous contract: {reason}")),
                ),
                SmtResult::Unknown(reason) => (
                    SubmitStatus::Unknown,
                    false,
                    None,
                    None,
                    None,
                    None,
                    Some(format!("solver unknown: {reason}")),
                ),
                SmtResult::Skipped(reason) => (
                    SubmitStatus::Skipped,
                    false,
                    None,
                    None,
                    None,
                    None,
                    Some(format!("verification skipped: {reason}")),
                ),
            };

        SubmitOutcomeDto {
            hole: hole_id.to_string(),
            status,
            proved,
            upgraded_to_verified: self.upgraded_to_verified,
            solver,
            solver_version: version,
            obligations_checked,
            cas_events: self.cas_events.clone(),
            counter_example,
            reason,
            conflict_report: self.conflict_report.clone(),
        }
    }
}

/// Structured CAS event emitted during candidate submission and concurrent resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CasEvent {
    StaleRootDetected {
        expected: Option<NodeHash>,
        current: Option<NodeHash>,
    },
    RebaseSucceeded {
        rebased_root: NodeHash,
        attempts: usize,
    },
    Ast3WayRebaseSucceeded {
        rebased_root: NodeHash,
        cascade_verified_count: usize,
    },
    AstMergeConflict {
        conflict_count: usize,
        summary: String,
    },
    CascadeVerificationFailed {
        failed_functions: Vec<String>,
        reasons: Vec<String>,
    },
}

/// Detailed outcome of reverifying an unproved hole task against the current Verification Profile.
#[derive(Debug, Clone, PartialEq)]
pub enum ReverifyOutcome {
    Committed {
        hole_id: String,
        receipt_hash: NodeHash,
    },
    Unproved {
        hole_id: String,
        smt_result: SmtResult,
    },
    Failed {
        hole_id: String,
        reason: String,
    },
    Skipped {
        hole_id: String,
        reason: String,
    },
}

impl ReverifyOutcome {
    pub fn hole_id(&self) -> &str {
        match self {
            Self::Committed { hole_id, .. }
            | Self::Unproved { hole_id, .. }
            | Self::Failed { hole_id, .. }
            | Self::Skipped { hole_id, .. } => hole_id,
        }
    }

    pub fn is_committed(&self) -> bool {
        matches!(self, Self::Committed { .. })
    }
}

/// Lifecycle status of a hole task in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoleStatus {
    /// No solution accepted yet.
    Unresolved,
    /// Has draft/sampled solution in provisional module; open for upgrade.
    Provisional,
    /// Mathematically verified with SMT proof, permanently committed to CAS database.
    Verified,
}

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
    provisional_drafts: HashMap<String, Expr>,
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
        Self::open_with_database(skeleton_module, db)
            .expect("in-memory database pool initialization cannot fail")
    }

    /// Open or initialize a task pool backed by a shared AST database instance.
    ///
    /// Single Source of Truth (SSOT):
    /// - If the database already contains a `root_ref`, the authoritative module is loaded
    ///   directly from CAS storage, completely replacing the passed skeleton text.
    /// - Hole evidence ledger is rehydrated from stored commits and receipts.
    /// - Pending holes are derived dynamically by extracting unresolved holes from the authoritative AST.
    pub fn open_with_database(
        skeleton_module: Module,
        mut db: AstDatabase,
    ) -> Result<Self, PoolError> {
        let mut seen_holes = HashSet::new();
        let tasks = extract_holes(&skeleton_module)
            .into_iter()
            .filter(|task| seen_holes.insert(task.hole_id.clone()))
            .collect::<Vec<_>>();

        // 1. 确定权威基线模块与 Root
        let (base_root, authoritative_module) = if let Some(root) = db.current_root() {
            let loaded_mod = db
                .get_module(&root)
                .map_err(|e| PoolError::CasFailure(e.to_string()))?
                .unwrap_or_else(|| skeleton_module.clone());
            (root, loaded_mod)
        } else {
            let mod_rec = db
                .insert_module(&skeleton_module)
                .map_err(|e| PoolError::CasFailure(e.to_string()))?;
            db.initialize_root(mod_rec.module_hash.clone())
                .map_err(|e| PoolError::CasFailure(e.to_string()))?;
            (mod_rec.module_hash, skeleton_module.clone())
        };

        // 2. 从权威模块动态派生当前真正剩余的 pending_holes
        let remaining_tasks = extract_holes(&authoritative_module);
        let mut pending_holes: HashSet<String> =
            remaining_tasks.into_iter().map(|t| t.hole_id).collect();

        // 3. [Standards 1, P1] 由 AstDatabase::mainline_commits() 提供经过完整性校验的拓扑有序序列
        let ordered_commits_refs = db
            .mainline_commits()
            .map_err(|e| PoolError::CasFailure(e.to_string()))?;
        let ordered_commits: Vec<CommitRecord> =
            ordered_commits_refs.into_iter().cloned().collect();

        // 如果存在未处于主干链上的孤立提交（例如分支尝试），按时间戳稳定排序后补充在其后，确保历史完整性
        let main_line_commits: HashSet<NodeHash> = ordered_commits
            .iter()
            .map(|c| c.commit_hash.clone())
            .collect();
        let mut side_commits: Vec<CommitRecord> = db
            .commits()
            .values()
            .filter(|c| !main_line_commits.contains(&c.commit_hash))
            .cloned()
            .collect();
        side_commits.sort_by(|a, b| {
            a.timestamp()
                .cmp(&b.timestamp())
                .then_with(|| a.commit_hash.cmp(&b.commit_hash))
        });

        let mut commits = ordered_commits;
        let replay_sequence = commits.clone();
        commits.extend(side_commits);

        let mut hole_evidence = HashMap::new();
        let current_profile_hash = axiom_db::current_verification_profile_hash();
        for commit in &replay_sequence {
            let receipt = db
                .get_receipt(commit.receipt_hash())
                .map_err(|e| PoolError::CasFailure(e.to_string()))?
                .ok_or_else(|| {
                    PoolError::CasFailure(format!(
                        "receipt {} not found in database",
                        commit.receipt_hash()
                    ))
                })?;
            let env = db
                .get_envelope(commit.envelope_hash())
                .map_err(|e| PoolError::CasFailure(e.to_string()))?
                .ok_or_else(|| {
                    PoolError::CasFailure(format!(
                        "envelope {} not found in database",
                        commit.envelope_hash()
                    ))
                })?;

            // [Spec 2 & Round 20 P1 Spec 1] 校验证明收据在当前规则下是否属于有效证明：
            // 必须满足：Profile Hash 一致、具备合法 64 位 solver_sha256、且 solver_version 已知。
            // 任何不满足项（包括从旧版本迁移来的无哈希收据）均视为旧收据失效，降级为 Unverified 触发重新验证。
            if receipt.is_currently_valid_proved(&current_profile_hash) {
                hole_evidence.insert(env.target_hole_id.clone(), ProofLevel::Proved);
            } else {
                hole_evidence.insert(env.target_hole_id.clone(), ProofLevel::Unverified);
            }
        }

        // 3.5 [Spec 4] 校验与收据重验闭环：
        // 凡是在初始任务 tasks 中定义、但在证据账本中未达到当前 Profile Proved 状态的洞
        // （例如规则升级后旧收据失效降级为 Unverified），必须强制重新纳入 pending_holes！
        for task in &tasks {
            if hole_evidence.get(&task.hole_id) != Some(&ProofLevel::Proved) {
                pending_holes.insert(task.hole_id.clone());
            }
        }

        // 4. 从持久化数据库恢复草稿（Provisional Drafts）
        let mut provisional_drafts = HashMap::new();
        for (hole_id, draft_rec) in db.drafts() {
            if pending_holes.contains(hole_id) {
                if let Ok(expr) = parse_expr(&draft_rec.code) {
                    provisional_drafts.insert(hole_id.clone(), expr);
                    hole_evidence.insert(hole_id.clone(), ProofLevel::Sampled);
                }
            }
        }

        // 5. 构造任务信封与静态提取真实依赖闭包（使用规范级 compute_contract_hash）
        let mut envelopes = HashMap::new();
        for task in &tasks {
            let func_opt = authoritative_module
                .functions
                .iter()
                .find(|f| f.name == task.target_function);
            let contract_hash = func_opt.map(compute_contract_hash).unwrap_or_default();
            let dep_hashes = if let Some(f) = func_opt {
                extract_function_dependencies(f, &authoritative_module)
            } else {
                Vec::new()
            };

            let env = TaskEnvelope {
                base_root_hash: Some(base_root.clone()),
                target_hole_id: task.hole_id.clone(),
                target_function: task.target_function.clone(),
                contract_hash,
                dependency_hashes: dep_hashes,
            };
            envelopes.insert(task.hole_id.clone(), env);
        }

        let mut pool = Self {
            verified_module: authoritative_module.clone(),
            provisional_module: authoritative_module,
            provisional_drafts,
            tasks,
            pending_holes,
            hole_evidence,
            policy: PoolVerificationPolicy::StrictProved,
            db,
            envelopes,
            commits,
        };
        pool.rebuild_provisional_module();
        Ok(pool)
    }

    /// Open or initialize a task pool backed by a persistent file database and skeleton source file.
    pub fn open_persisted(
        skeleton_path: &std::path::Path,
        db_path: &std::path::Path,
    ) -> Result<Self, PoolError> {
        let source = std::fs::read_to_string(skeleton_path)
            .map_err(|e| PoolError::CasFailure(e.to_string()))?;
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

    /// Return all hole tasks originally defined in the skeleton module.
    pub fn all_tasks(&self) -> &[HoleTask] {
        &self.tasks
    }

    /// Return the HoleTask corresponding to a given hole_id if present.
    pub fn task_for_hole(&self, hole_id: &str) -> Option<&HoleTask> {
        self.tasks.iter().find(|t| t.hole_id == hole_id)
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

    /// 针对单个未证明洞执行离线重新验证与原生 Re-attestation 提交。
    /// 委托给数据库内核下沉接口 reverify_and_reattest_hole，并由当前池同步更新内存账本。
    fn reverify_single_hole(&mut self, hole_id: &str) -> Result<ReverifyOutcome, PoolError> {
        let env = match self.envelopes.get(hole_id) {
            Some(e) => e,
            None => {
                return Ok(ReverifyOutcome::Skipped {
                    hole_id: hole_id.to_string(),
                    reason: "envelope not found".into(),
                });
            }
        };

        let func_opt = self
            .verified_module
            .functions
            .iter()
            .find(|f| f.name == env.target_function);

        let Some(func) = func_opt else {
            return Ok(ReverifyOutcome::Skipped {
                hole_id: hole_id.to_string(),
                reason: format!(
                    "target function `{}` not found in module",
                    env.target_function
                ),
            });
        };

        if axiom_ast::expr_contains_hole(&func.body) {
            return Ok(ReverifyOutcome::Skipped {
                hole_id: hole_id.to_string(),
                reason: "function still contains pending hole expression".into(),
            });
        }

        // [Standards 4] 下沉深接口：直接交由数据库内核完成主线落地见证核验、SMT 定理证明与原子双重 CAS 提交
        match self.db.reverify_and_reattest_hole(hole_id) {
            Ok(commit) => {
                let rcpt_hash = commit.receipt_hash().clone();
                self.apply_verified_commit(commit, self.verified_module.clone(), hole_id);
                Ok(ReverifyOutcome::Committed {
                    hole_id: hole_id.to_string(),
                    receipt_hash: rcpt_hash,
                })
            }
            Err(CasError::UnverifiedCandidate(reason)) => Ok(ReverifyOutcome::Unproved {
                hole_id: hole_id.to_string(),
                smt_result: SmtResult::Unknown(reason),
            }),
            Err(CasError::VerificationRejected(reason)) => Ok(ReverifyOutcome::Failed {
                hole_id: hole_id.to_string(),
                reason: format!("SMT rejected: {reason}"),
            }),
            Err(CasError::VacuousContract(reason)) => Ok(ReverifyOutcome::Failed {
                hole_id: hole_id.to_string(),
                reason: format!("vacuous contract: {reason}"),
            }),
            Err(CasError::InvalidCandidate(reason)) => Ok(ReverifyOutcome::Failed {
                hole_id: hole_id.to_string(),
                reason,
            }),
            Err(CasError::InvalidEnvelope(reason)) => Ok(ReverifyOutcome::Failed {
                hole_id: hole_id.to_string(),
                reason,
            }),
            Err(CasError::Db(db_err)) => Err(PoolError::CasFailure(db_err.to_string())),
            Err(other) => Ok(ReverifyOutcome::Failed {
                hole_id: hole_id.to_string(),
                reason: format!("re-attestation failed: {other}"),
            }),
        }
    }

    /// 重新验证所有未达到当前 Verification Profile 证明标准（即非 Proved 状态）的已填洞。
    /// 遍历所有处于未证明状态的洞任务，提取当前权威模块中该函数的实现，使用当前规则集执行全量证明：
    /// - 若证明通过，使用原生 Re-attestation 接口提交换发带有当前 Profile Hash 的新收据；
    /// - 显式返回正交强类型 ReverifyOutcome（Committed | Unproved | Failed | Skipped），杜绝静默丢失。
    pub fn reverify_unproved_holes(&mut self) -> Result<Vec<ReverifyOutcome>, PoolError> {
        let unproved_holes: Vec<String> = self
            .tasks
            .iter()
            .map(|t| t.hole_id.clone())
            .filter(|h| self.hole_evidence.get(h) != Some(&ProofLevel::Proved))
            .collect();

        let mut results = Vec::new();
        for hole_id in unproved_holes {
            let outcome = self.reverify_single_hole(&hole_id)?;
            results.push(outcome);
        }
        Ok(results)
    }

    /// Returns the lifecycle status of a specific hole task in the pool.
    pub fn hole_status(&self, hole_id: &str) -> Option<HoleStatus> {
        if !self.tasks.iter().any(|task| task.hole_id == hole_id) {
            return None;
        }
        match self.hole_evidence.get(hole_id) {
            Some(ProofLevel::Proved) => Some(HoleStatus::Verified),
            Some(ProofLevel::Sampled) => Some(HoleStatus::Provisional),
            _ => Some(HoleStatus::Unresolved),
        }
    }

    fn rebuild_provisional_module(&mut self) {
        let mut next_provisional = self.verified_module.clone();
        for (draft_id, draft_expr) in &self.provisional_drafts {
            for func in &mut next_provisional.functions {
                replace_hole(&mut func.body, draft_id, draft_expr);
            }
        }
        self.provisional_module = next_provisional;
    }

    fn persist_and_apply_draft(
        &mut self,
        hole_id: &str,
        solution: Expr,
        solution_source: &str,
        agent_id: &str,
    ) -> Result<(), PoolError> {
        // [Spec 3, P1] 生产路径全面接入类型化 Draft CAS，首次创建明确传入 Some(0) 防并发覆盖
        let existing_draft = self.db.get_draft(hole_id);
        let expected_base_root = self.db.current_root();
        let cas = match existing_draft {
            Some(d) => DraftCasExpectation::update(d.revision, expected_base_root.clone()),
            None => DraftCasExpectation::initial(expected_base_root.clone()),
        };

        let draft_rec = axiom_db::DraftRecord {
            hole_id: hole_id.to_string(),
            agent_id: agent_id.to_string(),
            code: solution_source.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            base_root: expected_base_root,
            revision: 0,
        };
        self.db.insert_draft_cas(draft_rec, &cas).map_err(|e| {
            PoolError::CasFailure(format!(
                "failed to persist provisional draft via CAS: {}",
                e
            ))
        })?;

        self.provisional_drafts
            .insert(hole_id.to_string(), solution);
        // 注意：Provisional 状态的草稿解不移出 pending_holes，仍需升级为正式 Verified！
        self.hole_evidence
            .insert(hole_id.to_string(), ProofLevel::Sampled);
        self.rebuild_provisional_module();
        Ok(())
    }

    /// Submits a provisional draft solution without requiring SMT formal proof.
    /// Marks the hole as Provisional in the ledger, allowing early assembly previews
    /// while remaining open for upgrade to Verified.
    pub fn submit_draft_solution(
        &mut self,
        hole_id: &str,
        solution_source: &str,
    ) -> Result<(), PoolError> {
        let current_status = self
            .hole_status(hole_id)
            .ok_or_else(|| PoolError::UnknownHole(hole_id.to_owned()))?;

        if current_status == HoleStatus::Verified {
            return Err(PoolError::AlreadyResolved(hole_id.to_owned()));
        }

        let solution = parse_expr(solution_source)?;
        if axiom_ast::expr_contains_hole(&solution) {
            return Err(PoolError::CandidateContainsHole(hole_id.to_owned()));
        }

        let mut found = false;
        for func in &mut self.verified_module.clone().functions {
            if replace_hole(&mut func.body, hole_id, &solution) {
                found = true;
                break;
            }
        }
        if !found {
            return Err(PoolError::MissingHole(hole_id.to_owned()));
        }

        self.persist_and_apply_draft(hole_id, solution, solution_source, "axiom-worker")
    }

    /// Parse, verify, and conditionally commit a candidate hole implementation via CAS.
    pub fn submit_solution(
        &mut self,
        hole_id: &str,
        solution_source: &str,
    ) -> Result<SmtResult, PoolError> {
        self.submit_solution_with_agent(hole_id, solution_source, "axiom-worker")
    }

    fn apply_verified_commit(
        &mut self,
        commit_record: CommitRecord,
        committed_module: Module,
        hole_id: &str,
    ) {
        self.commits.push(commit_record);
        self.verified_module = committed_module;
        self.pending_holes.remove(hole_id);
        self.provisional_drafts.remove(hole_id);
        self.hole_evidence
            .insert(hole_id.to_string(), ProofLevel::Proved);

        // [Spec 3, P1] DB 已在提交事务锁内原子删除草稿，严禁在锁外执行第二次删除以防误删并发新草稿！

        // 刷新 provisional_module：基于最新 verified_module，叠加剩余所有草稿
        self.rebuild_provisional_module();

        let latest_root = self.db.current_root();
        for env in self.envelopes.values_mut() {
            env.base_root_hash = latest_root.clone();
        }
    }

    /// Submit a candidate solution with full outcome reporting (precheck SMT result, CAS events, and status upgrade).
    pub fn submit_solution_with_outcome(
        &mut self,
        hole_id: &str,
        solution_source: &str,
        agent_id: &str,
    ) -> Result<SubmitOutcome, PoolError> {
        let current_status = self
            .hole_status(hole_id)
            .ok_or_else(|| PoolError::UnknownHole(hole_id.to_owned()))?;

        // 仅拦截已被正式证明（Verified）的洞；处于 Provisional 的草稿解允许继续提交升级为正式证明！
        if current_status == HoleStatus::Verified {
            return Err(PoolError::AlreadyResolved(hole_id.to_owned()));
        }

        let was_provisional = current_status == HoleStatus::Provisional;

        let solution = parse_expr(solution_source)?;
        if axiom_ast::expr_contains_hole(&solution) {
            return Err(PoolError::CandidateContainsHole(hole_id.to_owned()));
        }

        // 关键修复：打洞候选始终基于权威 verified_module 构建，使未证明的洞（即使处于 Provisional）保持为 Expr::Hole，
        // 绝不会因为之前存入草稿而发生 replace_hole 找不到洞的 MissingHole 错误！
        let mut candidate_module = self.verified_module.clone();
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
        let base_func = &candidate_module.functions[affected_functions[0]];
        let contract_hash = compute_contract_hash(base_func);
        let dep_hashes = extract_function_dependencies(base_func, &candidate_module);
        let current_root = self.db.current_root();
        let envelope = self
            .envelopes
            .get(hole_id)
            .cloned()
            .unwrap_or_else(|| TaskEnvelope {
                base_root_hash: current_root.clone(),
                target_hole_id: hole_id.to_owned(),
                target_function: candidate_module.functions[affected_functions[0]]
                    .name
                    .clone(),
                contract_hash,
                dependency_hashes: dep_hashes,
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
            SmtResult::CounterExample(_)
            | SmtResult::VacuousContract { .. }
            | SmtResult::Unknown(_) => {
                return Ok(SubmitOutcome {
                    smt_result: precheck_result,
                    cas_events: Vec::new(),
                    upgraded_to_verified: false,
                    conflict_report: None,
                });
            }
            SmtResult::Skipped(_) => {
                let func = &candidate_module.functions[affected_functions[0]];
                if func.ensures.is_empty() && self.policy == PoolVerificationPolicy::Permissive {
                    self.persist_and_apply_draft(hole_id, solution, solution_source, agent_id)?;
                    return Ok(SubmitOutcome {
                        smt_result: precheck_result,
                        cas_events: Vec::new(),
                        upgraded_to_verified: false,
                        conflict_report: None,
                    });
                }
                return Ok(SubmitOutcome {
                    smt_result: precheck_result,
                    cas_events: Vec::new(),
                    upgraded_to_verified: false,
                    conflict_report: None,
                });
            }
            SmtResult::Sampled { points_checked: _ } => {
                if self.policy == PoolVerificationPolicy::Permissive {
                    self.persist_and_apply_draft(hole_id, solution, solution_source, agent_id)?;
                    return Ok(SubmitOutcome {
                        smt_result: precheck_result,
                        cas_events: Vec::new(),
                        upgraded_to_verified: false,
                        conflict_report: None,
                    });
                } else {
                    return Ok(SubmitOutcome {
                        smt_result: precheck_result,
                        cas_events: Vec::new(),
                        upgraded_to_verified: false,
                        conflict_report: None,
                    });
                }
            }
            SmtResult::Proved { .. } => {}
        }

        // 2. 核心两阶段提交：直接交由数据库内核验证并铸造收据！
        let mut cas_events = Vec::new();
        let expected_root = envelope.base_root_hash.clone();
        match self.db.try_commit_candidate(
            &expected_root,
            envelope.clone(),
            candidate.clone(),
            Some(&candidate_module),
        ) {
            Ok(commit_record) => {
                self.apply_verified_commit(commit_record, candidate_module, hole_id);
                Ok(SubmitOutcome {
                    smt_result: precheck_result,
                    cas_events,
                    upgraded_to_verified: was_provisional,
                    conflict_report: None,
                })
            }
            Err(CasError::StaleRoot { current, expected }) => {
                cas_events.push(CasEvent::StaleRootDetected {
                    expected,
                    current: current.clone(),
                });

                // 从数据库 CAS 物理加载最新真实 AST 实施 Fast-Forward 重基！
                let latest_root = current.clone().ok_or_else(|| {
                    PoolError::CasFailure("stale root returned None current root".to_string())
                })?;
                let latest_module = self
                    .db
                    .get_module(&latest_root)
                    .map_err(|e| PoolError::CasFailure(e.to_string()))?
                    .ok_or_else(|| {
                        PoolError::CasFailure(format!(
                            "latest module for root `{latest_root}` not found in database"
                        ))
                    })?;

                // 获取 base_module：优先根据 envelope.base_root_hash 从数据库加载
                let base_module = if let Some(ref base_root) = envelope.base_root_hash {
                    self.db
                        .get_module(base_root)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| self.verified_module.clone())
                } else {
                    self.verified_module.clone()
                };

                // 核心：调用 AST 级结构化三路合并算法 (Ticket 04)
                let merge_result =
                    merge_modules_3way(&base_module, &candidate_module, &latest_module);

                let rebased_module = match merge_result {
                    Ok(merged) => merged,
                    Err(conflict_report) => {
                        let conflict_count = conflict_report.conflicts.len();
                        let summary = format!("{conflict_report}");
                        cas_events.push(CasEvent::AstMergeConflict {
                            conflict_count,
                            summary: summary.clone(),
                        });
                        return Ok(SubmitOutcome {
                            smt_result: SmtResult::Skipped(format!(
                                "AST 3-way merge conflict detected: {summary}"
                            )),
                            cas_events,
                            upgraded_to_verified: false,
                            conflict_report: Some(conflict_report),
                        });
                    }
                };

                let base_func = rebased_module
                    .functions
                    .iter()
                    .find(|f| f.name == envelope.target_function)
                    .cloned()
                    .ok_or_else(|| PoolError::MissingHole(hole_id.to_owned()))?;
                let rebased_func_hash = compute_contract_hash(&base_func);
                let rebased_deps = extract_function_dependencies(&base_func, &rebased_module);

                // 核心：执行跨符号级联 SMT 定理重验证 (Ticket 05)
                let cascade_report =
                    verify_dependent_closure(&rebased_module, &[&envelope.target_function]);

                if !cascade_report.is_all_proved() {
                    let failed_funcs: Vec<String> = cascade_report
                        .failures
                        .iter()
                        .map(|f| f.function_name.clone())
                        .collect();
                    let failure_reasons: Vec<String> = cascade_report
                        .failures
                        .iter()
                        .map(|f| format!("{}: {}", f.function_name, f.reason))
                        .collect();
                    cas_events.push(CasEvent::CascadeVerificationFailed {
                        failed_functions: failed_funcs,
                        reasons: failure_reasons,
                    });

                    let representative_result = cascade_report
                        .results
                        .get(&envelope.target_function)
                        .cloned()
                        .unwrap_or_else(|| {
                            cascade_report
                                .results
                                .values()
                                .find(|r| !r.is_proved())
                                .cloned()
                                .unwrap_or(SmtResult::Unknown("cascade verification failed".into()))
                        });

                    return Ok(SubmitOutcome {
                        smt_result: representative_result,
                        cas_events,
                        upgraded_to_verified: false,
                        conflict_report: None,
                    });
                }

                let rebased_envelope = TaskEnvelope {
                    base_root_hash: Some(latest_root.clone()),
                    target_hole_id: hole_id.to_owned(),
                    target_function: envelope.target_function.clone(),
                    contract_hash: rebased_func_hash,
                    dependency_hashes: rebased_deps,
                };
                let rebased_candidate = Candidate {
                    envelope_hash: compute_envelope_hash(&rebased_envelope),
                    expression_source: solution_source.to_string(),
                    expression_hash: cand_expr_hash,
                    author_agent_id: agent_id.to_string(),
                    timestamp: candidate.timestamp + 1,
                };

                let commit_record = self
                    .db
                    .try_commit_candidate(
                        &Some(latest_root.clone()),
                        rebased_envelope,
                        rebased_candidate,
                        Some(&rebased_module),
                    )
                    .map_err(|e| PoolError::CasFailure(e.to_string()))?;

                cas_events.push(CasEvent::RebaseSucceeded {
                    rebased_root: latest_root.clone(),
                    attempts: 1,
                });
                cas_events.push(CasEvent::Ast3WayRebaseSucceeded {
                    rebased_root: latest_root,
                    cascade_verified_count: cascade_report.verified_functions.len(),
                });

                self.apply_verified_commit(commit_record, rebased_module, hole_id);
                Ok(SubmitOutcome {
                    smt_result: cascade_report
                        .results
                        .get(&envelope.target_function)
                        .cloned()
                        .unwrap_or(precheck_result),
                    cas_events,
                    upgraded_to_verified: was_provisional,
                    conflict_report: None,
                })
            }
            Err(other) => Err(PoolError::CasFailure(other.to_string())),
        }
    }

    /// Parse, verify, and conditionally commit a candidate hole implementation with an explicit agent ID.
    pub fn submit_solution_with_agent(
        &mut self,
        hole_id: &str,
        solution_source: &str,
        agent_id: &str,
    ) -> Result<SmtResult, PoolError> {
        let outcome = self.submit_solution_with_outcome(hole_id, solution_source, agent_id)?;
        Ok(outcome.smt_result)
    }

    /// Return whether every dispatched hole has been mathematically verified with formal SMT proof.
    pub fn is_complete(&self) -> bool {
        self.pending_holes.is_empty()
    }

    /// Assemble a clone of the verified, hole-free module.
    ///
    /// 严禁妥协强门禁：
    /// 1. 所有已分发的洞必须已被解决（非 Unresolved）；
    /// 2. 证据账本强门禁：所有已分发洞 100% 具备 ProofLevel::Proved 证据；
    ///    若有任何洞仅为 Sampled 或 Unverified，坚决拒绝，返回 PoolError::InsufficientEvidence！
    /// 3. 深入 AST 扫描，若有残留的 Expr::Hole，拒绝导出。
    pub fn assemble_verified_module(&self) -> Result<Module, PoolError> {
        let unresolved_holes: Vec<String> = self
            .tasks
            .iter()
            .filter(|t| self.hole_status(&t.hole_id) == Some(HoleStatus::Unresolved))
            .map(|t| t.hole_id.clone())
            .collect();
        if !unresolved_holes.is_empty() {
            return Err(PoolError::Incomplete(unresolved_holes));
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
            return Err(PoolError::Incomplete(vec![
                "unresolved_ast_hole".to_string()
            ]));
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
pub fn verify_candidate_precheck(module: &Module, affected_functions: &[usize]) -> SmtResult {
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
    CandidateContainsHole(String),
    Incomplete(Vec<String>),
    InsufficientEvidence { hole_id: String, level: ProofLevel },
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
            Self::CandidateContainsHole(hole_id) => write!(
                formatter,
                "candidate solution for `{hole_id}` contains an unresolved hole, candidates must be closed expressions"
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
            | Self::CandidateContainsHole(_)
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

impl From<axiom_db::DbError> for PoolError {
    fn from(error: axiom_db::DbError) -> Self {
        Self::CasFailure(error.to_string())
    }
}

impl From<axiom_db::CasError> for PoolError {
    fn from(error: axiom_db::CasError) -> Self {
        Self::CasFailure(error.to_string())
    }
}

fn replace_hole(expression: &mut Expr, hole_id: &str, solution: &Expr) -> bool {
    let mut replaced = false;
    axiom_ast::visit_expr_mut_control(expression, &mut |expr| {
        if let Expr::Hole(current_id) = expr {
            if current_id == hole_id {
                *expr = solution.clone();
                replaced = true;
                return axiom_ast::VisitControl::SkipChildren;
            }
        }
        axiom_ast::VisitControl::Continue
    });
    replaced
}

fn ast_contains_any_hole_in_expr(expression: &Expr) -> bool {
    let mut has_hole = false;
    axiom_ast::visit_expr(expression, &mut |expr| {
        if matches!(expr, Expr::Hole(_)) {
            has_hole = true;
        }
    });
    has_hole
}

pub fn ast_contains_any_hole(module: &Module) -> bool {
    module
        .functions
        .iter()
        .any(|f| ast_contains_any_hole_in_expr(&f.body))
}

fn extract_function_dependencies(func: &axiom_ast::FunctionDecl, module: &Module) -> Vec<NodeHash> {
    let mut deps = Vec::new();
    let mut symbols = HashSet::new();

    for param in &func.params {
        if let axiom_ast::Type::Custom(ref name) = param.ty {
            symbols.insert(name.clone());
        }
    }
    if let axiom_ast::Type::Custom(ref name) = func.return_type {
        symbols.insert(name.clone());
    }

    let collect_expr_symbols = |expr: &Expr, syms: &mut HashSet<String>| {
        axiom_ast::visit_expr(expr, &mut |e| match e {
            Expr::Ident(name) => {
                syms.insert(name.clone());
            }
            Expr::RecordInit {
                ty_name: Some(name),
                ..
            } => {
                syms.insert(name.clone());
            }
            _ => {}
        });
    };

    for req in &func.requires {
        collect_expr_symbols(req, &mut symbols);
    }
    for ens in &func.ensures {
        collect_expr_symbols(ens, &mut symbols);
    }
    collect_expr_symbols(&func.body, &mut symbols);

    for ty in &module.types {
        let ty_name = match ty {
            axiom_ast::TypeDef::Record { name, .. } => name,
            axiom_ast::TypeDef::Enum { name, .. } => name,
        };
        if symbols.contains(ty_name) {
            deps.push(axiom_db::compute_type_hash(ty));
        }
    }

    for other_func in &module.functions {
        if other_func.name != func.name && symbols.contains(&other_func.name) {
            deps.push(compute_function_hash(other_func));
        }
    }

    for effect_name in &func.effects {
        if let Some(eff) = module.effects.iter().find(|e| &e.name == effect_name) {
            deps.push(axiom_db::compute_effect_hash(eff));
        }
    }

    deps.sort();
    deps.dedup();
    deps
}
