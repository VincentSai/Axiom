//! Commit graph topology traversal, integrity validation, and legacy migration.

use std::collections::{BTreeMap, HashSet};

use axiom_ast::Module;

use crate::{
    compute_candidate_hash, compute_commit_payload_hash, compute_envelope_hash, compute_expr_hash,
    compute_legacy_commit_payload_hash, compute_legacy_v2_commit_payload_hash,
    compute_legacy_v2_receipt_hash, compute_legacy_v3_receipt_hash, compute_module_hash,
    compute_proof_obligation_hash, compute_receipt_hash, derive_candidate_module, CommitRecord,
    DatabaseState, DbError, LegacyCommitPayload, LegacyV2CommitPayload,
    LegacyV2VerificationReceipt, LegacyV3VerificationReceipt, NodeHash, VerificationReceipt,
    CURRENT_SCHEMA_VERSION,
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
        let ordered_commits = Self::verify_commit_structure(state)?;
        Self::verify_artifact_bindings(&ordered_commits, state)?;
        Self::verify_semantic_replay(&ordered_commits, state)?;
        Ok(ordered_commits)
    }

    /// Layer 1: 验证主线提交图的因果拓扑结构、双向指针自洽性与防篡改自哈希
    fn verify_commit_structure(state: &DatabaseState) -> Result<Vec<&CommitRecord>, DbError> {
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

    /// Layer 2: 验证工件存在性、自哈希与交叉绑定；对 no-op 重签提交强校验历史真实 landing 绑定与证明义务
    fn verify_artifact_bindings(
        ordered_commits: &[&CommitRecord],
        state: &DatabaseState,
    ) -> Result<(), DbError> {
        let mut seen_commits: HashSet<&NodeHash> = HashSet::new();

        for commit in ordered_commits {
            let hash = &commit.commit_hash;

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

            // [Spec 1 核心加固]: 对于 parent_root == Some(new_root) 的 receipt-only no-op 提交，必须绑定历史真实落地提交
            if commit.parent_root() == Some(commit.new_root()) {
                let att_hash = commit.attested_commit_hash().ok_or_else(|| {
                    DbError::CorruptedCommitChain(format!(
                        "receipt-only commit `{hash}` missing required attested_commit_hash"
                    ))
                })?;

                if rcpt.attested_commit_hash.as_ref() != Some(att_hash) {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` attested_commit `{att_hash}` does not match receipt attested_commit `{:?}`",
                        rcpt.attested_commit_hash
                    )));
                }

                if !seen_commits.contains(att_hash) {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` attested_commit `{att_hash}` is not an ancestor in the active mainline history"
                    )));
                }

                let landing_commit = state.commits.get(att_hash).ok_or_else(|| {
                    DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` attested landing commit `{att_hash}` not found in database"
                    ))
                })?;

                if landing_commit.parent_root() == Some(landing_commit.new_root()) {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` attested landing commit `{att_hash}` is itself a receipt-only commit"
                    )));
                }

                if landing_commit.envelope_hash() != commit.envelope_hash() {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` envelope `{}` does not match attested landing commit envelope `{}`",
                        commit.envelope_hash(),
                        landing_commit.envelope_hash()
                    )));
                }

                if landing_commit.candidate_hash() != commit.candidate_hash() {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` candidate `{}` does not match attested landing commit candidate `{}`",
                        commit.candidate_hash(),
                        landing_commit.candidate_hash()
                    )));
                }

                if !rcpt
                    .verification_profile_hash
                    .as_str()
                    .starts_with("#vprof_")
                {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "receipt-only commit `{hash}` verification_profile `{}` has invalid format",
                        rcpt.verification_profile_hash
                    )));
                }

                let root_mod = state.modules.get(commit.new_root()).ok_or_else(|| {
                    DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` root module `{}` not found in database",
                        commit.new_root()
                    ))
                })?;

                let target_fn = root_mod
                    .functions
                    .iter()
                    .find(|f| f.name == env.target_function)
                    .ok_or_else(|| {
                        DbError::CorruptedCommitChain(format!(
                            "commit `{hash}` target function `{}` not found in root module",
                            env.target_function
                        ))
                    })?;

                if axiom_ast::expr_contains_hole(&target_fn.body) {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` target function `{}` contains unresolved holes",
                        env.target_function
                    )));
                }

                let computed_obl_hash = compute_proof_obligation_hash(target_fn);
                if computed_obl_hash != rcpt.proof_obligation_hash {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` proof_obligation_hash mismatch: recorded `{}`, computed `{computed_obl_hash}`",
                        rcpt.proof_obligation_hash
                    )));
                }
            }

            seen_commits.insert(hash);
        }

        Ok(())
    }

    /// Layer 3: 验证语义重放与 AST 确定性推导一致性
    fn verify_semantic_replay(
        ordered_commits: &[&CommitRecord],
        state: &DatabaseState,
    ) -> Result<(), DbError> {
        for commit in ordered_commits {
            let hash = &commit.commit_hash;

            // Envelope base_root_hash consistency check for root-changing commits
            if commit.parent_root() != Some(commit.new_root()) {
                let env = state.envelopes.get(commit.envelope_hash()).ok_or_else(|| {
                    DbError::CorruptedCommitChain(format!(
                        "commit `{hash}` missing task envelope `{}`",
                        commit.envelope_hash()
                    ))
                })?;

                if env.base_root_hash.as_ref() != commit.parent_root() {
                    return Err(DbError::CorruptedCommitChain(format!(
                        "root-changing commit `{hash}` parent_root `{:?}` does not match envelope base_root_hash `{:?}`",
                        commit.parent_root(),
                        env.base_root_hash
                    )));
                }

                let cand = state
                    .candidates
                    .get(commit.candidate_hash())
                    .ok_or_else(|| {
                        DbError::CorruptedCommitChain(format!(
                            "commit `{hash}` missing candidate `{}`",
                            commit.candidate_hash()
                        ))
                    })?;

                let parsed_expr =
                    axiom_parser::parse_expr(&cand.expression_source).map_err(|e| {
                        DbError::CorruptedCommitChain(format!(
                            "commit `{hash}` candidate expression parse failure: {e}"
                        ))
                    })?;

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
        }

        Ok(())
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

    /// 针对 Commit 4492184（Schema V2）数据库执行自底向上自愈迁移：
    /// 1. 预验真（防洗白）：使用 LegacyV2CommitPayload 与 LegacyV2VerificationReceipt 核验全库原始哈希，
    ///    若存在任何被篡改数据，立即拒绝迁移并返回 CorruptedCommitChain；
    /// 2. 拓扑扫描：从 head_commit 追溯至 Genesis，对每个 no-op 提交定位其在主线上的前驱 landing commit；
    /// 3. 收据与提交升级：为 no-op 收据与 commit payload 补齐 attested_commit_hash 语义绑定并重算收据哈希；
    /// 4. 重算全链哈希：自底向上重新计算全链 commit_hash，更新 parent_commit 指针、head_commit 以及 schema_version。
    pub(crate) fn repair_v2_migration(state: &mut DatabaseState) -> Result<bool, DbError> {
        if state.commits.is_empty() {
            state.schema_version = Some(CURRENT_SCHEMA_VERSION);
            return Ok(false);
        }

        let head_hash = match &state.head_commit {
            Some(h) => h,
            None => {
                return Err(DbError::CorruptedCommitChain(
                    "V2 database migration requires a non-empty head_commit".to_string(),
                ));
            }
        };

        // 步骤 1: 预验真 (防洗白)
        // 1.1 校验 receipts
        for (map_key, rcpt) in &state.receipts {
            let is_v4_or_v3_valid = compute_receipt_hash(rcpt) == *map_key;
            let is_legacy_v3_valid = {
                let leg_v3 = LegacyV3VerificationReceipt {
                    candidate_hash: rcpt.candidate_hash.clone(),
                    proof_level: rcpt.proof_level,
                    solver: rcpt.solver.clone(),
                    solver_version: rcpt.solver_version.clone(),
                    obligations_checked: rcpt.obligations_checked,
                    proof_obligation_hash: rcpt.proof_obligation_hash.clone(),
                    verification_profile_hash: rcpt.verification_profile_hash.clone(),
                    attested_commit_hash: rcpt.attested_commit_hash.clone(),
                    verified_at: rcpt.verified_at,
                };
                compute_legacy_v3_receipt_hash(&leg_v3) == *map_key
            };
            let is_v2_valid = {
                let legacy_rcpt = LegacyV2VerificationReceipt {
                    candidate_hash: rcpt.candidate_hash.clone(),
                    proof_level: rcpt.proof_level,
                    solver: rcpt.solver.clone(),
                    solver_version: rcpt.solver_version.clone(),
                    obligations_checked: rcpt.obligations_checked,
                    proof_obligation_hash: rcpt.proof_obligation_hash.clone(),
                    verification_profile_hash: rcpt.verification_profile_hash.clone(),
                    verified_at: rcpt.verified_at,
                };
                compute_legacy_v2_receipt_hash(&legacy_rcpt) == *map_key
            };
            if !is_v4_or_v3_valid && !is_legacy_v3_valid && !is_v2_valid {
                return Err(DbError::CorruptedCommitChain(format!(
                    "legacy receipt hash mismatch for `{map_key}`: neither valid V4/V3 nor valid V2"
                )));
            }
        }

        // 1.2 校验 commits
        for (map_key, commit) in &state.commits {
            if map_key != &commit.commit_hash {
                return Err(DbError::CorruptedCommitChain(format!(
                    "commits map key `{map_key}` does not match commit_hash `{}`",
                    commit.commit_hash
                )));
            }
            let is_v3_valid = compute_commit_payload_hash(&commit.payload) == commit.commit_hash;
            let is_v2_valid = {
                let legacy_commit = LegacyV2CommitPayload {
                    parent_commit: commit.parent_commit().cloned(),
                    parent_root: commit.parent_root().cloned(),
                    new_root: commit.new_root().clone(),
                    envelope_hash: commit.envelope_hash().clone(),
                    candidate_hash: commit.candidate_hash().clone(),
                    receipt_hash: commit.receipt_hash().clone(),
                    timestamp: commit.timestamp(),
                };
                compute_legacy_v2_commit_payload_hash(&legacy_commit) == commit.commit_hash
            };
            if !is_v3_valid && !is_v2_valid {
                return Err(DbError::CorruptedCommitChain(format!(
                    "legacy commit hash mismatch for `{}`: neither valid V3 nor valid V2",
                    commit.commit_hash
                )));
            }
        }

        // 步骤 2: 沿 parent_commit 回溯构建主线序列并检测孤儿
        let mut curr_opt = Some(head_hash);
        let mut backwards = Vec::new();
        let mut visited = HashSet::new();

        while let Some(curr_hash) = curr_opt {
            if !visited.insert(curr_hash.clone()) {
                return Err(DbError::CorruptedCommitChain(format!(
                    "cycle detected in parent_commit pointer at `{curr_hash}`"
                )));
            }
            let commit = state.commits.get(curr_hash).ok_or_else(|| {
                DbError::CorruptedCommitChain(format!(
                    "parent_commit `{curr_hash}` not found in database commits"
                ))
            })?;
            backwards.push(commit.clone());
            curr_opt = commit.parent_commit();
        }

        if backwards.len() != state.commits.len() {
            return Err(DbError::CorruptedCommitChain(format!(
                "legacy database migration failed: {} unlinked or orphaned commits detected",
                state.commits.len() - backwards.len()
            )));
        }

        backwards.reverse();
        let ordered_commits = backwards;

        // 步骤 3 & 4: 自底向上重构收据、补齐 landing 绑定与重算全链 commit hash
        let mut latest_landing: std::collections::HashMap<(NodeHash, NodeHash), NodeHash> =
            std::collections::HashMap::new();
        let mut new_receipts: BTreeMap<NodeHash, VerificationReceipt> = BTreeMap::new();
        let mut new_commits = BTreeMap::new();
        let mut prev_commit_hash: Option<NodeHash> = None;

        for mut commit in ordered_commits {
            let orig_rcpt = state
                .receipts
                .get(commit.receipt_hash())
                .ok_or_else(|| {
                    DbError::CorruptedCommitChain(format!(
                        "receipt `{}` not found in database",
                        commit.receipt_hash()
                    ))
                })?
                .clone();

            let is_noop = commit.parent_root() == Some(commit.new_root());

            let (att_commit_hash, new_rcpt_hash) = if is_noop {
                let landing_key = (
                    commit.envelope_hash().clone(),
                    commit.candidate_hash().clone(),
                );
                let landing_hash = latest_landing.get(&landing_key).ok_or_else(|| {
                    DbError::CorruptedCommitChain(format!(
                        "no-op commit `{}` has no preceding landing commit with envelope `{}` and candidate `{}`",
                        commit.commit_hash,
                        commit.envelope_hash(),
                        commit.candidate_hash()
                    ))
                })?;

                let mut upgraded_rcpt = orig_rcpt;
                upgraded_rcpt.attested_commit_hash = Some(landing_hash.clone());
                let h = compute_receipt_hash(&upgraded_rcpt);
                new_receipts.insert(h.clone(), upgraded_rcpt);
                (Some(landing_hash.clone()), h)
            } else {
                let mut upgraded_rcpt = orig_rcpt;
                upgraded_rcpt.attested_commit_hash = None;
                let h = compute_receipt_hash(&upgraded_rcpt);
                new_receipts.insert(h.clone(), upgraded_rcpt);
                (None, h)
            };

            commit.payload.receipt_hash = new_rcpt_hash;
            commit.payload.parent_commit = prev_commit_hash.clone();
            commit.payload.attested_commit_hash = att_commit_hash;
            commit.commit_hash = compute_commit_payload_hash(&commit.payload);

            if !is_noop {
                latest_landing.insert(
                    (
                        commit.envelope_hash().clone(),
                        commit.candidate_hash().clone(),
                    ),
                    commit.commit_hash.clone(),
                );
            }

            prev_commit_hash = Some(commit.commit_hash.clone());
            new_commits.insert(commit.commit_hash.clone(), commit);
        }

        // 强不变式：校验重建后的所有 receipt 严格满足内容寻址
        for (k, v) in &new_receipts {
            let actual_hash = compute_receipt_hash(v);
            if actual_hash != *k {
                return Err(DbError::CorruptedCommitChain(format!(
                    "migrated receipt map key `{k}` does not match content hash `{actual_hash}`"
                )));
            }
        }

        // 强不变式：校验重建后的所有 commit 严格满足内容寻址
        for (k, v) in &new_commits {
            let actual_hash = compute_commit_payload_hash(&v.payload);
            if actual_hash != *k || v.commit_hash != *k {
                return Err(DbError::CorruptedCommitChain(format!(
                    "migrated commit map key `{k}` does not match payload hash `{actual_hash}`"
                )));
            }
        }

        state.receipts = new_receipts;
        state.commits = new_commits;
        state.head_commit = prev_commit_hash;
        state.schema_version = Some(CURRENT_SCHEMA_VERSION);

        Ok(true)
    }
}
