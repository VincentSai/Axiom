//! Commit graph topology traversal, integrity validation, and legacy migration.

use std::collections::{BTreeMap, HashSet};

use axiom_ast::Module;

use crate::{
    compute_candidate_hash, compute_commit_payload_hash, compute_envelope_hash, compute_expr_hash,
    compute_legacy_commit_payload_hash, compute_module_hash, compute_receipt_hash,
    derive_candidate_module, CommitRecord, DatabaseState, DbError, LegacyCommitPayload, NodeHash,
};

/// Topology and integrity auditor for AST database commits.
pub(crate) struct CommitGraph;

impl CommitGraph {
    /// Validates the mainline commit chain starting from `state.head_commit` back to the genesis commit.
    ///
    /// Asserts:
    /// 1. `head_commit` corresponds to `state.root_ref` when present.
    /// 2. Commits map key strictly equals `commit.commit_hash`.
    /// 3. Self-hash cryptographic integrity holds for each commit (`verify_self_hash`).
    /// 4. TaskEnvelope, Candidate, and VerificationReceipt artifacts exist and match content hashes.
    /// 5. Cross-binding consistency between candidate, envelope, and receipt.
    /// 6. Strict genealogical continuity (`child.parent_commit == parent.commit_hash` and `child.parent_root == parent.new_root`).
    /// 7. Envelope base_root_hash strictly matches commit.parent_root.
    /// 8. Candidate expression_source re-parses and matches expression_hash.
    /// 9. Deterministic replay: candidate expression derived against parent module matches commit.new_root.
    pub(crate) fn validate_mainline(state: &DatabaseState) -> Result<Vec<&CommitRecord>, DbError> {
        let head_hash = match &state.head_commit {
            Some(h) => h,
            None => {
                if state.commits.is_empty() {
                    if let Some(ref root_ref) = state.root_ref {
                        if !state.modules.contains_key(root_ref) {
                            return Err(DbError::CorruptedCommitChain(format!(
                                "root_ref `{root_ref}` not found in database modules"
                            )));
                        }
                    }
                    return Ok(Vec::new());
                } else {
                    return Err(DbError::CorruptedCommitChain(
                        "head_commit is None but database contains commits".into(),
                    ));
                }
            }
        };

        let head_commit = state.commits.get(head_hash).ok_or_else(|| {
            DbError::CorruptedCommitChain(format!(
                "head commit `{head_hash}` not found in database commits"
            ))
        })?;

        let root_ref = state.root_ref.as_ref().ok_or_else(|| {
            DbError::CorruptedCommitChain(format!(
                "head commit `{head_hash}` exists but root_ref is None"
            ))
        })?;

        if head_commit.new_root() != root_ref {
            return Err(DbError::CorruptedCommitChain(format!(
                "head commit new root `{}` does not match database root_ref `{}`",
                head_commit.new_root(),
                root_ref
            )));
        }

        let mut ordered_commits = Vec::new();
        let mut curr_commit_hash = Some(head_hash);
        let mut visited: HashSet<&NodeHash> = HashSet::new();

        while let Some(hash) = curr_commit_hash {
            if !visited.insert(hash) {
                return Err(DbError::CorruptedCommitChain(format!(
                    "cycle detected in commit chain at commit hash `{hash}`"
                )));
            }

            let (map_key, commit) = state.commits.get_key_value(hash).ok_or_else(|| {
                DbError::CorruptedCommitChain(format!(
                    "broken commit chain: commit `{hash}` referenced but not found in commits map"
                ))
            })?;

            if map_key != &commit.commit_hash {
                return Err(DbError::CorruptedCommitChain(format!(
                    "commits map key `{map_key}` does not match commit_hash `{}`",
                    commit.commit_hash
                )));
            }

            if !commit.verify_self_hash() {
                return Err(DbError::CorruptedCommitChain(format!(
                    "commit hash mismatch for commit `{hash}`: payload corrupted or tampered"
                )));
            }

            // Artifact existence and hash integrity checks
            let env = state.envelopes.get(commit.envelope_hash()).ok_or_else(|| {
                DbError::CorruptedCommitChain(format!(
                    "commit `{hash}` references missing task envelope `{}`",
                    commit.envelope_hash()
                ))
            })?;
            let actual_env_hash = compute_envelope_hash(env);
            if &actual_env_hash != commit.envelope_hash() {
                return Err(DbError::CorruptedCommitChain(format!(
                    "envelope hash mismatch for `{}`: computed `{actual_env_hash}`",
                    commit.envelope_hash()
                )));
            }

            let cand = state
                .candidates
                .get(commit.candidate_hash())
                .ok_or_else(|| {
                    DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` references missing candidate `{}`",
                        commit.candidate_hash()
                    ))
                })?;
            let actual_cand_hash = compute_candidate_hash(cand);
            if &actual_cand_hash != commit.candidate_hash() {
                return Err(DbError::CorruptedCommitChain(format!(
                    "candidate hash mismatch for `{}`: computed `{actual_cand_hash}`",
                    commit.candidate_hash()
                )));
            }

            let rcpt = state.receipts.get(commit.receipt_hash()).ok_or_else(|| {
                DbError::CorruptedCommitChain(format!(
                    "commit `{hash}` references missing receipt `{}`",
                    commit.receipt_hash()
                ))
            })?;
            let actual_rcpt_hash = compute_receipt_hash(rcpt);
            if &actual_rcpt_hash != commit.receipt_hash() {
                return Err(DbError::CorruptedCommitChain(format!(
                    "receipt hash mismatch for `{}`: computed `{actual_rcpt_hash}`",
                    commit.receipt_hash()
                )));
            }

            // Cross-binding consistency checks
            if cand.envelope_hash != *commit.envelope_hash() {
                return Err(DbError::CorruptedCommitChain(format!(
                    "candidate envelope `{}` does not match commit envelope `{}`",
                    cand.envelope_hash,
                    commit.envelope_hash()
                )));
            }

            if rcpt.candidate_hash != *commit.candidate_hash() {
                return Err(DbError::CorruptedCommitChain(format!(
                    "receipt candidate `{}` does not match commit candidate `{}`",
                    rcpt.candidate_hash,
                    commit.candidate_hash()
                )));
            }

            // Envelope base_root_hash consistency check for root-changing commits
            if commit.parent_root() != Some(commit.new_root())
                && env.base_root_hash.as_ref() != commit.parent_root()
            {
                return Err(DbError::CorruptedCommitChain(format!(
                    "root-changing commit `{hash}` parent_root `{:?}` does not match envelope base_root_hash `{:?}`",
                    commit.parent_root(),
                    env.base_root_hash
                )));
            }

            // Candidate expression source re-parse and expression_hash check
            let parsed_expr = axiom_parser::parse_expr(&cand.expression_source).map_err(|e| {
                DbError::CorruptedCommitChain(format!(
                    "commit `{hash}` candidate `{}` expression parse failure: {e}",
                    commit.candidate_hash()
                ))
            })?;
            let computed_expr_hash = compute_expr_hash(&parsed_expr);
            if computed_expr_hash != cand.expression_hash {
                return Err(DbError::CorruptedCommitChain(format!(
                    "commit `{hash}` candidate `{}` expression_hash mismatch: recorded `{}`, computed `{computed_expr_hash}`",
                    commit.candidate_hash(),
                    cand.expression_hash
                )));
            }

            // Deterministic replay derivation check for root-changing commits
            if commit.parent_root() != Some(commit.new_root()) {
                let parent_mod = match commit.parent_root() {
                    Some(pr) => state.modules.get(pr).ok_or_else(|| {
                        DbError::CorruptedCommitChain(format!(
                            "commit `{hash}` parent root module `{pr}` not found in database",
                        ))
                    })?,
                    None => &Module::default(),
                };

                let derived =
                    derive_candidate_module(parent_mod, env, &parsed_expr).map_err(|e| {
                        DbError::CorruptedCommitChain(format!(
                            "commit `{hash}` candidate replay derivation failed: {e}",
                        ))
                    })?;

                let derived_hash = compute_module_hash(&derived);
                if &derived_hash != commit.new_root() {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` new_root mismatch: recorded `{}`, replayed `{derived_hash}`",
                        commit.new_root()
                    )));
                }
            }

            ordered_commits.push(commit);
            curr_commit_hash = commit.parent_commit();
        }

        ordered_commits.reverse();

        // Genesis commit in mainline must not have parent_commit
        if let Some(genesis) = ordered_commits.first() {
            if genesis.parent_commit().is_some() {
                return Err(DbError::CorruptedCommitChain(format!(
                    "mainline genesis commit `{}` has unexpected parent_commit pointer",
                    genesis.commit_hash
                )));
            }
        }

        // Validate strictly sequential parent -> child causality
        for i in 1..ordered_commits.len() {
            let parent = ordered_commits[i - 1];
            let child = ordered_commits[i];

            let p_commit = child.parent_commit().ok_or_else(|| {
                DbError::CorruptedCommitChain(format!(
                    "child commit `{}` is missing parent_commit pointer",
                    child.commit_hash
                ))
            })?;
            if p_commit != &parent.commit_hash {
                return Err(DbError::CorruptedCommitChain(format!(
                    "discontinuous commit chain: child `{}` points to parent `{}`, expected `{}`",
                    child.commit_hash, p_commit, parent.commit_hash
                )));
            }

            let p_root = child.parent_root().ok_or_else(|| {
                DbError::CorruptedCommitChain(format!(
                    "child commit `{}` is missing parent_root pointer",
                    child.commit_hash
                ))
            })?;
            if p_root != parent.new_root() {
                return Err(DbError::CorruptedCommitChain(format!(
                    "discontinuous commit root: child `{}` points to parent root `{}`, expected `{}`",
                    child.commit_hash,
                    p_root,
                    parent.new_root()
                )));
            }
        }

        Ok(ordered_commits)
    }

    /// Performs causal topology healing and complete hash recalculation for legacy unversioned databases.
    ///
    /// In legacy databases:
    /// 1. `parent_commit` field is physically missing from JSON, causing hash calculation discrepancies.
    /// 2. `head_commit` pointer is uninitialized.
    ///
    /// This method:
    /// - Identifies the single unambiguous Genesis commit (whose `parent_root` is not produced by any commit).
    /// - Linearly orders commits following root causality:
    ///   `Producer(R) -> NoOpCommits(R) (ordered by timestamp) -> AdvanceCommit(R -> R')`.
    /// - Fails closed if topology has ambiguous branches or unreachable orphans.
    /// - Links `parent_commit` pointers and recalculates `commit_hash` for EVERY commit starting from Genesis.
    /// - Updates the commits map keys and points `state.head_commit` to the final commit.
    pub(crate) fn repair_legacy_ancestry(state: &mut DatabaseState) -> Result<bool, DbError> {
        if state.head_commit.is_some() || state.commits.is_empty() {
            return Ok(false);
        }

        // 预验真：按 LegacyCommitPayload（旧 6 字段）严格核算原有哈希与 map_key，防止损坏数据被洗白
        for (map_key, commit) in &state.commits {
            if map_key != &commit.commit_hash {
                return Err(DbError::CorruptedCommitChain(format!(
                    "legacy commits map key `{map_key}` does not match commit_hash `{}`",
                    commit.commit_hash
                )));
            }

            let legacy_payload = LegacyCommitPayload {
                parent_root: commit.parent_root().cloned(),
                new_root: commit.new_root().clone(),
                envelope_hash: commit.envelope_hash().clone(),
                candidate_hash: commit.candidate_hash().clone(),
                receipt_hash: commit.receipt_hash().clone(),
                timestamp: commit.timestamp(),
            };
            let computed_legacy_hash = compute_legacy_commit_payload_hash(&legacy_payload);
            if computed_legacy_hash != commit.commit_hash {
                return Err(DbError::CorruptedCommitChain(format!(
                    "legacy commit hash mismatch for `{}`: computed legacy hash `{computed_legacy_hash}`",
                    commit.commit_hash
                )));
            }
        }

        let commit_list: Vec<CommitRecord> = state.commits.values().cloned().collect();
        let all_new_roots: HashSet<&NodeHash> = commit_list.iter().map(|c| c.new_root()).collect();

        // Genesis commit: parent_root is None or not produced by any other commit in database
        let genesis_candidates: Vec<&CommitRecord> = commit_list
            .iter()
            .filter(|c| {
                c.parent_root()
                    .map(|pr| !all_new_roots.contains(pr))
                    .unwrap_or(true)
            })
            .collect();

        if genesis_candidates.len() != 1 {
            return Err(DbError::CorruptedCommitChain(format!(
                "legacy database migration failed: found {} genesis candidates, expected exactly 1",
                genesis_candidates.len()
            )));
        }

        let genesis = genesis_candidates[0];
        let mut sorted_commits = Vec::with_capacity(commit_list.len());
        let mut visited: HashSet<NodeHash> = HashSet::new();

        sorted_commits.push(genesis.clone());
        visited.insert(genesis.commit_hash.clone());

        let mut curr_root = genesis.new_root().clone();

        loop {
            // Find all unvisited commits whose parent_root == curr_root
            let candidates: Vec<&CommitRecord> = commit_list
                .iter()
                .filter(|c| {
                    !visited.contains(&c.commit_hash) && c.parent_root() == Some(&curr_root)
                })
                .collect();

            if candidates.is_empty() {
                break;
            }

            // Separate into no-op commits (parent_root == new_root) and advance commits (new_root != parent_root)
            let mut noops: Vec<&CommitRecord> = candidates
                .iter()
                .filter(|c| c.new_root() == &curr_root)
                .cloned()
                .collect();
            let advances: Vec<&CommitRecord> = candidates
                .iter()
                .filter(|c| c.new_root() != &curr_root)
                .cloned()
                .collect();

            if advances.len() > 1 {
                return Err(DbError::CorruptedCommitChain(format!(
                    "legacy database migration failed: ambiguous branch forks detected from root `{curr_root}`"
                )));
            }

            // Order no-op commits deterministically by timestamp then commit_hash
            noops.sort_by_key(|c| (c.timestamp(), c.commit_hash.clone()));
            for noop in noops {
                visited.insert(noop.commit_hash.clone());
                sorted_commits.push(noop.clone());
            }

            if let Some(advance) = advances.first() {
                visited.insert(advance.commit_hash.clone());
                curr_root = advance.new_root().clone();
                sorted_commits.push((*advance).clone());
            } else {
                // No advance commit from this root
                break;
            }
        }

        // Fail-closed if there are orphan commits that could not be causally connected
        if sorted_commits.len() != commit_list.len() {
            return Err(DbError::CorruptedCommitChain(format!(
                "legacy database migration failed: {} unlinked or orphaned commits detected",
                commit_list.len() - sorted_commits.len()
            )));
        }

        // Recalculate commit_hash for EVERY commit (including Genesis) with updated payload
        let mut prev_commit_hash: Option<NodeHash> = None;
        let mut new_commits = BTreeMap::new();

        for mut c in sorted_commits {
            c.payload.parent_commit = prev_commit_hash.clone();
            c.commit_hash = compute_commit_payload_hash(&c.payload);
            prev_commit_hash = Some(c.commit_hash.clone());
            new_commits.insert(c.commit_hash.clone(), c);
        }

        state.commits = new_commits;
        state.head_commit = prev_commit_hash;

        Ok(true)
    }
}
