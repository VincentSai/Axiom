//! Integration with external Z3 / SMT-LIB2 solvers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::process_supervisor::{execute_process_until_deadline, TimeoutPolicy};
use crate::CounterExample;

/// Result from invoking an external Z3 solver.
#[derive(Debug, Clone, PartialEq)]
pub enum Z3Result {
    /// Formula is unsatisfiable: the contract holds for all inputs!
    Unsat,
    /// Formula is satisfiable: a concrete counterexample violates the contract.
    Sat(CounterExample),
    /// Solver returned unknown or timed out.
    Unknown(String),
}

fn clean_canonicalize(path: &Path) -> PathBuf {
    if let Ok(canon) = std::fs::canonicalize(path) {
        let s = canon.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            PathBuf::from(stripped)
        } else {
            canon
        }
    } else {
        path.to_path_buf()
    }
}

fn resolve_executable_in_path(name: &str) -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            #[cfg(windows)]
            {
                let cand1 = dir.join(format!("{name}.exe"));
                if cand1.is_file() {
                    return Some(cand1);
                }
                let cand2 = dir.join(name);
                if cand2.is_file() {
                    return Some(cand2);
                }
            }
            #[cfg(not(windows))]
            {
                let cand = dir.join(name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// 计算指定二进制文件的密码学 SHA-256 哈希
pub fn compute_file_sha256(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    if let Ok(bytes) = std::fs::read(path) {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Some(format!("{:x}", hasher.finalize()))
    } else {
        None
    }
}

/// Locates the Z3 executable on the system.
/// Checks `AXIOM_Z3_PATH`, system `PATH` (with strict 2s timeout watchdog), and default user install paths.
/// Returns canonicalized absolute path to prevent provenance drift.
pub fn find_z3_executable() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("AXIOM_Z3_PATH") {
        let p = PathBuf::from(env_path);
        if p.is_file() {
            return Some(clean_canonicalize(&p));
        }
    }

    // [Spec 4] Check system PATH with hard deadline watchdog (prevents permanent hang on frozen z3)
    if let Some(path_in_env) = resolve_executable_in_path("z3") {
        let mut probe_cmd = Command::new(&path_in_env);
        probe_cmd.arg("--version");
        let probe_deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
        if let Ok(out) = execute_process_until_deadline(
            probe_cmd,
            None,
            probe_deadline,
            std::time::Duration::from_millis(10),
        ) {
            if out.status.success() {
                return Some(clean_canonicalize(&path_in_env));
            }
        }
    }

    // Common fallback directories on Windows / Unix
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let candidates = [
        PathBuf::from(&home)
            .join(".axiom")
            .join("bin")
            .join("z3.exe"),
        PathBuf::from(&home).join(".axiom").join("bin").join("z3"),
        PathBuf::from("C:\\Program Files\\z3\\bin\\z3.exe"),
        PathBuf::from("/usr/bin/z3"),
        PathBuf::from("/usr/local/bin/z3"),
        PathBuf::from("/opt/homebrew/bin/z3"),
    ];

    for path in &candidates {
        if path.is_file() {
            return Some(clean_canonicalize(path));
        }
    }

    None
}

/// Retrieves the dynamically detected version string of Z3 executable with hard timeout watchdog and async I/O.
pub fn get_z3_version(z3_path: &Path) -> String {
    let mut cmd = Command::new(z3_path);
    cmd.arg("--version");
    let deadline = std::time::Instant::now() + TimeoutPolicy::DEFAULT_VERSION_PROBE.timeout;
    let poll_interval = TimeoutPolicy::DEFAULT_VERSION_PROBE.poll_interval;

    if let Ok(out) = execute_process_until_deadline(cmd, None, deadline, poll_interval) {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(pos) = s.find("version ") {
                let rest = &s[pos + 8..];
                let ver = rest.split_whitespace().next().unwrap_or("").trim();
                if !ver.is_empty() {
                    return ver.to_string();
                }
            }
        }
    }

    "unknown".to_string()
}

/// Executes a verification query using the Z3 solver via standard input with a 10s timeout.
pub fn run_z3_query(z3_path: &Path, smt2_query: &str) -> Result<Z3Result, String> {
    run_z3_query_with_timeout(z3_path, smt2_query, 10000)
}

