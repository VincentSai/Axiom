//! Content-addressed storage and Verifiable Change Protocol for Axiom abstract syntax trees.

pub(crate) mod commit_graph;

use axiom_ast::{EffectDecl, Expr, FunctionDecl, FunctionParam, Module, Type, TypeDef};
use commit_graph::CommitGraph;
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

pub fn compute_hash<T: Serialize>(prefix: &str, value: &T) -> NodeHash {
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
/// typed IR environment, decimal scale guards, fail-closed float semantics, and solver requirements.
pub const CURRENT_VERIFICATION_PROFILE: &str =
    "axiom-smt-v5-typed-ir-decimal-scale-guard-failclosed-float";

/// Current database schema version (Version 5 incorporates typed lexical IR & decimal scale guards).
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solver_sha256: Option<String>,
    pub obligations_checked: usize,
    pub proof_obligation_hash: NodeHash,
    #[serde(default = "legacy_verification_profile_hash")]
    pub verification_profile_hash: NodeHash,
    #[serde(default)]
    pub attested_commit_hash: Option<NodeHash>,
    pub verified_at: u64,
}

impl VerificationReceipt {
    /// 检查该证明收据在当前规则下是否属于有效证明：
    /// 1. 证明等级必须为 Proved；
    /// 2. 证明规范 Profile 哈希必须与当前最新系统 current_profile_hash 严格一致；
    /// 3. 必须包含合法的 solver_sha256（64位小写十六进制字符）；
    /// 4. 求解器版本 solver_version 必须已知（非空且非 unknown）。
    pub fn is_currently_valid_proved(&self, current_profile_hash: &NodeHash) -> bool {
        if self.proof_level != ProofLevel::Proved {
            return false;
        }
        if self.verification_profile_hash != *current_profile_hash {
            return false;
        }
        let has_valid_sha = match &self.solver_sha256 {
            Some(sha) => sha.len() == 64 && sha.chars().all(|c| c.is_ascii_hexdigit()),
            None => false,
        };
        let has_known_version = !self.solver_version.trim().is_empty()
            && !self.solver_version.trim().eq_ignore_ascii_case("unknown");
        has_valid_sha && has_known_version
    }
}

/// Computes content identifier for a verification receipt.
pub fn compute_receipt_hash(rcpt: &VerificationReceipt) -> NodeHash {
    compute_hash("rcpt", rcpt)
}

/// Legacy V3 verification receipt (9 fields, without solver_sha256) used in Schema V3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyV3VerificationReceipt {
    pub(crate) candidate_hash: NodeHash,
    pub(crate) proof_level: ProofLevel,
    pub(crate) solver: String,
    pub(crate) solver_version: String,
    pub(crate) obligations_checked: usize,
    pub(crate) proof_obligation_hash: NodeHash,
    #[serde(default = "legacy_verification_profile_hash")]
    pub(crate) verification_profile_hash: NodeHash,
    #[serde(default)]
    pub(crate) attested_commit_hash: Option<NodeHash>,
    pub(crate) verified_at: u64,
}

/// Computes content identifier for a legacy V3 verification receipt.
pub(crate) fn compute_legacy_v3_receipt_hash(rcpt: &LegacyV3VerificationReceipt) -> NodeHash {
    compute_hash("rcpt", rcpt)
}

/// Legacy V2 verification receipt (8 fields, lacking attested_commit_hash) used in Commit 4492184.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyV2VerificationReceipt {
    pub(crate) candidate_hash: NodeHash,
    pub(crate) proof_level: ProofLevel,
    pub(crate) solver: String,
    pub(crate) solver_version: String,
    pub(crate) obligations_checked: usize,
    pub(crate) proof_obligation_hash: NodeHash,
    #[serde(default = "legacy_verification_profile_hash")]
    pub(crate) verification_profile_hash: NodeHash,
    pub(crate) verified_at: u64,
}

/// Computes content identifier for a legacy V2 verification receipt.
pub(crate) fn compute_legacy_v2_receipt_hash(rcpt: &LegacyV2VerificationReceipt) -> NodeHash {
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

/// Typed CAS expectation for inserting or updating a provisional draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftCasExpectation {
    pub expected_revision: Option<u64>,
    pub expected_base_root: Option<NodeHash>,
}

impl DraftCasExpectation {
    /// Initial draft insertion expectation: draft MUST NOT exist (revision 0).
    pub fn initial(base_root: Option<NodeHash>) -> Self {
        Self {
            expected_revision: Some(0),
            expected_base_root: base_root,
        }
    }

    /// Existing draft update expectation: draft must be at `revision`.
    pub fn update(revision: u64, base_root: Option<NodeHash>) -> Self {
        Self {
            expected_revision: Some(revision),
            expected_base_root: base_root,
        }
    }

    /// Unconstrained expectation (disables CAS validation checks).
    pub fn unconstrained() -> Self {
        Self {
            expected_revision: None,
            expected_base_root: None,
        }
    }
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
    #[serde(default)]
    pub attested_commit_hash: Option<NodeHash>,
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

    /// Access optional attested commit hash (for receipt-only no-op commits).
    pub fn attested_commit_hash(&self) -> Option<&NodeHash> {
        self.payload.attested_commit_hash.as_ref()
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

/// Legacy commit payload (6 fields) used in pre-ancestry DAG database schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyCommitPayload {
    pub(crate) parent_root: Option<NodeHash>,
    pub(crate) new_root: NodeHash,
    pub(crate) envelope_hash: NodeHash,
    pub(crate) candidate_hash: NodeHash,
    pub(crate) receipt_hash: NodeHash,
    pub(crate) timestamp: u64,
}

/// Computes content identifier for a legacy commit payload.
pub(crate) fn compute_legacy_commit_payload_hash(payload: &LegacyCommitPayload) -> NodeHash {
    compute_hash("commit", payload)
}

/// Legacy V2 commit payload (7 fields, lacking attested_commit_hash) used in Commit 4492184.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyV2CommitPayload {
    #[serde(default)]
    pub(crate) parent_commit: Option<NodeHash>,
    pub(crate) parent_root: Option<NodeHash>,
    pub(crate) new_root: NodeHash,
    pub(crate) envelope_hash: NodeHash,
    pub(crate) candidate_hash: NodeHash,
    pub(crate) receipt_hash: NodeHash,
    pub(crate) timestamp: u64,
}

