//! Content-addressed storage and Verifiable Change Protocol for Axiom abstract syntax trees.

use axiom_ast::{EffectDecl, Expr, FunctionDecl, FunctionParam, Module, Type, TypeDef};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// A stable, typed SHA-256 content identifier for an AST node or protocol artifact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeHash(String);

impl NodeHash {
    /// Returns the textual content identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for NodeHash {
    fn default() -> Self {
        Self("#null_0000000000000000000000000000000000000000000000000000000000000000".into())
    }
}

/// A malformed textual AST node hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeHashError {
    value: String,
}

impl fmt::Display for ParseNodeHashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid AST node hash `{}`; expected `#<kind>_<64 lowercase hex digits>`",
            self.value
        )
    }
}

impl std::error::Error for ParseNodeHashError {}

impl FromStr for NodeHash {
    type Err = ParseNodeHashError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let valid_prefix = [
            "#type_", "#fn_", "#effect_", "#mod_", "#env_", "#cand_", "#rcpt_", "#commit_",
            "#obl_", "#vprof_", "#null_",
        ]
        .iter()
        .find(|prefix| value.starts_with(**prefix));
        let digest = valid_prefix.and_then(|prefix| value.strip_prefix(prefix));
        if digest.is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) {
            Ok(Self(value.to_owned()))
        } else {
            Err(ParseNodeHashError {
                value: value.to_owned(),
            })
        }
    }
}

impl AsRef<str> for NodeHash {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for NodeHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn compute_hash<T: Serialize>(prefix: &str, value: &T) -> NodeHash {
    let bytes = serde_json::to_vec(value).expect("Axiom serialization cannot fail");
    let digest = Sha256::digest(bytes);
    NodeHash(format!("#{prefix}_{digest:x}"))
}

/// Computes the content identifier of a type definition.
pub fn compute_type_hash(def: &TypeDef) -> NodeHash {
    compute_hash("type", def)
}

/// Computes the content identifier of a function declaration.
pub fn compute_function_hash(func: &FunctionDecl) -> NodeHash {
    compute_hash("fn", func)
}

/// Computes the content identifier of a function's specification contract (signature, effects, requires, ensures).
/// This purposefully excludes the function body implementation, allowing independent hole-filling.
pub fn compute_contract_hash(func: &FunctionDecl) -> NodeHash {
    #[derive(Serialize)]
    struct ContractPayload<'a> {
        name: &'a str,
        params: &'a [FunctionParam],
        return_type: &'a Type,
        effects: &'a [String],
        requires: &'a [Expr],
        ensures: &'a [Expr],
    }
    let payload = ContractPayload {
        name: &func.name,
        params: &func.params,
        return_type: &func.return_type,
        effects: &func.effects,
        requires: &func.requires,
        ensures: &func.ensures,
    };
    compute_hash("contract", &payload)
}

/// Computes the content identifier of an effect declaration.
pub fn compute_effect_hash(eff: &EffectDecl) -> NodeHash {
    compute_hash("effect", eff)
}

/// Computes the content identifier of a module.
pub fn compute_module_hash(module: &Module) -> NodeHash {
    compute_hash("mod", module)
}

/// Computes the content identifier of an expression.
pub fn compute_expr_hash(expr: &Expr) -> NodeHash {
    compute_hash("expr", expr)
}

/// Computes the content identifier for formal proof obligations of a function.
pub fn compute_proof_obligation_hash(func: &FunctionDecl) -> NodeHash {
    #[derive(Serialize)]
    struct ObligationPayload<'a> {
        target_function: &'a str,
        params: &'a [FunctionParam],
        return_type: &'a Type,
        requires: &'a [Expr],
        ensures: &'a [Expr],
    }
    let payload = ObligationPayload {
        target_function: &func.name,
        params: &func.params,
        return_type: &func.return_type,
        requires: &func.requires,
        ensures: &func.ensures,
    };
    compute_hash("obl", &payload)
}

/// Content-addressed task envelope defining the scope and formal baseline of a hole task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEnvelope {
    pub base_root_hash: Option<NodeHash>,
    pub target_hole_id: String,
    pub target_function: String,
    pub contract_hash: NodeHash,
    pub dependency_hashes: Vec<NodeHash>,
}

/// Computes content identifier for a task envelope.
pub fn compute_envelope_hash(env: &TaskEnvelope) -> NodeHash {
    compute_hash("env", env)
}

/// A candidate expression implementation produced by an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub envelope_hash: NodeHash,
    pub expression_source: String,
    pub expression_hash: NodeHash,
    pub author_agent_id: String,
    pub timestamp: u64,
}

/// Computes content identifier for a candidate.
pub fn compute_candidate_hash(cand: &Candidate) -> NodeHash {
    compute_hash("cand", cand)
}

/// Proof quality level for formal evidence grading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProofLevel {
    /// Formally proved globally by external SMT solver (e.g. Z3 Unsat).
    Proved,
    /// Verified on deterministic physical unit tests / ground-truth vectors.
    Tested,
    /// Bounded discrete grid sampling (Blocked from production release).
    Sampled,
    /// Solver unknown or timed out.
    Unknown,
    /// Unverified / unchecked candidate.
    Unverified,
}

impl fmt::Display for ProofLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proved => write!(f, "Proved"),
            Self::Tested => write!(f, "Tested"),
            Self::Sampled => write!(f, "Sampled"),
            Self::Unknown => write!(f, "Unknown"),
            Self::Unverified => write!(f, "Unverified"),
        }
    }
}

/// Current verification profile specification identifying SMT encoding rules,
/// arithmetic overflow guards, division-by-zero path-condition rules, and solver requirements.
pub const CURRENT_VERIFICATION_PROFILE: &str = "axiom-smt-v3-truncdiv-residualmatch";

/// Returns the cryptographically secure hash of the current verification profile.
pub fn current_verification_profile_hash() -> NodeHash {
    compute_hash("vprof", &CURRENT_VERIFICATION_PROFILE)
}

