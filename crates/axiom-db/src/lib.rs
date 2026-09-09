//! Content-addressed storage and Verifiable Change Protocol for Axiom abstract syntax trees.

use axiom_ast::{EffectDecl, Expr, FunctionDecl, Module, TypeDef};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
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
            "#type_", "#fn_", "#effect_", "#mod_", "#env_", "#cand_", "#rcpt_", "#commit_", "#null_",
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

/// Cryptographically signed verification receipt for an accepted candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReceipt {
    pub candidate_hash: NodeHash,
    pub proof_level: ProofLevel,
    pub solver: String,
    pub solver_version: String,
    pub obligations_checked: usize,
    pub proof_obligation_hash: NodeHash,
    pub verified_at: u64,
}

/// Computes content identifier for a verification receipt.
pub fn compute_receipt_hash(rcpt: &VerificationReceipt) -> NodeHash {
    compute_hash("rcpt", rcpt)
}

/// An immutable commit record linking parent root, new root, envelope, candidate, and receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitRecord {
    pub commit_hash: NodeHash,
    pub parent_root: Option<NodeHash>,
    pub new_root: NodeHash,
    pub envelope_hash: NodeHash,
    pub candidate_hash: NodeHash,
    pub receipt_hash: NodeHash,
    pub timestamp: u64,
}

/// Computes content identifier for a commit record.
pub fn compute_commit_hash(commit: &CommitRecord) -> NodeHash {
    compute_hash("commit", commit)
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
}

impl fmt::Display for DbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "AST database I/O error: {error}"),
            Self::Serialization(error) => {
                write!(formatter, "invalid AST database serialization: {error}")
            }
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Serialization(error) => Some(error),
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
    UnverifiedCandidate(String),
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
            Self::InvalidReceipt(msg) => write!(f, "CAS rejected: invalid verification receipt: {msg}"),
            Self::UnverifiedCandidate(msg) => write!(f, "CAS rejected: candidate does not meet proof bar: {msg}"),
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DatabaseState {
    #[serde(default)]
    root_ref: Option<NodeHash>,
    #[serde(default)]
    types: BTreeMap<NodeHash, TypeDef>,
    #[serde(default)]
    functions: BTreeMap<NodeHash, FunctionDecl>,
    #[serde(default)]
    effects: BTreeMap<NodeHash, EffectDecl>,
    #[serde(default)]
    modules: BTreeMap<NodeHash, Module>,
    #[serde(default)]
    envelopes: BTreeMap<NodeHash, TaskEnvelope>,
    #[serde(default)]
    candidates: BTreeMap<NodeHash, Candidate>,
    #[serde(default)]
    receipts: BTreeMap<NodeHash, VerificationReceipt>,
    #[serde(default)]
    commits: BTreeMap<NodeHash, CommitRecord>,
}

/// A content-addressed AST database backed by a JSON file or memory.
#[derive(Debug, Clone)]
pub struct AstDatabase {
    path: Option<PathBuf>,
    state: DatabaseState,
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

    /// Sets the root module hash reference directly.
    pub fn set_root(&mut self, root: NodeHash) -> Result<(), DbError> {
        let mut next_state = self.state.clone();
        next_state.root_ref = Some(root);
        self.persist(&next_state)?;
        self.state = next_state;
        Ok(())
    }