/// Computes content identifier for a legacy V2 commit payload.
pub(crate) fn compute_legacy_v2_commit_payload_hash(payload: &LegacyV2CommitPayload) -> NodeHash {
    compute_hash("commit", payload)
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
    IncompatibleSchemaVersion { found: u32, current: u32 },
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
            Self::IncompatibleSchemaVersion { found, current } => {
                write!(
                    formatter,
                    "incompatible AST database schema version {found}, current supported maximum version is {current}"
                )
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
            Self::IncompatibleSchemaVersion { .. } => None,
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
    StaleHead {
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
            Self::StaleHead { current, expected } => write!(
                f,
                "CAS rejected: stale head commit ref (expected `{:?}`, current is `{:?}`)",
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

/// Kernel-minted cryptographic proof evidence computed offline by the SMT engine.
/// This evidence is sealed and can only be constructed within the axiom-db kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertifiedProofEvidence {
    pub(crate) envelope_hash: NodeHash,
    pub(crate) candidate_hash: NodeHash,
    pub(crate) proof_obligation_hash: NodeHash,
    pub(crate) solver: String,
    pub(crate) solver_version: String,
    pub(crate) solver_sha256: Option<String>,
    pub(crate) obligations_checked: usize,
    pub(crate) _sealed: (),
}

impl CertifiedProofEvidence {
    pub fn envelope_hash(&self) -> &NodeHash {
        &self.envelope_hash
    }
    pub fn candidate_hash(&self) -> &NodeHash {
        &self.candidate_hash
    }
    pub fn proof_obligation_hash(&self) -> &NodeHash {
        &self.proof_obligation_hash
    }
    pub fn solver(&self) -> &str {
        &self.solver
    }
    pub fn solver_version(&self) -> &str {
        &self.solver_version
    }
    pub fn solver_sha256(&self) -> Option<&str> {
        self.solver_sha256.as_deref()
    }
    pub fn obligations_checked(&self) -> usize {
        self.obligations_checked
    }
}

/// Unified SMT theorem proving validator constructing sealed CertifiedProofEvidence or returning CasError.
fn require_proved(
    smt_result: axiom_smt::SmtResult,
    envelope_hash: NodeHash,
    candidate_hash: NodeHash,
    target_func: &FunctionDecl,
) -> Result<CertifiedProofEvidence, CasError> {
    let target_name = &target_func.name;
    match smt_result {
        axiom_smt::SmtResult::Proved {
            solver,
            version,
            solver_sha256,
            obligations_checked,
            ..
        } => {
            // [P1 Spec 3 核心闭环]: Fail-Closed 校验求解器版本与 64 位 SHA-256 哈希
            let trimmed_ver = version.trim();
            if trimmed_ver.is_empty() || trimmed_ver.eq_ignore_ascii_case("unknown") {
                return Err(CasError::UnverifiedCandidate(format!(
                    "SMT solver version for `{target_name}` must be known and non-empty, found: `{version}`"
                )));
            }
            let sha = match &solver_sha256 {
                Some(s) if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) => s.clone(),
                Some(invalid_sha) => {
                    return Err(CasError::UnverifiedCandidate(format!(
                        "SMT solver sha256 for `{target_name}` is invalid format (must be 64-character hex): `{invalid_sha}`"
                    )));
                }
                None => {
                    return Err(CasError::UnverifiedCandidate(format!(
                        "SMT solver sha256 for `{target_name}` is missing, strict provenance attestation required"
                    )));
                }
            };

            let proof_obligation_hash = compute_proof_obligation_hash(target_func);
            Ok(CertifiedProofEvidence {
                envelope_hash,
                candidate_hash,
                proof_obligation_hash,
                solver,
                solver_version: version,
                solver_sha256: Some(sha),
                obligations_checked,
                _sealed: (),
            })
        }
        axiom_smt::SmtResult::VacuousContract { reason } => {
            Err(CasError::VacuousContract(reason))
        }
        axiom_smt::SmtResult::CounterExample(ce) => {
            Err(CasError::VerificationRejected(format!(
                "SMT counterexample found for `{target_name}`: {:?}",
                ce
            )))
        }
        axiom_smt::SmtResult::Sampled { points_checked } => {
            Err(CasError::UnverifiedCandidate(format!(
                "candidate for `{target_name}` was only heuristically sampled ({points_checked} points), strict mathematical proof required"
            )))
        }
        axiom_smt::SmtResult::Unknown(reason) => {
            Err(CasError::UnverifiedCandidate(format!(
                "SMT solver returned unknown for `{target_name}`: {reason}"
            )))
        }
        axiom_smt::SmtResult::Skipped(reason) => {
            Err(CasError::UnverifiedCandidate(format!(
                "verification skipped for `{target_name}`: {reason}"
            )))
        }
    }
}

/// Kernel-minted cryptographic verification certificate computed offline without database locks.
/// This certificate is sealed: it cannot be constructed from outside axiom-db or deserialized arbitrarily.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedVerification {
    pub(crate) base_root_hash: Option<NodeHash>,
    pub(crate) derived_module: Module,
    pub(crate) evidence: CertifiedProofEvidence,
}

impl CertifiedVerification {
    pub fn base_root_hash(&self) -> Option<&NodeHash> {
        self.base_root_hash.as_ref()
    }
    pub fn derived_module(&self) -> &Module {
        &self.derived_module
    }
    pub fn evidence(&self) -> &CertifiedProofEvidence {
        &self.evidence
    }
}

/// Represents the tip of a commit branch, encapsulating both AST root hash and DAG commit hash.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitTip {
    pub root: Option<NodeHash>,
    pub head: Option<NodeHash>,
}

impl CommitTip {
    pub fn new(root: Option<NodeHash>, head: Option<NodeHash>) -> Self {
        Self { root, head }
    }
}

/// Kernel-minted cryptographic reattestation certificate computed offline without database locks.
/// This certificate is sealed: it cannot be constructed from outside axiom-db or deserialized arbitrarily.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedReattestation {
    pub(crate) expected_tip: CommitTip,
    pub(crate) target_hole_id: String,
    pub(crate) attested_commit_hash: NodeHash,
    pub(crate) evidence: CertifiedProofEvidence,
}

impl CertifiedReattestation {
    pub fn expected_tip(&self) -> &CommitTip {
        &self.expected_tip
    }
    pub fn expected_root(&self) -> Option<&NodeHash> {
        self.expected_tip.root.as_ref()
    }
    pub fn expected_head(&self) -> Option<&NodeHash> {
        self.expected_tip.head.as_ref()
    }
    pub fn target_hole_id(&self) -> &str {
        &self.target_hole_id
    }
    pub fn attested_commit_hash(&self) -> &NodeHash {
        &self.attested_commit_hash
    }
    pub fn evidence(&self) -> &CertifiedProofEvidence {
        &self.evidence
    }
}