/// Returns a deterministic fallback hash for legacy unversioned receipts.
pub fn legacy_verification_profile_hash() -> NodeHash {
    compute_hash("vprof", &"axiom-legacy-unversioned-profile")
}

/// Cryptographically signed verification receipt for an accepted candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReceipt {
    pub candidate_hash: NodeHash,
    pub proof_level: ProofLevel,
    pub solver: String,
    pub solver_version: String,
    pub obligations_checked: usize,
    pub proof_obligation_hash: NodeHash,
    #[serde(default = "legacy_verification_profile_hash")]
    pub verification_profile_hash: NodeHash,
    pub verified_at: u64,
}

/// Computes content identifier for a verification receipt.
pub fn compute_receipt_hash(rcpt: &VerificationReceipt) -> NodeHash {
    compute_hash("rcpt", rcpt)
}

/// Non-authoritative draft implementation for a hole under provisional trial.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DraftRecord {
    pub hole_id: String,
    pub agent_id: String,
    pub code: String,
    pub timestamp: u64,
    #[serde(default)]
    pub base_root: Option<NodeHash>,
    #[serde(default)]
    pub revision: u64,
}

/// The immutable data payload of a commit, excluding the commit's self-identifying hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitPayload {
    #[serde(default)]
    pub parent_commit: Option<NodeHash>,
    pub parent_root: Option<NodeHash>,
    pub new_root: NodeHash,
    pub envelope_hash: NodeHash,
    pub candidate_hash: NodeHash,
    pub receipt_hash: NodeHash,
    pub timestamp: u64,
}

/// An immutable commit record linking parent root, new root, envelope, candidate, and receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitRecord {
    pub commit_hash: NodeHash,
    #[serde(flatten)]
    pub payload: CommitPayload,
}

impl std::ops::Deref for CommitRecord {
    type Target = CommitPayload;
    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl CommitRecord {
    /// Creates a commit record by hashing the commit payload.
    pub fn new(payload: CommitPayload) -> Self {
        let commit_hash = compute_commit_payload_hash(&payload);
        Self {
            commit_hash,
            payload,
        }
    }

    /// Access parent commit hash reference in DAG topology.
    pub fn parent_commit(&self) -> Option<&NodeHash> {
        self.payload.parent_commit.as_ref()
    }

    /// Access parent root reference.
    pub fn parent_root(&self) -> Option<&NodeHash> {
        self.payload.parent_root.as_ref()
    }

    /// Access new root reference.
    pub fn new_root(&self) -> &NodeHash {
        &self.payload.new_root
    }

    /// Access envelope content identifier.
    pub fn envelope_hash(&self) -> &NodeHash {
        &self.payload.envelope_hash
    }

    /// Access candidate content identifier.
    pub fn candidate_hash(&self) -> &NodeHash {
        &self.payload.candidate_hash
    }

    /// Access receipt content identifier.
    pub fn receipt_hash(&self) -> &NodeHash {
        &self.payload.receipt_hash
    }

    /// Access commit timestamp.
    pub fn timestamp(&self) -> u64 {
        self.payload.timestamp
    }

    /// Verify cryptographic self-consistency of this commit record.
    pub fn verify_self_hash(&self) -> bool {
        compute_commit_payload_hash(&self.payload) == self.commit_hash
    }
}

/// Computes content identifier for a commit payload.
pub fn compute_commit_payload_hash(payload: &CommitPayload) -> NodeHash {
    compute_hash("commit", payload)
}

/// Computes content identifier for a commit record.
pub fn compute_commit_hash(commit: &CommitRecord) -> NodeHash {
    compute_commit_payload_hash(&commit.payload)
}

/// The content identifiers created while inserting a complete module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleRecord {
    pub module_hash: NodeHash,
    pub type_hashes: Vec<NodeHash>,
    pub function_hashes: Vec<NodeHash>,
    pub effect_hashes: Vec<NodeHash>,
}

impl ModuleRecord {
    /// Returns the module's content identifier.
    pub fn hash(&self) -> &NodeHash {
        &self.module_hash
    }
}

/// An error opening or updating an AST database.
#[derive(Debug)]
pub enum DbError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
    CorruptedCommitChain(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "AST database I/O error: {error}"),
            Self::Serialization(error) => {
                write!(formatter, "invalid AST database serialization: {error}")
            }
            Self::CorruptedCommitChain(msg) => {
                write!(formatter, "AST database commit chain corrupted: {msg}")
            }
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Serialization(error) => Some(error),
            Self::CorruptedCommitChain(_) => None,
        }
    }
}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

/// Error encountered during CAS (Compare-And-Swap) commit.
#[derive(Debug)]
pub enum CasError {
    StaleRoot {
        current: Option<NodeHash>,
        expected: Option<NodeHash>,
    },
    InvalidReceipt(String),
    InvalidCandidate(String),
    InvalidEnvelope(String),
    ForgedModule(String),
    UnverifiedCandidate(String),
    VacuousContract(String),
    VerificationRejected(String),
    StaleDraftRevision {
        hole_id: String,
        current: u64,
        expected: u64,
    },
    StaleRootMismatch {
        hole_id: String,
        current: Option<NodeHash>,
        expected: Option<NodeHash>,
    },
    Db(DbError),
}

