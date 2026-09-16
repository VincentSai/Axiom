//! SMT counter-example guided auto-healing workflow for Axiom holes.
//!
//! Provides automated single-step and iterative self-healing loops for hole tasks,
//! orchestrating external Agent/LLM engines (Mock, Codex, etc.) with strict formal SMT verification.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::{AgentTaskPool, HoleStatus, PoolError, SubmitStatus};
use axiom_hole_extractor::HoleTask;

/// Record of a single attempt and verification outcome during an auto-heal loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealStepRecord {
    pub step: usize,
    pub candidate_solution: String,
    pub status: SubmitStatus,
    pub counter_example: Option<String>,
    pub reason: Option<String>,
    pub proved: bool,
}

/// Aggregated outcome of an auto-heal workflow execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoHealOutcome {
    pub hole_id: String,
    pub success: bool,
    pub final_solution: Option<String>,
    pub total_steps: usize,
    pub history: Vec<HealStepRecord>,
}

/// Execute an automated solve-and-heal loop using a custom strategy closure.
///
/// `strategy` receives `(step_index, last_counter_example)` and returns an Axiom candidate expression.
/// If formal SMT verification proves the implementation and commits it via CAS, the loop terminates early.
pub fn solve_and_heal_with_strategy<F>(
    pool: &mut AgentTaskPool,
    hole_id: &str,
    max_steps: usize,
    mut strategy: F,
) -> Result<AutoHealOutcome, PoolError>
where
    F: FnMut(usize, Option<&str>) -> String,
{
    if pool.hole_status(hole_id).is_none() {
        return Err(PoolError::UnknownHole(hole_id.to_string()));
    }

    let mut history = Vec::new();
    let mut last_counter_example: Option<String> = None;
    let mut final_solution = None;
    let mut success = false;
    let mut total_steps = 0;

    for step in 0..max_steps {
        total_steps += 1;
        let candidate = strategy(step, last_counter_example.as_deref());

        match pool.submit_solution_with_outcome(hole_id, &candidate, "axiom-agent") {
            Ok(outcome) => {
                let dto = outcome.to_dto(hole_id);
                let is_verified =
                    dto.proved && pool.hole_status(hole_id) == Some(HoleStatus::Verified);
                let record = HealStepRecord {
                    step,
                    candidate_solution: candidate.clone(),
                    status: dto.status,
                    counter_example: dto.counter_example.clone(),
                    reason: dto.reason.clone(),
                    proved: dto.proved,
                };
                history.push(record);

                if is_verified {
                    final_solution = Some(candidate);
                    success = true;
                    break;
                }

                last_counter_example = dto.counter_example;
            }
            Err(PoolError::Parse(parse_err)) => {
                let record = HealStepRecord {
                    step,
                    candidate_solution: candidate.clone(),
                    status: SubmitStatus::Rejected,
                    counter_example: Some(format!("Syntax parse error: {parse_err}")),
                    reason: Some("Expression syntax parse error".to_string()),
                    proved: false,
                };
                last_counter_example = record.counter_example.clone();
                history.push(record);
            }
            Err(other_err) => {
                return Err(other_err);
            }
        }
    }

    Ok(AutoHealOutcome {
        hole_id: hole_id.to_string(),
        success,
        final_solution,
        total_steps,
        history,
    })
}

/// Locate codex binary executable on current system (supporting Windows and POSIX).
pub fn find_codex_binary() -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let direct = dir.join("codex");
            if direct.is_file() {
                return Some(direct);
            }
            #[cfg(windows)]
            {
                let cmd = dir.join("codex.cmd");
                if cmd.is_file() {
                    return Some(cmd);
                }
                let exe = dir.join("codex.exe");
                if exe.is_file() {
                    return Some(exe);
                }
            }
        }
    }

    #[cfg(windows)]
    {
        let default_cmd = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\npm\codex.cmd");
        if default_cmd.is_file() {
            return Some(default_cmd);
        }
    }

    None
}

