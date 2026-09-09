//! Integration with external Z3 / SMT-LIB2 solvers.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

/// Locates the Z3 executable on the system.
/// Checks `AXIOM_Z3_PATH`, system `PATH`, and default user install paths.
pub fn find_z3_executable() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("AXIOM_Z3_PATH") {
        let p = PathBuf::from(env_path);
        if p.is_file() {
            return Some(p);
        }
    }

    // Check system PATH by probing `z3 --version`
    if let Ok(output) = Command::new("z3").arg("--version").output() {
        if output.status.success() {
            return Some(PathBuf::from("z3"));
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
            return Some(path.clone());
        }
    }

    None
}

/// Retrieves the dynamically detected version string of Z3 executable.
pub fn get_z3_version(z3_path: &Path) -> String {
    if let Ok(output) = Command::new(z3_path).arg("--version").output() {
        let s = String::from_utf8_lossy(&output.stdout);
        if let Some(pos) = s.find("version ") {
            let rest = &s[pos + 8..];
            let ver = rest.split_whitespace().next().unwrap_or("").trim();
            if !ver.is_empty() {
                return ver.to_string();
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
    let mut child = Command::new(z3_path)
        .arg("-in")
        .arg("-smt2")
        .arg(format!("-t:{}", timeout_ms))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn z3 at {}: {}", z3_path.display(), e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(smt2_query.as_bytes())
            .map_err(|e| format!("failed to write query to z3 stdin: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for z3: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stderr.is_empty() && !output.status.success() {
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