impl fmt::Display for CasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRoot { current, expected } => write!(
                f,
                "CAS rejected: stale root ref (expected `{:?}`, current is `{:?}`)",
                expected, current
            ),
            Self::InvalidReceipt(msg) => {
                write!(f, "CAS rejected: invalid verification receipt: {msg}")
            }
            Self::InvalidCandidate(msg) => write!(f, "CAS rejected: invalid candidate: {msg}"),
            Self::InvalidEnvelope(msg) => write!(f, "CAS rejected: invalid task envelope: {msg}"),
            Self::ForgedModule(msg) => write!(f, "CAS rejected: forged module mismatch: {msg}"),
            Self::UnverifiedCandidate(msg) => {
                write!(f, "CAS rejected: candidate does not meet proof bar: {msg}")
            }
            Self::VacuousContract(msg) => write!(
                f,
                "CAS rejected: vacuous contract with unsatisfiable preconditions: {msg}"
            ),
            Self::VerificationRejected(msg) => {
                write!(f, "CAS rejected: formal theorem verification failed: {msg}")
            }
            Self::StaleDraftRevision {
                hole_id,
                current,
                expected,
            } => write!(
                f,
                "CAS rejected: stale draft revision for hole `{hole_id}` (expected {expected}, current is {current})"
            ),
            Self::StaleRootMismatch {
                hole_id,
                current,
                expected,
            } => write!(
                f,
                "CAS rejected: stale base root for hole `{hole_id}` (expected `{:?}`, current is `{:?}`)",
                expected, current
            ),
            Self::Db(err) => write!(f, "database error during CAS: {err}"),
        }
    }
}

impl std::error::Error for CasError {}

impl From<DbError> for CasError {
    fn from(err: DbError) -> Self {
        Self::Db(err)
    }
}

impl From<std::io::Error> for CasError {
    fn from(err: std::io::Error) -> Self {
        Self::Db(DbError::Io(err))
    }
}

impl From<serde_json::Error> for CasError {
    fn from(err: serde_json::Error) -> Self {
        Self::Db(DbError::Serialization(err))
    }
}

/// Kernel-minted cryptographic verification certificate computed offline without database locks.
/// This certificate is sealed: it cannot be constructed from outside axiom-db or deserialized arbitrarily.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedVerification {
    pub(crate) base_root_hash: Option<NodeHash>,
    pub(crate) envelope_hash: NodeHash,
    pub(crate) candidate_hash: NodeHash,
    pub(crate) proof_obligation_hash: NodeHash,
    pub(crate) derived_module: Module,
    pub(crate) solver: String,
    pub(crate) solver_version: String,
    pub(crate) obligations_checked: usize,
    pub(crate) _sealed: (),
}

impl CertifiedVerification {
    pub fn base_root_hash(&self) -> Option<&NodeHash> {
        self.base_root_hash.as_ref()
    }
    pub fn envelope_hash(&self) -> &NodeHash {
        &self.envelope_hash
    }
    pub fn candidate_hash(&self) -> &NodeHash {
        &self.candidate_hash
    }
    pub fn proof_obligation_hash(&self) -> &NodeHash {
        &self.proof_obligation_hash
    }
    pub fn derived_module(&self) -> &Module {
        &self.derived_module
    }
    pub fn solver(&self) -> &str {
        &self.solver
    }
    pub fn solver_version(&self) -> &str {
        &self.solver_version
    }
    pub fn obligations_checked(&self) -> usize {
        self.obligations_checked
    }
}

/// Internal storage state of the AST database, accessible within transactions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatabaseState {
    #[serde(default)]
    pub(crate) root_ref: Option<NodeHash>,
    #[serde(default)]
    pub(crate) head_commit: Option<NodeHash>,
    #[serde(default)]
    pub(crate) types: BTreeMap<NodeHash, TypeDef>,
    #[serde(default)]
    pub(crate) functions: BTreeMap<NodeHash, FunctionDecl>,
    #[serde(default)]
    pub(crate) effects: BTreeMap<NodeHash, EffectDecl>,
    #[serde(default)]
    pub(crate) modules: BTreeMap<NodeHash, Module>,
    #[serde(default)]
    pub(crate) envelopes: BTreeMap<NodeHash, TaskEnvelope>,
    #[serde(default)]
    pub(crate) candidates: BTreeMap<NodeHash, Candidate>,
    #[serde(default)]
    pub(crate) receipts: BTreeMap<NodeHash, VerificationReceipt>,
    #[serde(default)]
    pub(crate) commits: BTreeMap<NodeHash, CommitRecord>,
    #[serde(default)]
    pub(crate) drafts: BTreeMap<String, DraftRecord>,
}

/// A content-addressed AST database backed by a JSON file or memory.
#[derive(Debug, Clone)]
pub struct AstDatabase {
    path: Option<PathBuf>,
    state: DatabaseState,
}

/// Deterministically derives a new module from `base_module` by substituting `envelope.target_hole_id`
/// with `parsed_candidate` inside `envelope.target_function`.
pub fn derive_candidate_module(
    base_module: &Module,
    envelope: &TaskEnvelope,
    parsed_candidate: &Expr,
) -> Result<Module, CasError> {
    let mut derived_module = base_module.clone();
    let mut replaced = false;
    for func in &mut derived_module.functions {
        if func.name == envelope.target_function
            && (replace_hole_in_expr(&mut func.body, &envelope.target_hole_id, parsed_candidate)
                || func.body == *parsed_candidate)
        {
            replaced = true;
            break;
        }
    }

    if !replaced {
        return Err(CasError::InvalidEnvelope(format!(
            "target hole `{}` not found in target function `{}`",
            envelope.target_hole_id, envelope.target_function
        )));
    }

    Ok(derived_module)
}