/// Build standard prompt instructing an agent to solve a hole task, incorporating counter-examples when available.
pub fn build_hole_prompt(
    task: &HoleTask,
    last_candidate: Option<&str>,
    counter_example: Option<&str>,
) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are an expert formal verification coding agent writing pure expressions in the Axiom language.\n\n");
    prompt.push_str(&format!(
        "Target Function: {}\nTarget Hole: ?{}\nReturn Type: {}\nAllowed Effects: {:?}\n\n",
        task.target_function, task.hole_id, task.return_type, task.allowed_effects
    ));

    prompt.push_str("Parameters:\n");
    for param in &task.parameters {
        prompt.push_str(&format!("  - {}: {}\n", param.name, param.r#type));
    }

    prompt.push_str("\nFormal Contracts:\n");
    prompt.push_str("  Requires (Preconditions):\n");
    for req in &task.contracts.requires {
        prompt.push_str(&format!("    - {}\n", req));
    }
    if task.contracts.requires.is_empty() {
        prompt.push_str("    - None (free input domain)\n");
    }

    prompt.push_str("  Ensures (Postconditions):\n");
    for ens in &task.contracts.ensures {
        prompt.push_str(&format!("    - {}\n", ens));
    }

    if let Some(ce) = counter_example {
        prompt.push_str("\n==================================================\n");
        prompt.push_str("CRITICAL SMT VERIFICATION FAILURE FEEDBACK:\n");
        if let Some(cand) = last_candidate {
            prompt.push_str(&format!("Previous Candidate Expression: {}\n", cand));
        }
        prompt.push_str(&format!("Counter-Example Violating Contract: {}\n", ce));
        prompt.push_str(
            "The SMT solver mathematically proved your previous expression violates the ensures contract on this input!\n",
        );
        prompt.push_str(
            "HINT: Guard the boundary or negative cases using conditional match expressions, e.g.:\n",
        );
        prompt.push_str("  match param < 0 { true => -param, false => param }\n");
        prompt.push_str("==================================================\n\n");
    }

    prompt.push_str(
        "Instruction: Output ONLY the raw Axiom pure expression to fill the hole. DO NOT wrap with `fn`, DO NOT include explanations, markdown code blocks, or extra text.\n",
    );
    prompt
}

/// Strip markdown fences and whitespace from LLM output.
pub fn clean_expression_output(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(stripped) = trimmed.strip_prefix("```axiom") {
        if let Some(end) = stripped.strip_suffix("```") {
            return end.trim().to_string();
        }
    }
    if let Some(stripped) = trimmed.strip_prefix("```") {
        if let Some(end) = stripped.strip_suffix("```") {
            return end.trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Invoke local Codex-CLI non-interactively to generate a solution under ProcessSupervisor watchdog.
pub fn call_codex_cli(prompt: &str, codex_bin: &Path) -> Result<String, String> {
    let timeout_ms = std::env::var("AXIOM_CODEX_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(30_000);
    call_codex_cli_with_timeout(prompt, codex_bin, timeout_ms)
}

/// Invoke local Codex-CLI non-interactively with an explicit watchdog timeout in milliseconds.
pub fn call_codex_cli_with_timeout(
    prompt: &str,
    codex_bin: &Path,
    timeout_ms: u32,
) -> Result<String, String> {
    let mut cmd = Command::new(codex_bin);
    cmd.args([
        "exec",
        "--ephemeral",
        "--skip-git-repo-check",
        "--ignore-user-config",
        "-s",
        "read-only",
    ]);

    let policy = axiom_smt::TimeoutPolicy::from_millis(timeout_ms);
    let output = axiom_smt::execute_supervised_command(cmd, Some(prompt.as_bytes()), policy)
        .map_err(|e| format!("codex supervisor execution failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "codex execution failed (status: {:?}): {stderr}",
            output.status.code()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(clean_expression_output(&stdout))
}

/// Solve and self-heal using a named engine (`mock` or `codex`).
pub fn solve_and_heal_with_engine(
    pool: &mut AgentTaskPool,
    hole_id: &str,
    engine: &str,
    max_steps: usize,
) -> Result<AutoHealOutcome, PoolError> {
    let task = pool
        .task_for_hole(hole_id)
        .cloned()
        .ok_or_else(|| PoolError::UnknownHole(hole_id.to_string()))?;

    match engine {
        "mock" => {
            let param_name = task
                .parameters
                .first()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "x".to_string());

            solve_and_heal_with_strategy(pool, hole_id, max_steps, move |step, ce| {
                if step == 0 || ce.is_none() {
                    param_name.clone()
                } else {
                    // 反例自愈：构建具有分支防护的正确解
                    format!(
                        "match {} < 0 {{ true => -{}, false => {} }}",
                        param_name, param_name, param_name
                    )
                }
            })
        }
        "codex" => {
            let codex_bin = find_codex_binary().ok_or_else(|| {
                PoolError::CasFailure(
                    "codex-cli executable not found in PATH or npm default".into(),
                )
            })?;

            let mut last_cand: Option<String> = None;
            solve_and_heal_with_strategy(pool, hole_id, max_steps, move |_step, ce| {
                let prompt = build_hole_prompt(&task, last_cand.as_deref(), ce);
                match call_codex_cli(&prompt, &codex_bin) {
                    Ok(solution) => {
                        last_cand = Some(solution.clone());
                        solution
                    }
                    Err(err) => {
                        eprintln!("[Codex Engine Error] {err}");
                        "0".to_string()
                    }
                }
            })
        }
        other => Err(PoolError::CasFailure(format!(
            "unsupported auto-heal engine `{other}` (available: mock, codex)"
        ))),
    }
}