    fn insert_module_to_state(&self, state: &mut DatabaseState, module: &Module) -> ModuleRecord {
        let type_hashes = module.types.iter().map(compute_type_hash).collect::<Vec<_>>();
        let function_hashes = module.functions.iter().map(compute_function_hash).collect::<Vec<_>>();
        let effect_hashes = module.effects.iter().map(compute_effect_hash).collect::<Vec<_>>();
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

    /// Inserts a module and every type, function, and effect it owns.
    pub fn insert_module(&mut self, module: &Module) -> Result<ModuleRecord, DbError> {
        let mut next_state = self.state.clone();
        let record = self.insert_module_to_state(&mut next_state, module);
        if next_state.root_ref.is_none() {
            next_state.root_ref = Some(record.module_hash.clone());
        }
        self.persist(&next_state)?;
        self.state = next_state;
        Ok(record)
    }

    /// Records a task envelope into the database.
    pub fn record_envelope(&mut self, envelope: TaskEnvelope) -> Result<NodeHash, DbError> {
        let hash = compute_envelope_hash(&envelope);
        let mut next_state = self.state.clone();
        next_state.envelopes.insert(hash.clone(), envelope);
        self.persist(&next_state)?;
        self.state = next_state;
        Ok(hash)
    }

    /// Records a candidate implementation without committing it to the root ref.
    pub fn record_candidate(&mut self, candidate: Candidate) -> Result<NodeHash, DbError> {
        let hash = compute_candidate_hash(&candidate);
        let mut next_state = self.state.clone();
        next_state.candidates.insert(hash.clone(), candidate);
        self.persist(&next_state)?;
        self.state = next_state;
        Ok(hash)
    }

    /// Records a verification receipt for audit and compliance.
    pub fn record_receipt(&mut self, receipt: VerificationReceipt) -> Result<NodeHash, DbError> {
        let hash = compute_receipt_hash(&receipt);
        let mut next_state = self.state.clone();
        next_state.receipts.insert(hash.clone(), receipt);
        self.persist(&next_state)?;
        self.state = next_state;
        Ok(hash)
    }

    /// Attempts an atomic CAS (Compare-And-Swap) commit of a verified candidate.
    ///
    /// Preserves all envelopes, candidates, and receipts regardless of CAS success or failure.
    pub fn try_commit_candidate(
        &mut self,
        expected_root: &Option<NodeHash>,
        envelope: TaskEnvelope,
        candidate: Candidate,
        receipt: VerificationReceipt,
        new_module: &Module,
    ) -> Result<CommitRecord, CasError> {
        let cand_hash = compute_candidate_hash(&candidate);
        if receipt.candidate_hash != cand_hash {
            return Err(CasError::InvalidReceipt(format!(
                "receipt candidate hash `{}` does not match computed `{}`",
                receipt.candidate_hash, cand_hash
            )));
        }

        if matches!(receipt.proof_level, ProofLevel::Sampled | ProofLevel::Unknown | ProofLevel::Unverified) {
            return Err(CasError::UnverifiedCandidate(format!(
                "candidate proof level `{:?}` cannot be committed to root ref",
                receipt.proof_level
            )));
        }

        let env_hash = compute_envelope_hash(&envelope);
        let rcpt_hash = compute_receipt_hash(&receipt);

        let mut next_state = self.state.clone();
        next_state.envelopes.insert(env_hash.clone(), envelope);
        next_state.candidates.insert(cand_hash.clone(), candidate.clone());
        next_state.receipts.insert(rcpt_hash.clone(), receipt);

        if &self.state.root_ref != expected_root {
            let _ = self.persist(&next_state);
            self.state = next_state;
            return Err(CasError::StaleRoot {
                current: self.state.root_ref.clone(),
                expected: expected_root.clone(),
            });
        }

        let mod_record = self.insert_module_to_state(&mut next_state, new_module);
        let new_root = mod_record.module_hash;

        let commit_proto = CommitRecord {
            commit_hash: NodeHash::default(),
            parent_root: expected_root.clone(),
            new_root: new_root.clone(),
            envelope_hash: env_hash,
            candidate_hash: cand_hash,
            receipt_hash: rcpt_hash,
            timestamp: candidate.timestamp,
        };
        let commit_hash = compute_commit_hash(&commit_proto);
        let final_commit = CommitRecord {
            commit_hash: commit_hash.clone(),
            ..commit_proto
        };
        next_state.commits.insert(commit_hash, final_commit.clone());
        next_state.root_ref = Some(new_root);

        self.persist(&next_state)?;
        self.state = next_state;

        Ok(final_commit)
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
        fs::write(path, bytes)?;
        Ok(())
    }
}

impl Default for AstDatabase {
    fn default() -> Self {
        Self::in_memory()
    }
}