impl AstDatabase {
    /// Opens an existing database file or creates an empty one at `path`.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if path.exists() {
            let bytes = fs::read(path)?;
            let state = serde_json::from_slice(&bytes)?;
            Ok(Self {
                path: Some(path.to_path_buf()),
                state,
            })
        } else {
            let database = Self {
                path: Some(path.to_path_buf()),
                state: DatabaseState::default(),
            };
            database.persist(&database.state)?;
            Ok(database)
        }
    }

    /// Creates an empty database without a persistent backing file.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            state: DatabaseState::default(),
        }
    }

    /// Returns the current root module hash reference.
    pub fn current_root(&self) -> Option<NodeHash> {
        self.state.root_ref.clone()
    }

    /// Returns the current head commit hash reference.
    pub fn head_commit(&self) -> Option<&NodeHash> {
        self.state.head_commit.as_ref()
    }

    /// Returns the verified mainline commits sequence ordered from earliest ancestor to latest head.
    /// Performs integrity validation, broken chain checks, and self-loop / cycle detection.
    pub fn mainline_commits(&self) -> Result<Vec<&CommitRecord>, DbError> {
        let mut ordered_commits = Vec::new();
        let mut curr_commit_hash = self.state.head_commit.as_ref();
        let mut visited: HashSet<&NodeHash> = HashSet::new();

        while let Some(hash) = curr_commit_hash {
            if !visited.insert(hash) {
                return Err(DbError::CorruptedCommitChain(format!(
                    "cycle detected in commit chain at commit hash {}",
                    hash
                )));
            }
            let commit = self.state.commits.get(hash).ok_or_else(|| {
                DbError::CorruptedCommitChain(format!(
                    "broken commit chain: parent commit hash {} not found in database",
                    hash
                ))
            })?;
            ordered_commits.push(commit);
            curr_commit_hash = commit.parent_commit();
        }

        // 若历史旧版本数据库缺少 head_commit，但存在 commits，按时间戳稳定排序作为 fallback
        if ordered_commits.is_empty() && !self.state.commits.is_empty() {
            let mut fallback: Vec<&CommitRecord> = self.state.commits.values().collect();
            fallback.sort_by_key(|c| c.timestamp());
            return Ok(fallback);
        }

        ordered_commits.reverse();
        Ok(ordered_commits)
    }

    fn lock_and_sync(&mut self) -> Result<Option<FileLockGuard>, DbError> {
        let lock = self
            .path
            .as_ref()
            .map(|p| FileLockGuard::acquire(p))
            .transpose()?;
        if let Some(path) = &self.path {
            if path.exists() {
                let bytes = fs::read(path)?;
                let disk_state: DatabaseState = serde_json::from_slice(&bytes)?;
                self.state = disk_state;
            }
        }
        Ok(lock)
    }

    /// Executes a state-mutating transaction with an exclusive OS file lock and read-latest consistency.
    pub(crate) fn locked_transaction<T, E>(
        &mut self,
        action: impl FnOnce(&mut DatabaseState) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<DbError>,
    {
        let _lock = self.lock_and_sync()?;
        let mut next_state = self.state.clone();
        let result = action(&mut next_state)?;
        self.persist(&next_state).map_err(E::from)?;
        self.state = next_state;
        Ok(result)
    }

    /// Safely initializes the root module reference when no root exists yet.
    pub fn initialize_root(&mut self, root: NodeHash) -> Result<(), DbError> {
        self.locked_transaction(|state| {
            if state.root_ref.is_none() {
                state.root_ref = Some(root);
            }
            Ok(())
        })
    }

    fn insert_module_to_state(state: &mut DatabaseState, module: &Module) -> ModuleRecord {
        let type_hashes = module
            .types
            .iter()
            .map(compute_type_hash)
            .collect::<Vec<_>>();
        let function_hashes = module
            .functions
            .iter()
            .map(compute_function_hash)
            .collect::<Vec<_>>();
        let effect_hashes = module
            .effects
            .iter()
            .map(compute_effect_hash)
            .collect::<Vec<_>>();
        let module_hash = compute_module_hash(module);

        for (hash, definition) in type_hashes.iter().zip(&module.types) {
            state.types.insert(hash.clone(), definition.clone());
        }
        for (hash, function) in function_hashes.iter().zip(&module.functions) {
            state.functions.insert(hash.clone(), function.clone());
        }
        for (hash, effect) in effect_hashes.iter().zip(&module.effects) {
            state.effects.insert(hash.clone(), effect.clone());
        }
        state.modules.insert(module_hash.clone(), module.clone());

        ModuleRecord {
            module_hash,
            type_hashes,
            function_hashes,
            effect_hashes,
        }
    }

    /// Inserts a module and every type, function, and effect it owns under lock.
    pub fn insert_module(&mut self, module: &Module) -> Result<ModuleRecord, DbError> {
        self.locked_transaction(|state| {
            let record = Self::insert_module_to_state(state, module);
            if state.root_ref.is_none() {
                state.root_ref = Some(record.module_hash.clone());
            }
            Ok(record)
        })
    }

    /// Records a task envelope into the database under lock.
    pub fn record_envelope(&mut self, envelope: TaskEnvelope) -> Result<NodeHash, DbError> {
        self.locked_transaction(|state| {
            let hash = compute_envelope_hash(&envelope);
            state.envelopes.insert(hash.clone(), envelope);
            Ok(hash)
        })
    }

    /// Records a candidate implementation without committing it to the root ref under lock.
    pub fn record_candidate(&mut self, candidate: Candidate) -> Result<NodeHash, DbError> {
        self.locked_transaction(|state| {
            let hash = compute_candidate_hash(&candidate);
            state.candidates.insert(hash.clone(), candidate);
            Ok(hash)
        })
    }

    /// Performs offline Z3 formal verification of a candidate against the expected base module
    /// without acquiring database file locks.
    ///
    /// This allows expensive mathematical proving to run completely concurrently across workers
    /// without freezing the CAS database write lock.
    pub fn verify_candidate_offline(
        &self,
        expected_root: &Option<NodeHash>,
        envelope: &TaskEnvelope,
        candidate: &Candidate,
        caller_module: Option<&Module>,
    ) -> Result<CertifiedVerification, CasError> {
        // 1. 候选内容哈希自验
        let env_hash = compute_envelope_hash(envelope);
        if candidate.envelope_hash != env_hash {
            return Err(CasError::InvalidCandidate(format!(
                "candidate envelope hash `{}` does not match computed `{}`",
                candidate.envelope_hash, env_hash
            )));
        }

        let parsed_expr = axiom_parser::parse_expr(&candidate.expression_source)
            .map_err(|e| CasError::InvalidCandidate(format!("expression parse error: {e}")))?;
        let expr_hash = compute_expr_hash(&parsed_expr);
        if candidate.expression_hash != expr_hash {
            return Err(CasError::InvalidCandidate(format!(
                "candidate expression hash `{}` does not match parsed expr hash `{}`",
                candidate.expression_hash, expr_hash
            )));
        }

        let cand_hash = compute_candidate_hash(candidate);

        // 2. 信封基线根校验
        if &envelope.base_root_hash != expected_root {
            return Err(CasError::InvalidEnvelope(format!(
                "envelope base root `{:?}` does not match expected root `{:?}`",
                envelope.base_root_hash, expected_root
            )));
        }

        // 3. 信封依赖完整性校验
        for dep in &envelope.dependency_hashes {
            if !self.state.functions.contains_key(dep)
                && !self.state.types.contains_key(dep)
                && !self.state.effects.contains_key(dep)
                && !self.state.modules.contains_key(dep)
            {
                return Err(CasError::InvalidEnvelope(format!(
                    "envelope dependency `{}` not found in database",
                    dep
                )));
            }
        }

        // 4. 内核 AST 独立自推导与基线契约比对
        let base_module = match expected_root {
            Some(root) => self.state.modules.get(root).cloned().ok_or_else(|| {
                CasError::Db(DbError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("base module with root `{root}` not found in database"),
                )))
            })?,
            None => Module::default(),
        };

        let target_func = base_module
            .functions
            .iter()
            .find(|f| f.name == envelope.target_function)
            .ok_or_else(|| {
                CasError::InvalidEnvelope(format!(
                    "target function `{}` not found in base module",
                    envelope.target_function
                ))
            })?;

        let actual_contract_hash = compute_contract_hash(target_func);
        if envelope.contract_hash != actual_contract_hash {
            return Err(CasError::InvalidEnvelope(format!(
                "envelope contract hash `{}` does not match base function contract hash `{}`",
                envelope.contract_hash, actual_contract_hash
            )));
        }

        let derived_module = derive_candidate_module(&base_module, envelope, &parsed_expr)?;

        // 5. 防篡改断言：比对调用方模块与内核推导模块
        if let Some(caller_mod) = caller_module {
            let derived_hash = compute_module_hash(&derived_module);
            let caller_hash = compute_module_hash(caller_mod);
            if derived_hash != caller_hash {
                return Err(CasError::ForgedModule(format!(
                    "caller module hash `{caller_hash}` does not match kernel-derived module `{derived_hash}`"
                )));
            }
        }

        // 6. 内核无锁调用 SMT 定理证明
        let affected_func = derived_module
            .functions
            .iter()
            .find(|f| f.name == envelope.target_function)
            .expect("target function exists in derived module");

        let smt_result = axiom_smt::verify_function_in_module(affected_func, &derived_module);

        let (solver, solver_version, obligations_checked) = match smt_result {
            axiom_smt::SmtResult::Proved {
                solver,
                version,
                obligations_checked,
            } => (solver, version, obligations_checked),
            axiom_smt::SmtResult::VacuousContract { reason } => {
                return Err(CasError::VacuousContract(reason));
            }
            axiom_smt::SmtResult::CounterExample(ce) => {
                return Err(CasError::VerificationRejected(format!(
                    "SMT counterexample found for `{}`: {:?}",
                    envelope.target_function, ce
                )));
            }
            axiom_smt::SmtResult::Sampled { points_checked } => {
                return Err(CasError::UnverifiedCandidate(format!(
                    "candidate for `{}` was only heuristically sampled ({points_checked} points), strict mathematical proof required",
                    envelope.target_function
                )));
            }
            axiom_smt::SmtResult::Unknown(reason) => {
                return Err(CasError::UnverifiedCandidate(format!(
                    "SMT solver returned unknown for `{}`: {reason}",
                    envelope.target_function
                )));
            }
            axiom_smt::SmtResult::Skipped(reason) => {
                return Err(CasError::UnverifiedCandidate(format!(
                    "verification skipped for `{}`: {reason}",
                    envelope.target_function
                )));
            }
        };

        let proof_obligation_hash = compute_proof_obligation_hash(affected_func);

        Ok(CertifiedVerification {
            base_root_hash: expected_root.clone(),
            envelope_hash: env_hash,
            candidate_hash: cand_hash,
            proof_obligation_hash,
            derived_module,
            solver,
            solver_version,
            obligations_checked,
            _sealed: (),
        })
    }

    /// Two-phase commit Phase 2: Atomically applies a certified verification candidate to the CAS database.
    ///
    /// The lock is held ONLY for the microsecond duration of checking the root, verifying the derived
    /// module hash matches deterministic AST transformation, and persisting the new state to disk.
    /// Mathematical theorem proving is NOT performed under lock.
    pub fn try_commit_verified(
        &mut self,
        expected_root: &Option<NodeHash>,
        envelope: TaskEnvelope,
        candidate: Candidate,
        certified: CertifiedVerification,
    ) -> Result<CommitRecord, CasError> {
        let env_hash = compute_envelope_hash(&envelope);
        let cand_hash = compute_candidate_hash(&candidate);

        if certified.envelope_hash != env_hash || certified.candidate_hash != cand_hash {
            return Err(CasError::InvalidCandidate(
                "certified verification does not match envelope or candidate".to_string(),
            ));
        }

        if &certified.base_root_hash != expected_root {
            return Err(CasError::InvalidCandidate(
                "certified verification base_root_hash does not match expected_root".to_string(),
            ));
        }

        // 1. 获取微秒级跨进程排他锁并同步最新磁盘状态
        let _lock = self.lock_and_sync()?;

        let mut next_state = self.state.clone();
        next_state
            .envelopes
            .insert(env_hash.clone(), envelope.clone());
        next_state
            .candidates
            .insert(cand_hash.clone(), candidate.clone());

        // 2. 比较当前 root 是否与 expected_root 匹配 (CAS 门禁)
        if &self.state.root_ref != expected_root {
            let persist_res = self.persist(&next_state);
            if persist_res.is_ok() {
                self.state = next_state;
            }
            return Err(CasError::StaleRoot {
                current: self.state.root_ref.clone(),
                expected: expected_root.clone(),
            });
        }

        // 2.5 锁内极速验证 derived_module 确由 base_module 与 candidate 表达式确定性推导生成（纳秒级操作，防止模块伪造）
        let base_module = match expected_root {
            Some(root_hash) => self.state.modules.get(root_hash).ok_or_else(|| {
                CasError::InvalidEnvelope(format!("expected root module {} not found", root_hash))
            })?,
            None => &Module::default(),
        };

        let parsed_candidate =
            axiom_parser::parse_expr(&candidate.expression_source).map_err(|e| {
                CasError::InvalidCandidate(format!("candidate expression parse failure: {}", e))
            })?;
        let expected_derived = derive_candidate_module(base_module, &envelope, &parsed_candidate)
            .map_err(|e| {
            CasError::InvalidCandidate(format!("commit derivation failure: {}", e))
        })?;
        let expected_derived_hash = compute_module_hash(&expected_derived);
        let actual_derived_hash = compute_module_hash(&certified.derived_module);
        if expected_derived_hash != actual_derived_hash {
            return Err(CasError::ForgedModule(format!(
                "certified module hash `{actual_derived_hash}` does not match kernel deterministic derivation `{expected_derived_hash}`"
            )));
        }

        // 3. 内核铸造防伪证明收据
        let receipt = VerificationReceipt {
            candidate_hash: cand_hash.clone(),
            proof_level: ProofLevel::Proved,
            solver: certified.solver,
            solver_version: certified.solver_version,
            obligations_checked: certified.obligations_checked,
            proof_obligation_hash: certified.proof_obligation_hash,
            verification_profile_hash: current_verification_profile_hash(),
            verified_at: candidate.timestamp,
        };
        let rcpt_hash = compute_receipt_hash(&receipt);
        next_state.receipts.insert(rcpt_hash.clone(), receipt);

        // 4. 录入新模块与生成 CommitRecord
        let mod_record = Self::insert_module_to_state(&mut next_state, &certified.derived_module);
        let new_root = mod_record.module_hash;

        let payload = CommitPayload {
            parent_commit: next_state.head_commit.clone(),
            parent_root: expected_root.clone(),
            new_root: new_root.clone(),
            envelope_hash: env_hash,
            candidate_hash: cand_hash,
            receipt_hash: rcpt_hash,
            timestamp: candidate.timestamp,
        };
        let final_commit = CommitRecord::new(payload);
        next_state
            .commits
            .insert(final_commit.commit_hash.clone(), final_commit.clone());
        next_state.root_ref = Some(new_root);
        next_state.head_commit = Some(final_commit.commit_hash.clone());

        // [Spec 4] 锁内原子化删除已晋升为正式提交的草稿，杜绝幽灵草稿与并发竞态
        next_state.drafts.remove(&envelope.target_hole_id);

        self.persist(&next_state)?;
        self.state = next_state;

        Ok(final_commit)
    }

    /// 针对已存在于当前权威模块中的单洞实现，换发新的认证收据并生成 Re-attestation CommitRecord。
    /// 解决嵌套洞在重验时因函数已填充而无法再次填洞的结构性缺陷。
    pub fn try_commit_reattestation(
        &mut self,
        target_hole_id: &str,
        envelope_hash: &NodeHash,
        candidate_hash: &NodeHash,
        receipt: VerificationReceipt,
    ) -> Result<CommitRecord, CasError> {
        let _guard = self.lock_and_sync()?;
        let mut next_state = self.state.clone();

        let current_root = next_state.root_ref.clone().ok_or_else(|| {
            CasError::InvalidCandidate("cannot reattest candidate in an empty database".into())
        })?;

        // 验证收据合法性
        if receipt.candidate_hash != *candidate_hash {
            return Err(CasError::InvalidReceipt(
                "receipt candidate hash does not match target candidate".into(),
            ));
        }
        if receipt.verification_profile_hash != current_verification_profile_hash() {
            return Err(CasError::InvalidReceipt(
                "receipt profile hash does not match current profile".into(),
            ));
        }

        let rcpt_hash = compute_receipt_hash(&receipt);
        next_state
            .receipts
            .insert(rcpt_hash.clone(), receipt.clone());

        let payload = CommitPayload {
            parent_commit: next_state.head_commit.clone(),
            parent_root: Some(current_root.clone()),
            new_root: current_root,
            envelope_hash: envelope_hash.clone(),
            candidate_hash: candidate_hash.clone(),
            receipt_hash: rcpt_hash,
            timestamp: receipt.verified_at,
        };
        let final_commit = CommitRecord::new(payload);
        next_state
            .commits
            .insert(final_commit.commit_hash.clone(), final_commit.clone());
        next_state.head_commit = Some(final_commit.commit_hash.clone());

        // 原子清理该洞可能残留的草稿
        next_state.drafts.remove(target_hole_id);

        self.persist(&next_state)?;
        self.state = next_state;

        Ok(final_commit)
    }

    /// Convenience atomic CAS combining offline verification and locked commit.
    pub fn try_commit_candidate(
        &mut self,
        expected_root: &Option<NodeHash>,
        envelope: TaskEnvelope,
        candidate: Candidate,
        caller_module: Option<&Module>,
    ) -> Result<CommitRecord, CasError> {
        let certified =
            self.verify_candidate_offline(expected_root, &envelope, &candidate, caller_module)?;
        self.try_commit_verified(expected_root, envelope, candidate, certified)
    }

    /// Returns a reference to all committed records.
    pub fn commits(&self) -> &BTreeMap<NodeHash, CommitRecord> {
        &self.state.commits
    }

    /// Returns a reference to all verification receipts.
    pub fn receipts(&self) -> &BTreeMap<NodeHash, VerificationReceipt> {
        &self.state.receipts
    }

    /// Returns a reference to all task envelopes.
    pub fn envelopes(&self) -> &BTreeMap<NodeHash, TaskEnvelope> {
        &self.state.envelopes
    }

    /// Returns a reference to all candidates.
    pub fn candidates(&self) -> &BTreeMap<NodeHash, Candidate> {
        &self.state.candidates
    }

    /// Retrieves a function by its content identifier.
    pub fn get_function(&self, hash: &NodeHash) -> Result<Option<FunctionDecl>, DbError> {
        Ok(self.state.functions.get(hash).cloned())
    }

    /// Retrieves a type definition by its content identifier.
    pub fn get_type(&self, hash: &NodeHash) -> Result<Option<TypeDef>, DbError> {
        Ok(self.state.types.get(hash).cloned())
    }

    /// Retrieves an effect declaration by its content identifier.
    pub fn get_effect(&self, hash: &NodeHash) -> Result<Option<EffectDecl>, DbError> {
        Ok(self.state.effects.get(hash).cloned())
    }

    /// Retrieves a module by its content identifier.
    pub fn get_module(&self, hash: &NodeHash) -> Result<Option<Module>, DbError> {
        Ok(self.state.modules.get(hash).cloned())
    }

    /// Retrieves a task envelope by its content identifier.
    pub fn get_envelope(&self, hash: &NodeHash) -> Result<Option<TaskEnvelope>, DbError> {
        Ok(self.state.envelopes.get(hash).cloned())
    }

    /// Retrieves a candidate by its content identifier.
    pub fn get_candidate(&self, hash: &NodeHash) -> Result<Option<Candidate>, DbError> {
        Ok(self.state.candidates.get(hash).cloned())
    }

    /// Retrieves a verification receipt by its content identifier.
    pub fn get_receipt(&self, hash: &NodeHash) -> Result<Option<VerificationReceipt>, DbError> {
        Ok(self.state.receipts.get(hash).cloned())
    }

    /// Retrieves a commit record by its content identifier.
    pub fn get_commit(&self, hash: &NodeHash) -> Result<Option<CommitRecord>, DbError> {
        Ok(self.state.commits.get(hash).cloned())
    }

    /// Returns all provisional drafts currently stored in the database.
    pub fn drafts(&self) -> &BTreeMap<String, DraftRecord> {
        &self.state.drafts
    }

    /// Retrieves a provisional draft by hole ID.
    pub fn get_draft(&self, hole_id: &str) -> Option<&DraftRecord> {
        self.state.drafts.get(hole_id)
    }

    /// Inserts or updates a provisional draft with CAS validation on expected revision and base root.
    pub fn insert_draft_cas(
        &mut self,
        mut draft: DraftRecord,
        expected_revision: Option<u64>,
        expected_base_root: Option<&NodeHash>,
    ) -> Result<DraftRecord, CasError> {
        let _guard = self.lock_and_sync()?;
        let mut next_state = self.state.clone();

        // 1. 严格检查 base_root 匹配，防止并发基于陈旧 baseline 覆写
        if let Some(exp_root) = expected_base_root {
            let current_sys_root = next_state.root_ref.as_ref();
            if current_sys_root != Some(exp_root) {
                return Err(CasError::StaleRootMismatch {
                    hole_id: draft.hole_id.clone(),
                    current: current_sys_root.cloned(),
                    expected: Some(exp_root.clone()),
                });
            }
        }

        // 2. 严格检查 revision 版本匹配
        if let Some(existing) = next_state.drafts.get(&draft.hole_id) {
            if let Some(exp_rev) = expected_revision {
                if existing.revision != exp_rev {
                    return Err(CasError::StaleDraftRevision {
                        hole_id: draft.hole_id,
                        current: existing.revision,
                        expected: exp_rev,
                    });
                }
            }
            draft.revision = existing.revision + 1;
        } else {
            if let Some(exp_rev) = expected_revision {
                if exp_rev != 0 {
                    return Err(CasError::StaleDraftRevision {
                        hole_id: draft.hole_id,
                        current: 0,
                        expected: exp_rev,
                    });
                }
            }
            draft.revision = 1;
        }

        if draft.base_root.is_none() {
            draft.base_root = next_state.root_ref.clone();
        }

        next_state
            .drafts
            .insert(draft.hole_id.clone(), draft.clone());
        self.persist(&next_state)?;
        self.state = next_state;
        Ok(draft)
    }

    /// Convenience wrapper around insert_draft_cas without revision constraints.
    pub fn insert_draft(&mut self, draft: DraftRecord) -> Result<DraftRecord, CasError> {
        self.insert_draft_cas(draft, None, None)
    }

    /// Removes a provisional draft for a hole (e.g. upon promotion to verified commit).
    pub fn remove_draft(&mut self, hole_id: &str) -> Result<Option<DraftRecord>, DbError> {
        let _guard = self.lock_and_sync()?;
        let mut next_state = self.state.clone();
        let removed = next_state.drafts.remove(hole_id);
        if removed.is_some() {
            self.persist(&next_state)?;
            self.state = next_state;
        }
        Ok(removed)
    }

    /// Retrieves all candidates associated with a specific task envelope hash.
    pub fn get_candidates_for_envelope(&self, env_hash: &NodeHash) -> Vec<Candidate> {
        self.state
            .candidates
            .values()
            .filter(|c| &c.envelope_hash == env_hash)
            .cloned()
            .collect()
    }

    fn persist(&self, state: &DatabaseState) -> Result<(), DbError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(state)?;
        let tmp_path = path.with_extension(format!(
            "tmp_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        atomic_replace_file(&tmp_path, path)?;
        Ok(())
    }
}