/// Internal storage state of the AST database, accessible within transactions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct DatabaseState {
    #[serde(default)]
    pub(crate) schema_version: Option<u32>,
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
    /// 针对旧版本数据库执行自愈迁移：
    /// 1. V1 数据库（缺少 head_commit）：重建 parent_commit 祖先拓扑链；
    /// 2. V2 数据库（Commit 4492184，缺少 attested_commit_hash）：防洗白验真原始哈希，
    ///    为 no-op 提交与收据自底向上补齐 landing commit 强绑定，重新计算全链哈希；
    /// 3. V3 数据库（无 solver_sha256）：通过 LegacyV3 验真原始收据与提交哈希，自愈迁移升级至 Schema V4。
    pub(crate) fn migrate_legacy_commits_if_needed(
        state: &mut DatabaseState,
    ) -> Result<bool, DbError> {
        let mut mutated = false;
        let ver = state.schema_version.unwrap_or(0);
        if ver > CURRENT_SCHEMA_VERSION {
            return Err(DbError::IncompatibleSchemaVersion {
                found: ver,
                current: CURRENT_SCHEMA_VERSION,
            });
        }
        if ver < CURRENT_SCHEMA_VERSION {
            if !state.commits.is_empty() {
                if state.head_commit.is_none() {
                    CommitGraph::repair_legacy_ancestry(state)?;
                }
                CommitGraph::repair_v2_migration(state)?;
            }
            state.schema_version = Some(CURRENT_SCHEMA_VERSION);
            mutated = true;
        }
        Ok(mutated)
    }

    /// 统一的反序列化、版本兼容性检查、自愈迁移升级与主链因果验真入口。
    /// 保证首次打开（open）与跨进程并发同步（lock_and_sync）使用完全一致的状态加载与版本协商规范。
    pub(crate) fn load_validate_and_migrate_state(
        bytes: &[u8],
    ) -> Result<(DatabaseState, bool), DbError> {
        let mut state: DatabaseState = serde_json::from_slice(bytes)?;
        let migrated = Self::migrate_legacy_commits_if_needed(&mut state)?;
        CommitGraph::validate_mainline(&state)?;
        Ok((state, migrated))
    }

    /// Opens an existing database file or creates an empty one at `path`.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if path.exists() {
            let bytes = fs::read(path)?;
            let (state, migrated) = Self::load_validate_and_migrate_state(&bytes)?;
            let database = Self {
                path: Some(path.to_path_buf()),
                state,
            };
            if migrated {
                database.persist(&database.state)?;
            }
            Ok(database)
        } else {
            let init_state = DatabaseState {
                schema_version: Some(CURRENT_SCHEMA_VERSION),
                ..Default::default()
            };
            let database = Self {
                path: Some(path.to_path_buf()),
                state: init_state,
            };
            database.persist(&database.state)?;
            Ok(database)
        }
    }

    /// Creates an empty database without a persistent backing file.
    pub fn in_memory() -> Self {
        let init_state = DatabaseState {
            schema_version: Some(CURRENT_SCHEMA_VERSION),
            ..Default::default()
        };
        Self {
            path: None,
            state: init_state,
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

    /// Returns the current commit tip encapsulating both root and head references.
    pub fn commit_tip(&self) -> CommitTip {
        CommitTip {
            root: self.state.root_ref.clone(),
            head: self.state.head_commit.clone(),
        }
    }

    /// Returns the verified mainline commits sequence ordered from earliest ancestor to latest head.
    /// Performs deep integrity validation, artifact existence and cross-binding checks, and self-loop / cycle detection.
    pub fn mainline_commits(&self) -> Result<Vec<&CommitRecord>, DbError> {
        CommitGraph::validate_mainline(&self.state)
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
                let (disk_state, migrated) = Self::load_validate_and_migrate_state(&bytes)?;
                if migrated {
                    self.persist(&disk_state)?;
                }
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
        let evidence = require_proved(smt_result, env_hash, cand_hash, affected_func)?;

        // [Ticket 18 / Review Spec 3]: 统一级联验证内核：
        // 检查下游传递闭包，确保已完工的下游函数没有被破坏
        let cascade_report = axiom_smt::cascade::verify_dependent_closure(
            &derived_module,
            &[&envelope.target_function],
        );
        if cascade_report.has_failures() {
            let reasons: Vec<String> = cascade_report
                .failures
                .iter()
                .map(|f| format!("{}: {}", f.function_name, f.reason))
                .collect();
            return Err(CasError::VerificationRejected(format!(
                "downstream contract violated by upstream candidate: {}",
                reasons.join("; ")
            )));
        }

        Ok(CertifiedVerification {
            base_root_hash: expected_root.clone(),
            derived_module,
            evidence,
        })
    }

    /// Atomically appends a new CommitRecord, advances head_commit and root_ref,
    /// cleans up associated draft, and persists to storage while holding lock.
    fn append_commit(
        &mut self,
        next_state: &mut DatabaseState,
        payload: CommitPayload,
        cleanup_hole_id: Option<&str>,
    ) -> Result<CommitRecord, CasError> {
        let new_root = payload.new_root.clone();
        let final_commit = CommitRecord::new(payload);
        next_state
            .commits
            .insert(final_commit.commit_hash.clone(), final_commit.clone());
        next_state.root_ref = Some(new_root);
        next_state.head_commit = Some(final_commit.commit_hash.clone());

        if let Some(hole_id) = cleanup_hole_id {
            next_state.drafts.remove(hole_id);
        }

        self.persist(next_state)?;
        self.state = next_state.clone();

        Ok(final_commit)
    }

    /// 统一私有重签认证内核：
    /// 1. 检验 tip.root 在当前 state 存在；
    /// 2. 检验主线落地见证，确保 candidate 是针对 target_hole_id 最新生效且修改了 root 的提交引入；
    ///    跳过后续 parent_root == new_root 的 receipt-only / no-op 提交，防止连续重签升级二次重放失败；
    /// 3. [Spec 1 核心] 确定性可重放验证：从 landing_commit 的 parent_root 重放 candidate 表达式，
    ///    核对推导模块哈希严格等于 landing_commit.new_root；
    /// 4. 确保当前 root 中目标函数无未填洞；
    /// 5. 严格 SMT 全域定理证明，通过 require_proved 密封铸造 CertifiedProofEvidence。
    fn certify_reattestation_kernel(
        &self,
        tip: &CommitTip,
        target_hole_id: &str,
        envelope_hash: Option<&NodeHash>,
        candidate_hash: Option<&NodeHash>,
    ) -> Result<CertifiedReattestation, CasError> {
        let root_hash = tip.root.as_ref().ok_or_else(|| {
            CasError::InvalidCandidate("cannot reattest candidate in an empty database".into())
        })?;
        let root_module = self.state.modules.get(root_hash).ok_or_else(|| {
            CasError::InvalidEnvelope(format!(
                "expected root module {} not found in database",
                root_hash
            ))
        })?;

        // 1. 获取当前主线历史
        let mainline = self.mainline_commits()?;

        // 2. 主线落地见证倒序查找：必须是针对该 target_hole_id 最新生效且改变了 root 的提交
        // （跳过 parent_root == new_root 的 receipt-only / no-op 提交，防止连续重签升级在已无洞的 parent 上二次重放报错）
        let mut landing_commit_opt: Option<&CommitRecord> = None;
        for commit in mainline.iter().rev() {
            if commit.parent_root() == Some(commit.new_root()) {
                continue;
            }
            if let Some(env) = self.state.envelopes.get(commit.envelope_hash()) {
                if env.target_hole_id == target_hole_id {
                    landing_commit_opt = Some(*commit);
                    break;
                }
            }
        }

        let landing_commit = landing_commit_opt.ok_or_else(|| {
            CasError::InvalidCandidate(format!(
                "candidate for hole `{target_hole_id}` has never landed in the active mainline history"
            ))
        })?;

        if let Some(exp_env) = envelope_hash {
            if landing_commit.envelope_hash() != exp_env {
                return Err(CasError::InvalidCandidate(format!(
                    "landing commit envelope {} does not match requested envelope {}",
                    landing_commit.envelope_hash(),
                    exp_env
                )));
            }
        }

        if let Some(exp_cand) = candidate_hash {
            if landing_commit.candidate_hash() != exp_cand {
                return Err(CasError::InvalidCandidate(format!(
                    "landing commit candidate {} does not match requested candidate {}",
                    landing_commit.candidate_hash(),
                    exp_cand
                )));
            }
        }

        let landing_env = self
            .state
            .envelopes
            .get(landing_commit.envelope_hash())
            .ok_or_else(|| {
                CasError::InvalidEnvelope(format!(
                    "task envelope {} not found in database",
                    landing_commit.envelope_hash()
                ))
            })?;

        if landing_env.target_hole_id != target_hole_id {
            return Err(CasError::InvalidEnvelope(format!(
                "task envelope hole `{}` does not match target hole `{}`",
                landing_env.target_hole_id, target_hole_id
            )));
        }

        let landing_cand = self
            .state
            .candidates
            .get(landing_commit.candidate_hash())
            .ok_or_else(|| {
                CasError::InvalidCandidate(format!(
                    "candidate {} not found in database",
                    landing_commit.candidate_hash()
                ))
            })?;

        if landing_cand.envelope_hash != *landing_commit.envelope_hash() {
            return Err(CasError::InvalidCandidate(format!(
                "candidate envelope hash {} does not match target envelope {}",
                landing_cand.envelope_hash,
                landing_commit.envelope_hash()
            )));
        }

        // 3. [Spec 1 核心] 确定性可重放落地验证 (Deterministic Replay Verification)
        let landing_base_module = match landing_commit.parent_root() {
            Some(pr) => self.state.modules.get(pr).ok_or_else(|| {
                CasError::InvalidEnvelope(format!(
                    "landing commit parent root module {} not found in database",
                    pr
                ))
            })?,
            None => &Module::default(),
        };

        let parsed_candidate =
            axiom_parser::parse_expr(&landing_cand.expression_source).map_err(|e| {
                CasError::InvalidCandidate(format!("candidate expression parse failure: {}", e))
            })?;

        let replayed_module =
            derive_candidate_module(landing_base_module, landing_env, &parsed_candidate).map_err(
                |e| CasError::InvalidCandidate(format!("replay derivation failure: {}", e)),
            )?;

        let replayed_hash = compute_module_hash(&replayed_module);
        if &replayed_hash != landing_commit.new_root() {
            return Err(CasError::ForgedModule(format!(
                "candidate replay derived module hash `{replayed_hash}` does not match landing commit new root `{}`",
                landing_commit.new_root()
            )));
        }

        // 4. 核验当前 root 目标函数无未填洞
        let func = root_module
            .functions
            .iter()
            .find(|f| f.name == landing_env.target_function)
            .ok_or_else(|| {
                CasError::InvalidEnvelope(format!(
                    "target function `{}` not found in root module",
                    landing_env.target_function
                ))
            })?;

        if axiom_ast::expr_contains_hole(&func.body) {
            return Err(CasError::UnverifiedCandidate(
                "target function still contains unresolved holes".into(),
            ));
        }

        // 5. 调用外部 SMT 验证并构造证明证据
        let smt_result = axiom_smt::verify_function_in_module(func, root_module);
        let evidence = require_proved(
            smt_result,
            landing_commit.envelope_hash().clone(),
            landing_commit.candidate_hash().clone(),
            func,
        )?;

        Ok(CertifiedReattestation {
            expected_tip: tip.clone(),
            target_hole_id: target_hole_id.to_string(),
            attested_commit_hash: landing_commit.commit_hash.clone(),
            evidence,
        })
    }

    /// 对指定 hole 的当前生效实现执行重新验证与证明签发，并原子提交重验收据。
    pub fn reverify_and_reattest_hole(
        &mut self,
        target_hole_id: &str,
    ) -> Result<CommitRecord, CasError> {
        let tip = self.commit_tip();

        let certified = self.certify_reattestation_kernel(&tip, target_hole_id, None, None)?;

        self.try_commit_reattestation(certified)
    }

    /// Performs offline theorem proving and verification for re-attesting an existing hole implementation
    /// without holding database locks. Returns a sealed CertifiedReattestation.
    #[allow(dead_code)]
    pub(crate) fn certify_reattestation_offline(
        &self,
        tip: &CommitTip,
        target_hole_id: &str,
        envelope_hash: &NodeHash,
        candidate_hash: &NodeHash,
    ) -> Result<CertifiedReattestation, CasError> {
        self.certify_reattestation_kernel(
            tip,
            target_hole_id,
            Some(envelope_hash),
            Some(candidate_hash),
        )
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

        if certified.evidence.envelope_hash != env_hash
            || certified.evidence.candidate_hash != cand_hash
        {
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
            self.persist(&next_state)?;
            self.state = next_state;
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

        // 3. 内核铸造防伪证明收据（直接继承原始证明求解器与版本信息）
        let receipt = VerificationReceipt {
            candidate_hash: cand_hash.clone(),
            proof_level: ProofLevel::Proved,
            solver: certified.evidence.solver,
            solver_version: certified.evidence.solver_version,
            solver_sha256: certified.evidence.solver_sha256,
            obligations_checked: certified.evidence.obligations_checked,
            proof_obligation_hash: certified.evidence.proof_obligation_hash,
            verification_profile_hash: current_verification_profile_hash(),
            attested_commit_hash: None,
            verified_at: candidate.timestamp,
        };
        let rcpt_hash = compute_receipt_hash(&receipt);
        next_state.receipts.insert(rcpt_hash.clone(), receipt);

        // 4. 录入新模块并通过通用原语写入提交记录
        let mod_record = Self::insert_module_to_state(&mut next_state, &certified.derived_module);
        let new_root = mod_record.module_hash;

        let payload = CommitPayload {
            parent_commit: self.state.head_commit.clone(),
            parent_root: expected_root.clone(),
            new_root,
            envelope_hash: env_hash,
            candidate_hash: cand_hash,
            receipt_hash: rcpt_hash,
            attested_commit_hash: None,
            timestamp: candidate.timestamp,
        };
        self.append_commit(&mut next_state, payload, Some(&envelope.target_hole_id))
    }

    /// 针对已存在于当前权威模块中的单洞实现，核验 CertifiedReattestation 并生成 Re-attestation CommitRecord。
    /// 锁内核验 (expected_root, expected_head) 双重 CAS、candidate 与 envelope 存在性，由内核权威铸造绑定当前 Profile 的收据。
    pub fn try_commit_reattestation(
        &mut self,
        certified: CertifiedReattestation,
    ) -> Result<CommitRecord, CasError> {
        let _guard = self.lock_and_sync()?;
        let mut next_state = self.state.clone();

        // 1. 双重 CAS 门禁：expected_tip 必须严格匹配
        if self.state.root_ref != certified.expected_tip.root {
            return Err(CasError::StaleRoot {
                current: self.state.root_ref.clone(),
                expected: certified.expected_tip.root,
            });
        }
        if self.state.head_commit != certified.expected_tip.head {
            return Err(CasError::StaleHead {
                current: self.state.head_commit.clone(),
                expected: certified.expected_tip.head,
            });
        }

        let current_root = self.state.root_ref.clone().ok_or_else(|| {
            CasError::InvalidCandidate("cannot reattest candidate in an empty database".into())
        })?;

        // 2. 锁内核验 candidate 与 envelope 依然存在于数据库中
        if !next_state
            .candidates
            .contains_key(&certified.evidence.candidate_hash)
        {
            return Err(CasError::InvalidCandidate(format!(
                "candidate {} not found in database",
                certified.evidence.candidate_hash
            )));
        }
        if !next_state
            .envelopes
            .contains_key(&certified.evidence.envelope_hash)
        {
            return Err(CasError::InvalidEnvelope(format!(
                "task envelope {} not found in database",
                certified.evidence.envelope_hash
            )));
        }

        // 3. 由 DB 内核权威铸造认证收据（绑定当前 Profile 并保真原始求解器元数据）
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let receipt = VerificationReceipt {
            candidate_hash: certified.evidence.candidate_hash.clone(),
            proof_level: ProofLevel::Proved,
            solver: certified.evidence.solver,
            solver_version: certified.evidence.solver_version,
            solver_sha256: certified.evidence.solver_sha256,
            obligations_checked: certified.evidence.obligations_checked,
            proof_obligation_hash: certified.evidence.proof_obligation_hash,
            verification_profile_hash: current_verification_profile_hash(),
            attested_commit_hash: Some(certified.attested_commit_hash.clone()),
            verified_at: timestamp,
        };
        let rcpt_hash = compute_receipt_hash(&receipt);
        next_state.receipts.insert(rcpt_hash.clone(), receipt);

        // 4. 调用 append_commit 原语完成原子写入与草稿清理
        let payload = CommitPayload {
            parent_commit: self.state.head_commit.clone(),
            parent_root: Some(current_root.clone()),
            new_root: current_root,
            envelope_hash: certified.evidence.envelope_hash,
            candidate_hash: certified.evidence.candidate_hash,
            receipt_hash: rcpt_hash,
            attested_commit_hash: Some(certified.attested_commit_hash),
            timestamp,
        };
        self.append_commit(&mut next_state, payload, Some(&certified.target_hole_id))
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

    /// Inserts or updates a provisional draft with typed CAS validation on expected revision and base root.
    pub fn insert_draft_cas(
        &mut self,
        mut draft: DraftRecord,
        cas: &DraftCasExpectation,
    ) -> Result<DraftRecord, CasError> {
        let _guard = self.lock_and_sync()?;
        let mut next_state = self.state.clone();

        // 1. 严格检查 base_root 匹配，防止并发基于陈旧 baseline 覆写
        if let Some(ref exp_root) = cas.expected_base_root {
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
            if let Some(exp_rev) = cas.expected_revision {
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
            if let Some(exp_rev) = cas.expected_revision {
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

        // 由 DB 锁内权威设置实际 base_root
        draft.base_root = next_state.root_ref.clone();

        next_state
            .drafts
            .insert(draft.hole_id.clone(), draft.clone());
        self.persist(&next_state)?;
        self.state = next_state;
        Ok(draft)
    }

    /// Convenience wrapper around insert_draft_cas without revision constraints.
    pub fn insert_draft(&mut self, draft: DraftRecord) -> Result<DraftRecord, CasError> {
        self.insert_draft_cas(draft, &DraftCasExpectation::unconstrained())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_genesis_detection_and_ancestry_repair() {
        let mut state = DatabaseState::default();

        let mut mod0 = Module::default();
        mod0.functions.push(FunctionDecl {
            name: "f1".into(),
            params: vec![],
            return_type: Type::Int,
            effects: vec![],
            requires: vec![],
            ensures: vec![],
            body: Expr::Hole("h1".into()),
        });
        mod0.functions.push(FunctionDecl {
            name: "f2".into(),
            params: vec![],
            return_type: Type::Int,
            effects: vec![],
            requires: vec![],
            ensures: vec![],
            body: Expr::Hole("h2".into()),
        });
        mod0.functions.push(FunctionDecl {
            name: "f3".into(),
            params: vec![],
            return_type: Type::Int,
            effects: vec![],
            requires: vec![],
            ensures: vec![],
            body: Expr::Hole("h3".into()),
        });
        let root0 = compute_module_hash(&mod0);

        let mut mod1 = mod0.clone();
        mod1.functions[0].body = axiom_parser::parse_expr("1").unwrap();
        let root1 = compute_module_hash(&mod1);

        let mut mod2 = mod1.clone();
        mod2.functions[1].body = axiom_parser::parse_expr("2").unwrap();
        let root2 = compute_module_hash(&mod2);

        let mut mod3 = mod2.clone();
        mod3.functions[2].body = axiom_parser::parse_expr("3").unwrap();
        let root3 = compute_module_hash(&mod3);

        state.modules.insert(root0.clone(), mod0.clone());
        state.modules.insert(root1.clone(), mod1.clone());
        state.modules.insert(root2.clone(), mod2.clone());
        state.modules.insert(root3.clone(), mod3.clone());

        // 构造三步变更的合法信封、候选与收据工件
        let mk_artifacts =
            |base_root: Option<NodeHash>, fn_name: &str, hole_id: &str, expr_src: &str, ts: u64| {
                let env = TaskEnvelope {
                    base_root_hash: base_root,
                    target_hole_id: hole_id.into(),
                    target_function: fn_name.into(),
                    contract_hash: compute_contract_hash(&mod0.functions[0]),
                    dependency_hashes: vec![],
                };
                let env_hash = compute_envelope_hash(&env);

                let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();
                let cand = Candidate {
                    envelope_hash: env_hash.clone(),
                    expression_source: expr_src.into(),
                    expression_hash: compute_expr_hash(&parsed_expr),
                    author_agent_id: "test_agent".into(),
                    timestamp: ts,
                };
                let cand_hash = compute_candidate_hash(&cand);

                let rcpt = VerificationReceipt {
                    candidate_hash: cand_hash.clone(),
                    proof_level: ProofLevel::Proved,
                    solver: "z3".into(),
                    solver_version: "4.12.0".into(),
                    solver_sha256: None,
                    obligations_checked: 1,
                    proof_obligation_hash: compute_proof_obligation_hash(&mod0.functions[0]),
                    verification_profile_hash: current_verification_profile_hash(),
                    attested_commit_hash: None,
                    verified_at: ts,
                };
                let rcpt_hash = compute_receipt_hash(&rcpt);

                (env_hash, env, cand_hash, cand, rcpt_hash, rcpt)
            };

        let (e1_h, e1, c1_h, c1, r1_h, r1) =
            mk_artifacts(Some(root0.clone()), "f1", "h1", "1", 100);
        let (e2_h, e2, c2_h, c2, r2_h, r2) =
            mk_artifacts(Some(root1.clone()), "f2", "h2", "2", 200);
        let (e3_h, e3, c3_h, c3, r3_h, r3) =
            mk_artifacts(Some(root2.clone()), "f3", "h3", "3", 300);

        state.envelopes.insert(e1_h.clone(), e1);
        state.candidates.insert(c1_h.clone(), c1);
        state.receipts.insert(r1_h.clone(), r1);

        state.envelopes.insert(e2_h.clone(), e2);
        state.candidates.insert(c2_h.clone(), c2);
        state.receipts.insert(r2_h.clone(), r2);

        state.envelopes.insert(e3_h.clone(), e3);
        state.candidates.insert(c3_h.clone(), c3);
        state.receipts.insert(r3_h.clone(), r3);

        // 构造未版本化的 LegacyCommitPayload（6 字段）并计算历史旧哈希
        let mk_legacy_commit = |parent_root: Option<NodeHash>,
                                new_root: NodeHash,
                                env_hash: NodeHash,
                                cand_hash: NodeHash,
                                rcpt_hash: NodeHash,
                                ts: u64| {
            let leg_payload = LegacyCommitPayload {
                parent_root: parent_root.clone(),
                new_root: new_root.clone(),
                envelope_hash: env_hash.clone(),
                candidate_hash: cand_hash.clone(),
                receipt_hash: rcpt_hash.clone(),
                timestamp: ts,
            };
            let commit_hash = compute_legacy_commit_payload_hash(&leg_payload);
            CommitRecord {
                commit_hash,
                payload: CommitPayload {
                    parent_commit: None,
                    parent_root,
                    new_root,
                    envelope_hash: env_hash,
                    candidate_hash: cand_hash,
                    receipt_hash: rcpt_hash,
                    attested_commit_hash: None,
                    timestamp: ts,
                },
            }
        };

        let com1 = mk_legacy_commit(Some(root0), root1.clone(), e1_h, c1_h, r1_h, 100);
        let com2 = mk_legacy_commit(Some(root1), root2.clone(), e2_h, c2_h, r2_h, 200);
        let com3 = mk_legacy_commit(Some(root2), root3.clone(), e3_h, c3_h, r3_h, 300);

        // 乱序存入 commits map，模拟真实旧版本数据库且 head_commit 未初始化
        state.commits.insert(com2.commit_hash.clone(), com2);
        state.commits.insert(com1.commit_hash.clone(), com1);
        state.commits.insert(com3.commit_hash.clone(), com3);
        state.root_ref = Some(root3);
        state.head_commit = None;

        let migrated = AstDatabase::migrate_legacy_commits_if_needed(&mut state);
        assert!(migrated.unwrap());
        assert!(state.head_commit.is_some());

        let db = AstDatabase { path: None, state };
        let mainline = db
            .mainline_commits()
            .expect("mainline validation must succeed");
        assert_eq!(mainline.len(), 3);
        assert_eq!(mainline[1].parent_commit(), Some(&mainline[0].commit_hash));
        assert_eq!(mainline[2].parent_commit(), Some(&mainline[1].commit_hash));
        for c in &mainline {
            assert!(c.verify_self_hash());
        }
    }

    #[test]
    fn test_internal_tampered_legacy_payload_fails_closed_without_whitewashing() {
        let mut state = DatabaseState::default();

        let mut mod0 = Module::default();
        mod0.functions.push(FunctionDecl {
            name: "f1".into(),
            params: vec![],
            return_type: Type::Int,
            effects: vec![],
            requires: vec![],
            ensures: vec![],
            body: Expr::Hole("h1".into()),
        });
        let root0 = compute_module_hash(&mod0);

        let mut mod1 = mod0.clone();
        mod1.functions[0].body = axiom_parser::parse_expr("1").unwrap();
        let root1 = compute_module_hash(&mod1);

        state.modules.insert(root0.clone(), mod0.clone());
        state.modules.insert(root1.clone(), mod1);

        let env = TaskEnvelope {
            base_root_hash: Some(root0.clone()),
            target_hole_id: "h1".into(),
            target_function: "f1".into(),
            contract_hash: compute_contract_hash(&mod0.functions[0]),
            dependency_hashes: vec![],
        };
        let env_hash = compute_envelope_hash(&env);

        let parsed_expr = axiom_parser::parse_expr("1").unwrap();
        let cand = Candidate {
            envelope_hash: env_hash.clone(),
            expression_source: "1".into(),
            expression_hash: compute_expr_hash(&parsed_expr),
            author_agent_id: "test_agent".into(),
            timestamp: 100,
        };
        let cand_hash = compute_candidate_hash(&cand);

        let rcpt = VerificationReceipt {
            candidate_hash: cand_hash.clone(),
            proof_level: ProofLevel::Proved,
            solver: "z3".into(),
            solver_version: "4.12.0".into(),
            solver_sha256: None,
            obligations_checked: 1,
            proof_obligation_hash: compute_proof_obligation_hash(&mod0.functions[0]),
            verification_profile_hash: current_verification_profile_hash(),
            attested_commit_hash: None,
            verified_at: 100,
        };
        let rcpt_hash = compute_receipt_hash(&rcpt);

        state.envelopes.insert(env_hash.clone(), env);
        state.candidates.insert(cand_hash.clone(), cand);
        state.receipts.insert(rcpt_hash.clone(), rcpt);

        let leg_payload = LegacyCommitPayload {
            parent_root: Some(root0),
            new_root: root1.clone(),
            envelope_hash: env_hash.clone(),
            candidate_hash: cand_hash.clone(),
            receipt_hash: rcpt_hash.clone(),
            timestamp: 100,
        };
        let honest_hash = compute_legacy_commit_payload_hash(&leg_payload);

        let tampered_record = CommitRecord {
            commit_hash: honest_hash.clone(),
            payload: CommitPayload {
                parent_commit: None,
                parent_root: None,
                new_root: root1.clone(),
                envelope_hash: env_hash,
                candidate_hash: cand_hash,
                receipt_hash: rcpt_hash,
                attested_commit_hash: None,
                timestamp: 999,
            },
        };

        state.commits.insert(honest_hash, tampered_record);
        state.root_ref = Some(root1);
        state.head_commit = None;

        let res = AstDatabase::migrate_legacy_commits_if_needed(&mut state);
        match res {
            Err(DbError::CorruptedCommitChain(msg)) => {
                assert!(
                    msg.contains("legacy commit hash mismatch"),
                    "expected legacy commit hash mismatch, got: {msg}"
                );
            }
            other => panic!(
                "expected migration to fail-closed on tampered legacy payload, got: {other:?}"
            ),
        }
    }

    #[test]
    fn test_internal_reattestation_offline_binding_and_strict_cas() {
        let mut db = AstDatabase::in_memory();
        let mod0 = axiom_parser::parse_module(
            r#"
            fn increment(x: Int) -> Int
                requires { x >= 0, x < 1000000 }
                ensures { result > x }
            {
                ?inc_impl
            }
            "#,
        )
        .expect("skeleton must parse");

        let mod_rec = db.insert_module(&mod0).unwrap();
        let root0 = mod_rec.module_hash;
        db.initialize_root(root0.clone()).unwrap();

        let env = TaskEnvelope {
            base_root_hash: Some(root0.clone()),
            target_hole_id: "inc_impl".into(),
            target_function: "increment".into(),
            contract_hash: compute_contract_hash(&mod0.functions[0]),
            dependency_hashes: vec![],
        };
        let env_hash = compute_envelope_hash(&env);

        let parsed_cand = axiom_parser::parse_expr("x + 1").unwrap();
        let cand = Candidate {
            envelope_hash: env_hash.clone(),
            expression_source: "x + 1".into(),
            expression_hash: compute_expr_hash(&parsed_cand),
            author_agent_id: "agent_1".into(),
            timestamp: 1000,
        };
        let cand_hash = compute_candidate_hash(&cand);

        let commit1 = db
            .try_commit_candidate(&Some(root0), env, cand, None)
            .expect("first candidate commit should succeed");

        let tip = db.commit_tip();
        let certified = db
            .certify_reattestation_offline(&tip, "inc_impl", &env_hash, &cand_hash)
            .expect("certify_reattestation_offline should succeed");

        assert_eq!(certified.attested_commit_hash(), &commit1.commit_hash);

        let commit2 = db
            .try_commit_reattestation(certified.clone())
            .expect("try_commit_reattestation should succeed");
        assert_eq!(commit2.attested_commit_hash(), Some(&commit1.commit_hash));

        // 双重 CAS 校验：再次使用持有旧 head (commit1) 的 certified 提交，必须被 StaleHead 拦截！
        let stale_res = db.try_commit_reattestation(certified);
        assert!(
            matches!(stale_res, Err(CasError::StaleHead { .. })),
            "stale certified reattestation must be rejected by double CAS"
        );
    }

    #[test]
    fn test_internal_v2_4492184_database_migration_success() {
        let temp_dir = std::env::temp_dir().join(format!("v2_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("v2_db.json");

        // 构造真正的 Commit 4492184 (Schema V2) 数据库 JSON
        let mod0 = axiom_parser::parse_module(
            r#"
            fn calculate(x: Int) -> Int
                requires { x > 0 }
                ensures { result > x }
            {
                ?h_calc
            }
            "#,
        )
        .unwrap();
        let root0 = compute_module_hash(&mod0);

        let parsed_cand = axiom_parser::parse_expr("x + 10").unwrap();
        let mut mod1 = mod0.clone();
        mod1.functions[0].body = parsed_cand.clone();
        let root1 = compute_module_hash(&mod1);

        let env = TaskEnvelope {
            base_root_hash: Some(root0.clone()),
            target_hole_id: "h_calc".into(),
            target_function: "calculate".into(),
            contract_hash: compute_contract_hash(&mod0.functions[0]),
            dependency_hashes: vec![],
        };
        let env_hash = compute_envelope_hash(&env);

        let cand = Candidate {
            envelope_hash: env_hash.clone(),
            expression_source: "x + 10".into(),
            expression_hash: compute_expr_hash(&parsed_cand),
            author_agent_id: "agent_4492184".into(),
            timestamp: 1000,
        };
        let cand_hash = compute_candidate_hash(&cand);

        let v2_rcpt1 = LegacyV2VerificationReceipt {
            candidate_hash: cand_hash.clone(),
            proof_level: ProofLevel::Proved,
            solver: "z3".into(),
            solver_version: "4.12.2".into(),
            obligations_checked: 1,
            proof_obligation_hash: compute_proof_obligation_hash(&mod1.functions[0]),
            verification_profile_hash: current_verification_profile_hash(),
            verified_at: 1000,
        };
        let rcpt1_hash = compute_legacy_v2_receipt_hash(&v2_rcpt1);

        let v2_commit1 = LegacyV2CommitPayload {
            parent_commit: None,
            parent_root: Some(root0.clone()),
            new_root: root1.clone(),
            envelope_hash: env_hash.clone(),
            candidate_hash: cand_hash.clone(),
            receipt_hash: rcpt1_hash.clone(),
            timestamp: 1000,
        };
        let commit1_hash = compute_legacy_v2_commit_payload_hash(&v2_commit1);

        // 重签产生的 no-op 提交 (parent_root == new_root == root1)
        let v2_rcpt2 = LegacyV2VerificationReceipt {
            candidate_hash: cand_hash.clone(),
            proof_level: ProofLevel::Proved,
            solver: "z3".into(),
            solver_version: "4.12.2".into(),
            obligations_checked: 1,
            proof_obligation_hash: compute_proof_obligation_hash(&mod1.functions[0]),
            verification_profile_hash: current_verification_profile_hash(),
            verified_at: 2000,
        };
        let rcpt2_hash = compute_legacy_v2_receipt_hash(&v2_rcpt2);

        let v2_commit2 = LegacyV2CommitPayload {
            parent_commit: Some(commit1_hash.clone()),
            parent_root: Some(root1.clone()),
            new_root: root1.clone(),
            envelope_hash: env_hash.clone(),
            candidate_hash: cand_hash.clone(),
            receipt_hash: rcpt2_hash.clone(),
            timestamp: 2000,
        };
        let commit2_hash = compute_legacy_v2_commit_payload_hash(&v2_commit2);

        let raw_json = serde_json::json!({
            "root_ref": root1,
            "head_commit": commit2_hash,
            "modules": {
                root0.as_str(): mod0,
                root1.as_str(): mod1
            },
            "envelopes": {
                env_hash.as_str(): env
            },
            "candidates": {
                cand_hash.as_str(): cand
            },
            "receipts": {
                rcpt1_hash.as_str(): v2_rcpt1,
                rcpt2_hash.as_str(): v2_rcpt2
            },
            "commits": {
                commit1_hash.as_str(): serde_json::json!({
                    "commit_hash": commit1_hash,
                    "parent_commit": v2_commit1.parent_commit,
                    "parent_root": v2_commit1.parent_root,
                    "new_root": v2_commit1.new_root,
                    "envelope_hash": v2_commit1.envelope_hash,
                    "candidate_hash": v2_commit1.candidate_hash,
                    "receipt_hash": v2_commit1.receipt_hash,
                    "timestamp": v2_commit1.timestamp
                }),
                commit2_hash.as_str(): serde_json::json!({
                    "commit_hash": commit2_hash,
                    "parent_commit": v2_commit2.parent_commit,
                    "parent_root": v2_commit2.parent_root,
                    "new_root": v2_commit2.new_root,
                    "envelope_hash": v2_commit2.envelope_hash,
                    "candidate_hash": v2_commit2.candidate_hash,
                    "receipt_hash": v2_commit2.receipt_hash,
                    "timestamp": v2_commit2.timestamp
                })
            }
        });

        std::fs::write(&db_path, serde_json::to_string_pretty(&raw_json).unwrap()).unwrap();

        // 执行打开，触发 V2 自愈迁移
        let db = AstDatabase::open(&db_path).expect("V2 database migration must succeed");
        assert_eq!(db.state.schema_version, Some(CURRENT_SCHEMA_VERSION));

        let mainline = db.mainline_commits().expect("mainline should be valid");
        assert_eq!(mainline.len(), 2);

        let c1 = &mainline[0];
        assert_eq!(c1.parent_commit(), None);
        assert_eq!(c1.attested_commit_hash(), None);
        assert!(c1.verify_self_hash());

        let c2 = &mainline[1];
        assert_eq!(c2.parent_commit(), Some(&c1.commit_hash));
        assert_eq!(c2.attested_commit_hash(), Some(&c1.commit_hash));
        assert!(c2.verify_self_hash());

        // 再次打开验证自洽性，不再触发重算
        let db_reopen = AstDatabase::open(&db_path).expect("reopen must succeed");
        assert_eq!(db_reopen.head_commit(), db.head_commit());

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_internal_v2_4492184_database_tampered_fails_closed() {
        let temp_dir = std::env::temp_dir().join(format!("v2_tamper_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("v2_tampered.json");

        let mod0 = axiom_parser::parse_module("fn f() -> Int { 42 }").unwrap();
        let root0 = compute_module_hash(&mod0);

        let v2_rcpt = LegacyV2VerificationReceipt {
            candidate_hash: compute_hash("cand", &"test_candidate"),
            proof_level: ProofLevel::Proved,
            solver: "z3".into(),
            solver_version: "4.12.2".into(),
            obligations_checked: 0,
            proof_obligation_hash: compute_hash("obl", &"test_obligation"),
            verification_profile_hash: current_verification_profile_hash(),
            verified_at: 1000,
        };
        let legit_rcpt_hash = compute_legacy_v2_receipt_hash(&v2_rcpt);

        // 篡改收据内部字段，但保持 map key 为原 legit_rcpt_hash
        let mut tampered_rcpt = v2_rcpt;
        tampered_rcpt.solver_version = "hacked-solver".into();

        let raw_json = serde_json::json!({
            "root_ref": root0,
            "head_commit": "commit_dummy",
            "modules": { root0.as_str(): mod0 },
            "receipts": {
                legit_rcpt_hash.as_str(): tampered_rcpt
            },
            "commits": {
                "commit_dummy": {
                    "commit_hash": "commit_dummy",
                    "parent_commit": null,
                    "parent_root": null,
                    "new_root": root0,
                    "envelope_hash": "env_test",
                    "candidate_hash": "cand_test",
                    "receipt_hash": legit_rcpt_hash,
                    "timestamp": 1000
                }
            }
        });

        std::fs::write(&db_path, serde_json::to_string_pretty(&raw_json).unwrap()).unwrap();

        let open_res = AstDatabase::open(&db_path);
        match open_res {
            Err(DbError::CorruptedCommitChain(msg)) => {
                assert!(
                    msg.contains("legacy receipt hash mismatch"),
                    "expected tamper detection rejection, got: {msg}"
                );
            }
            other => panic!("expected CorruptedCommitChain, got: {other:?}"),
        }

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_internal_historical_profile_integrity_decoupled_from_current() {
        // 验证旧 Profile 的合法历史收据在因果图校验中顺利通过，不报损坏
        let mut db = AstDatabase::in_memory();
        let mod0 = axiom_parser::parse_module(
            r#"
            fn inc(x: Int) -> Int
                requires { x >= 0, x < 1000000 }
                ensures { result > x }
            {
                ?h_inc
            }
            "#,
        )
        .unwrap();

        let mod_rec = db.insert_module(&mod0).unwrap();
        let root0 = mod_rec.module_hash;
        db.initialize_root(root0.clone()).unwrap();

        let env = TaskEnvelope {
            base_root_hash: Some(root0.clone()),
            target_hole_id: "h_inc".into(),
            target_function: "inc".into(),
            contract_hash: compute_contract_hash(&mod0.functions[0]),
            dependency_hashes: vec![],
        };
        let env_hash = compute_envelope_hash(&env);

        let parsed_cand = axiom_parser::parse_expr("x + 1").unwrap();
        let cand = Candidate {
            envelope_hash: env_hash.clone(),
            expression_source: "x + 1".into(),
            expression_hash: compute_expr_hash(&parsed_cand),
            author_agent_id: "agent_old".into(),
            timestamp: 1000,
        };
        let cand_hash = compute_candidate_hash(&cand);

        let _landing_commit = db
            .try_commit_candidate(&Some(root0), env, cand, None)
            .expect("candidate commit should succeed");

        let tip = db.commit_tip();
        let certified = db
            .certify_reattestation_offline(&tip, "h_inc", &env_hash, &cand_hash)
            .expect("certify_reattestation_offline should succeed");

        let noop_commit = db
            .try_commit_reattestation(certified)
            .expect("try_commit_reattestation should succeed");

        // 将生成的 no-op receipt 的 profile 篡改为历史旧 profile，重新计算 receipt_hash 并更新提交
        let mut state = db.state.clone();
        let old_rcpt = state
            .receipts
            .get(noop_commit.receipt_hash())
            .unwrap()
            .clone();
        let mut historical_rcpt = old_rcpt;
        historical_rcpt.verification_profile_hash = legacy_verification_profile_hash();
        let new_rcpt_hash = compute_receipt_hash(&historical_rcpt);

        state.receipts.remove(noop_commit.receipt_hash());
        state
            .receipts
            .insert(new_rcpt_hash.clone(), historical_rcpt);

        let mut updated_noop = noop_commit.clone();
        updated_noop.payload.receipt_hash = new_rcpt_hash;
        updated_noop.commit_hash = compute_commit_payload_hash(&updated_noop.payload);

        state.commits.remove(&noop_commit.commit_hash);
        state
            .commits
            .insert(updated_noop.commit_hash.clone(), updated_noop.clone());
        state.head_commit = Some(updated_noop.commit_hash.clone());

        // 主链因果校验必须顺利通过，不再因为 profile != current 而判损坏！
        let res = CommitGraph::validate_mainline(&state);
        assert!(
            res.is_ok(),
            "historical profile receipts must be validated without corrupting the commit chain: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_future_schema_version_is_rejected_fail_closed() {
        let temp_dir = std::env::temp_dir().join(format!("future_schema_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("future_schema.json");

        // 构造未来版本 (schema_version = 999) 数据库
        let future_state = DatabaseState {
            schema_version: Some(999),
            ..Default::default()
        };
        fs::write(&db_path, serde_json::to_vec_pretty(&future_state).unwrap()).unwrap();

        // 尝试打开未来版本的数据库，必须 Fail-Closed 拒绝
        let res = AstDatabase::open(&db_path);
        match res {
            Err(DbError::IncompatibleSchemaVersion { found, current }) => {
                assert_eq!(found, 999);
                assert_eq!(current, CURRENT_SCHEMA_VERSION);
            }
            other => panic!("expected IncompatibleSchemaVersion error, got {other:?}"),
        }

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_empty_v2_database_is_upgraded_to_current_schema() {
        let temp_dir = std::env::temp_dir().join(format!("empty_v2_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("empty_v2.json");

        // 构造空的 V2 版本数据库 (schema_version = 2)
        let empty_v2 = DatabaseState {
            schema_version: Some(2),
            ..Default::default()
        };
        fs::write(&db_path, serde_json::to_vec_pretty(&empty_v2).unwrap()).unwrap();

        // 打开空旧库，必须无条件升级为 Schema 3 并落盘
        let db = AstDatabase::open(&db_path).expect("opening empty v2 database should succeed");
        assert_eq!(db.state.schema_version, Some(CURRENT_SCHEMA_VERSION));

        // 重新从磁盘读取，验证落盘数据也已升级
        let disk_bytes = fs::read(&db_path).unwrap();
        let disk_state: DatabaseState = serde_json::from_slice(&disk_bytes).unwrap();
        assert_eq!(disk_state.schema_version, Some(CURRENT_SCHEMA_VERSION));

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_migrated_v2_database_has_no_orphan_receipts() {
        let temp_dir = std::env::temp_dir().join(format!("migration_clean_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("clean_receipts.json");

        let mut db = AstDatabase::open(&db_path).unwrap();
        let mod0 = axiom_parser::parse_module(
            r#"
            fn inc(x: Int) -> Int
                requires { x > 0 && x < 1000 }
                ensures { result > x }
            {
                ?h_inc
            }
            "#,
        )
        .unwrap();
        let mod_rec = db.insert_module(&mod0).unwrap();
        let root0 = mod_rec.module_hash;
        db.initialize_root(root0.clone()).unwrap();

        let env = TaskEnvelope {
            base_root_hash: Some(root0.clone()),
            target_hole_id: "h_inc".into(),
            target_function: "inc".into(),
            contract_hash: compute_contract_hash(&mod0.functions[0]),
            dependency_hashes: vec![],
        };
        let env_hash = compute_envelope_hash(&env);

        let parsed_cand = axiom_parser::parse_expr("x + 1").unwrap();
        let cand = Candidate {
            envelope_hash: env_hash.clone(),
            expression_source: "x + 1".into(),
            expression_hash: compute_expr_hash(&parsed_cand),
            author_agent_id: "agent_a".into(),
            timestamp: 1000,
        };
        let cand_hash = compute_candidate_hash(&cand);

        let _landing = db
            .try_commit_candidate(&Some(root0), env, cand, None)
            .unwrap();
        let tip = db.commit_tip();
        let cert = db
            .certify_reattestation_offline(&tip, "h_inc", &env_hash, &cand_hash)
            .unwrap();
        let _noop = db.try_commit_reattestation(cert).unwrap();

        // 降级构造 V2 格式数据
        let mut v2_state = db.state.clone();
        v2_state.schema_version = None;

        // 迁移执行
        let mut migrated_state = v2_state;
        CommitGraph::repair_v2_migration(&mut migrated_state).unwrap();

        // 强断言：所有收据严格满足 compute_receipt_hash(rcpt) == key，没有任何旧哈希孤儿收据
        assert_eq!(
            migrated_state.receipts.len(),
            2,
            "migrated state should only contain the 2 valid active receipts"
        );
        for (k, v) in &migrated_state.receipts {
            let computed = compute_receipt_hash(v);
            assert_eq!(
                &computed, k,
                "receipt map key must strictly equal its content hash"
            );
        }

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_require_proved_fail_closed_validation() {
        let target_func = FunctionDecl {
            name: "test_func".into(),
            params: vec![],
            return_type: Type::Int,
            effects: vec![],
            requires: vec![],
            ensures: vec![],
            body: Expr::Literal(axiom_ast::Literal::Int(0)),
        };
        let env_hash = NodeHash("#env_dummy".into());
        let cand_hash = NodeHash("#cand_dummy".into());

        // 1. None solver_sha256 必须被拦截
        let smt_none = axiom_smt::SmtResult::Proved {
            solver: "z3".into(),
            solver_path: None,
            solver_sha256: None,
            version: "4.12.2".into(),
            obligations_checked: 1,
        };
        let res_none = require_proved(smt_none, env_hash.clone(), cand_hash.clone(), &target_func);
        assert!(
            matches!(res_none, Err(CasError::UnverifiedCandidate(ref msg)) if msg.contains("missing")),
            "缺少 solver_sha256 必须被 require_proved 拒绝，收到: {res_none:?}"
        );

        // 2. 非法格式 solver_sha256 必须被拦截
        let smt_bad_sha = axiom_smt::SmtResult::Proved {
            solver: "z3".into(),
            solver_path: None,
            solver_sha256: Some("invalid_short_hash".into()),
            version: "4.12.2".into(),
            obligations_checked: 1,
        };
        let res_bad = require_proved(
            smt_bad_sha,
            env_hash.clone(),
            cand_hash.clone(),
            &target_func,
        );
        assert!(
            matches!(res_bad, Err(CasError::UnverifiedCandidate(ref msg)) if msg.contains("invalid format")),
            "非法格式 solver_sha256 必须被 require_proved 拒绝，收到: {res_bad:?}"
        );

        // 3. unknown 或空版本必须被拦截
        let smt_unknown = axiom_smt::SmtResult::Proved {
            solver: "z3".into(),
            solver_path: None,
            solver_sha256: Some(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            ),
            version: "unknown".into(),
            obligations_checked: 1,
        };
        let res_unknown = require_proved(
            smt_unknown,
            env_hash.clone(),
            cand_hash.clone(),
            &target_func,
        );
        assert!(
            matches!(res_unknown, Err(CasError::UnverifiedCandidate(ref msg)) if msg.contains("must be known")),
            "unknown 求解器版本必须被 require_proved 拒绝，收到: {res_unknown:?}"
        );

        // 4. 合法 64 位 SHA-256 和明确版本必须通过
        let smt_valid = axiom_smt::SmtResult::Proved {
            solver: "z3".into(),
            solver_path: None,
            solver_sha256: Some(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            ),
            version: "4.13.0".into(),
            obligations_checked: 1,
        };
        let res_valid = require_proved(smt_valid, env_hash, cand_hash, &target_func);
        assert!(res_valid.is_ok(), "合法的证明结果与求解器哈希必须成功通过");
        let ev = res_valid.unwrap();
        assert_eq!(
            ev.solver_sha256(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
    }
}