/// Executes a verification query using the Z3 solver with configurable hard timeout.
pub fn run_z3_query_with_timeout(
    z3_path: &Path,
    smt2_query: &str,
    timeout_ms: u32,
) -> Result<Z3Result, String> {
    // [Spec 4] 全局精确绝对截止时间控制，严格等于调用方设定的 timeout_ms
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms as u64);
    let poll_interval = std::time::Duration::from_millis(5);

    let mut cmd = Command::new(z3_path);
    cmd.arg("-in")
        .arg("-smt2")
        .arg(format!("-t:{}", timeout_ms));

    let out =
        execute_process_until_deadline(cmd, Some(smt2_query.as_bytes()), deadline, poll_interval)?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    if !stderr.is_empty() && stderr.contains("error") {
        return Err(format!("z3 error: {}", stderr.trim()));
    }

    parse_z3_output(&stdout)
}

/// Parses the output of `(check-sat)` and `(get-model)` from Z3.
pub fn parse_z3_output(stdout: &str) -> Result<Z3Result, String> {
    let trimmed = stdout.trim();
    if trimmed.starts_with("unsat") {
        return Ok(Z3Result::Unsat);
    }
    if trimmed.starts_with("unknown") || trimmed.starts_with("timeout") {
        return Ok(Z3Result::Unknown(trimmed.to_string()));
    }
    if trimmed.starts_with("sat") {
        let counter_example = parse_z3_model(trimmed);
        return Ok(Z3Result::Sat(counter_example));
    }

    Err(format!("unrecognized z3 output: {}", trimmed))
}

/// Extracts variable assignments from SMT-LIB2 model S-expressions.
/// e.g. `(model (define-fun x () Real 10.5) (define-fun y () Int (- 5)))`
fn parse_z3_model(output: &str) -> CounterExample {
    let mut model = HashMap::new();
    let lines: Vec<&str> = output.lines().collect();

    for i in 0..lines.len() {
        let line = lines[i].trim();
        if line.starts_with("(define-fun ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let var_name = parts[1].to_string();
                if var_name == "result" {
                    continue; // Skip the derived result variable
                }

                // Value might be on the same line or next line
                if let Some(val) = extract_numeric_value(line) {
                    model.insert(var_name, val);
                } else if i + 1 < lines.len() {
                    if let Some(val) = extract_numeric_value(lines[i + 1].trim()) {
                        model.insert(var_name, val);
                    }
                }
            }
        }
    }

    model
}

fn extract_numeric_value(text: &str) -> Option<f64> {
    let text = text.trim().trim_end_matches(')').trim();

    // Check for negative numbers like `(- 10.5)` or `(- 5)`
    if text.contains("(- ") {
        if let Some(start) = text.find("(- ") {
            let sub = &text[start + 3..].trim_end_matches(')').trim();
            if let Ok(n) = sub.parse::<f64>() {
                return Some(-n);
            }
        }
    }

    // Check for rational division like `(/ 1.0 2.0)`
    if text.contains("(/ ") {
        if let Some(start) = text.find("(/ ") {
            let sub = &text[start + 3..].trim_end_matches(')').trim();
            let div_parts: Vec<&str> = sub.split_whitespace().collect();
            if div_parts.len() == 2 {
                if let (Ok(num), Ok(den)) =
                    (div_parts[0].parse::<f64>(), div_parts[1].parse::<f64>())
                {
                    if den != 0.0 {
                        return Some(num / den);
                    }
                }
            }
        }
    }

    // Direct float/int parse
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if let Some(last) = tokens.last() {
        let cleaned = last.trim_matches(|c| c == '(' || c == ')');
        if let Ok(n) = cleaned.parse::<f64>() {
            return Some(n);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_execution_deadline_watchdog_kills_hung_process() {
        #[cfg(windows)]
        let mut cmd = Command::new("powershell");
        #[cfg(windows)]
        cmd.args(["-Command", "Start-Sleep -Seconds 10"]);

        #[cfg(unix)]
        let mut cmd = Command::new("sleep");
        #[cfg(unix)]
        cmd.arg("10");

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(150);
        let start = std::time::Instant::now();
        let res = execute_process_until_deadline(
            cmd,
            None,
            deadline,
            std::time::Duration::from_millis(10),
        );

        let elapsed = start.elapsed();
        assert!(
            res.is_err(),
            "expected process to be terminated due to deadline"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(3),
            "process watchdog must kill process promptly without hanging, elapsed: {:?}",
            elapsed
        );
    }
}