/// RAII cross-process exclusive OS file lock guard.
#[derive(Debug)]
pub struct FileLockGuard {
    #[allow(dead_code)]
    lock_path: PathBuf,
    file: Option<fs::File>,
}

impl FileLockGuard {
    /// Attempts to acquire an OS-level exclusive handle lock on the target lock file with timeout.
    pub fn acquire(target_path: &Path) -> Result<Self, DbError> {
        let lock_path = target_path.with_extension("lock");
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(5);
        let mut backoff = std::time::Duration::from_millis(5);

        loop {
            match Self::try_open_exclusive(&lock_path) {
                Ok(file) => {
                    return Ok(Self {
                        lock_path,
                        file: Some(file),
                    })
                }
                Err(e) if Self::is_contended(&e) => {
                    if start.elapsed() > timeout {
                        return Err(DbError::Io(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!(
                                "timed out waiting for exclusive OS file lock on `{}`",
                                lock_path.display()
                            ),
                        )));
                    }
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(std::time::Duration::from_millis(50));
                }
                Err(e) => return Err(DbError::Io(e)),
            }
        }
    }

    #[cfg(windows)]
    fn try_open_exclusive(path: &Path) -> std::io::Result<fs::File> {
        use std::os::windows::fs::OpenOptionsExt;
        // share_mode(0): Exclusive access. Other processes attempting to open will fail with sharing violation.
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .share_mode(0)
            .open(path)
    }

    #[cfg(unix)]
    fn try_open_exclusive(path: &Path) -> std::io::Result<fs::File> {
        use std::os::unix::io::AsRawFd;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(file)
    }

    #[cfg(not(any(windows, unix)))]
    fn try_open_exclusive(path: &Path) -> std::io::Result<fs::File> {
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
    }

    fn is_contended(err: &std::io::Error) -> bool {
        #[cfg(windows)]
        {
            // Windows error code 32: ERROR_SHARING_VIOLATION
            err.raw_os_error() == Some(32)
                || err.kind() == std::io::ErrorKind::PermissionDenied
                || err.kind() == std::io::ErrorKind::AlreadyExists
        }
        #[cfg(unix)]
        {
            err.raw_os_error() == Some(libc::EWOULDBLOCK)
                || err.raw_os_error() == Some(libc::EAGAIN)
                || err.kind() == std::io::ErrorKind::WouldBlock
        }
        #[cfg(not(any(windows, unix)))]
        {
            err.kind() == std::io::ErrorKind::AlreadyExists
                || err.kind() == std::io::ErrorKind::WouldBlock
        }
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        // Drop the file handle to release OS kernel lock (Windows share_mode / Unix flock).
        // The lock file is permanently retained to preserve the single inode and prevent
        // double-locking race conditions between unlink and concurrent re-creation.
        self.file.take();
    }
}

#[cfg(windows)]
fn atomic_replace_file(from: &Path, to: &Path) -> Result<(), DbError> {
    use std::os::windows::ffi::OsStrExt;
    let from_wide: Vec<u16> = from
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let to_wide: Vec<u16> = to
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    extern "system" {
        fn MoveFileExW(
            lpExistingFileName: *const u16,
            lpNewFileName: *const u16,
            dwFlags: u32,
        ) -> i32;
    }

    if !to.exists() {
        fs::rename(from, to)?;
        return Ok(());
    }

    let mut last_err = None;
    for _ in 0..10 {
        let ret = unsafe {
            MoveFileExW(
                from_wide.as_ptr(),
                to_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if ret != 0 {
            return Ok(());
        }
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(5) || err.raw_os_error() == Some(32) {
            std::thread::sleep(std::time::Duration::from_millis(15));
            last_err = Some(err);
            continue;
        }
        let _ = fs::remove_file(from);
        return Err(DbError::Io(err));
    }
    let _ = fs::remove_file(from);
    Err(DbError::Io(
        last_err.unwrap_or_else(std::io::Error::last_os_error),
    ))
}

#[cfg(not(windows))]
fn atomic_replace_file(from: &Path, to: &Path) -> Result<(), DbError> {
    fs::rename(from, to)?;
    Ok(())
}

/// Replaces occurrences of a hole identifier with a replacement expression within an AST expression.
pub fn replace_hole_in_expr(expression: &mut Expr, hole_id: &str, solution: &Expr) -> bool {
    match expression {
        Expr::Hole(current_hole_id) if current_hole_id == hole_id => {
            *expression = solution.clone();
            true
        }
        Expr::BinaryOp { left, right, .. } => {
            replace_hole_in_expr(left, hole_id, solution)
                | replace_hole_in_expr(right, hole_id, solution)
        }
        Expr::UnaryOp { operand, .. } | Expr::Let { value: operand, .. } => {
            replace_hole_in_expr(operand, hole_id, solution)
        }
        Expr::Block { statements, tail } => {
            let statements_changed = statements.iter_mut().fold(false, |changed, statement| {
                replace_hole_in_expr(statement, hole_id, solution) | changed
            });
            let tail_changed = tail
                .as_mut()
                .is_some_and(|tail| replace_hole_in_expr(tail, hole_id, solution));
            statements_changed | tail_changed
        }
        Expr::Call { callee, args } => {
            let callee_changed = replace_hole_in_expr(callee, hole_id, solution);
            let args_changed = args.iter_mut().fold(false, |changed, argument| {
                replace_hole_in_expr(argument, hole_id, solution) | changed
            });
            callee_changed | args_changed
        }
        Expr::FieldAccess { target, .. } => replace_hole_in_expr(target, hole_id, solution),
        Expr::RecordInit { fields, .. } => fields.iter_mut().fold(false, |changed, (_, value)| {
            replace_hole_in_expr(value, hole_id, solution) | changed
        }),
        Expr::RecordUpdate { base, updates } => {
            let base_changed = replace_hole_in_expr(base, hole_id, solution);
            let updates_changed = updates.iter_mut().fold(false, |changed, (_, value)| {
                replace_hole_in_expr(value, hole_id, solution) | changed
            });
            base_changed | updates_changed
        }
        Expr::Match { target, arms } => {
            let target_changed = replace_hole_in_expr(target, hole_id, solution);
            let arms_changed = arms.iter_mut().fold(false, |changed, arm| {
                replace_hole_in_expr(&mut arm.body, hole_id, solution) | changed
            });
            target_changed | arms_changed
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) | Expr::Raw(_) => false,
    }
}

impl Default for AstDatabase {
    fn default() -> Self {
        Self::in_memory()
    }
}
