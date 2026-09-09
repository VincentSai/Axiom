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

/// Unified process timeout and polling watchdog policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutPolicy {
    pub timeout: std::time::Duration,
    pub poll_interval: std::time::Duration,
}

impl TimeoutPolicy {
    pub const DEFAULT_VERSION_PROBE: Self = Self {
        timeout: std::time::Duration::from_millis(3000),
        poll_interval: std::time::Duration::from_millis(10),
    };

    pub const DEFAULT_SOLVER_QUERY: Self = Self {
        timeout: std::time::Duration::from_millis(10000),
        poll_interval: std::time::Duration::from_millis(20),
    };

    pub fn from_millis(ms: u32) -> Self {
        Self {
            timeout: std::time::Duration::from_millis(ms as u64),
            poll_interval: std::time::Duration::from_millis(20),
        }
    }

    /// Waits for the child process to exit, terminating it if it exceeds `self.timeout`.
    pub fn wait_child_with_watchdog(
        &self,
        child: &mut std::process::Child,
    ) -> Result<std::process::ExitStatus, String> {
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {
                    if start.elapsed() >= self.timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!(
                            "child process hard-killed: execution time exceeded timeout threshold of {}ms",
                            self.timeout.as_millis()
                        ));
                    }
                    std::thread::sleep(self.poll_interval);
                }
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("error polling child process: {}", e));
                }
            }
        }
    }
}

/// Retrieves the dynamically detected version string of Z3 executable with hard timeout watchdog.
pub fn get_z3_version(z3_path: &Path) -> String {
    let mut child = match Command::new(z3_path)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return "unknown".to_string(),
    };

    let policy = TimeoutPolicy::DEFAULT_VERSION_PROBE;
    match policy.wait_child_with_watchdog(&mut child) {
        Ok(status) if status.success() => {
            if let Some(mut out) = child.stdout.take() {
                use std::io::Read;
                let mut buf = Vec::new();
                let _ = out.read_to_end(&mut buf);
                let s = String::from_utf8_lossy(&buf);
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
        _ => "unknown".to_string(),
    }
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
    use std::io::Read;

    let mut child = Command::new(z3_path)
        .arg("-in")
        .arg("-smt2")
        .arg(format!("-t:{}", timeout_ms))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn z3 at {}: {}", z3_path.display(), e))?;

    let stdin_join = if let Some(mut stdin) = child.stdin.take() {
        let query_bytes = smt2_query.as_bytes().to_vec();
        Some(std::thread::spawn(move || {
            let res = stdin.write_all(&query_bytes);
            drop(stdin);
            res
        }))
    } else {
        None
    };

    let mut stdout_handle = child.stdout.take().ok_or("failed to take stdout")?;
    let mut stderr_handle = child.stderr.take().ok_or("failed to take stderr")?;

    let stdout_join = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_handle.read_to_end(&mut buf);
        buf
    });
    let stderr_join = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_handle.read_to_end(&mut buf);
        buf
    });

    let policy = TimeoutPolicy::from_millis(timeout_ms);
    let status_res = policy.wait_child_with_watchdog(&mut child);

    // [Spec 7 & Standards 2] 读写线程退出全局绝对截止时间控制与资源无泄漏回收
    let join_grace = std::time::Duration::from_millis(1500);
    let deadline = std::time::Instant::now() + join_grace;

    let mut io_timed_out = false;
    while !stdout_join.is_finished() || !stderr_join.is_finished() {
        if std::time::Instant::now() > deadline {
            io_timed_out = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    if !io_timed_out {
        if let Some(ref stdin_handle) = stdin_join {
            while !stdin_handle.is_finished() {
                if std::time::Instant::now() > deadline {
                    io_timed_out = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }

    // 若读写线程超时或子进程监控报错，立即强杀子进程切断管道，并强制回收线程句柄杜绝泄漏
    if io_timed_out || status_res.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        if let Some(stdin_handle) = stdin_join {
            let _ = stdin_handle.join();
        }
        let _ = stdout_join.join();
        let _ = stderr_join.join();

        if io_timed_out {
            return Err("timed out waiting for z3 reader/writer threads to exit".into());
        }
        status_res?;
        return Err("z3 process exited abnormally".into());
    }

    if let Some(stdin_handle) = stdin_join {
        match stdin_handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(io_err)) => {
                let _ = stdout_join.join();
                let _ = stderr_join.join();
                return Err(format!("failed to write query to z3 stdin: {}", io_err));
            }
            Err(_) => {
                let _ = stdout_join.join();
                let _ = stderr_join.join();
                return Err("z3 stdin writer thread panicked".into());
            }
        }
    }

    let stdout_bytes = stdout_join
        .join()
        .map_err(|_| "stdout reader thread panicked")?;
    let stderr_bytes = stderr_join
        .join()
        .map_err(|_| "stderr reader thread panicked")?;
    status_res?;

    let stdout = String::from_utf8_lossy(&stdout_bytes);
    let stderr = String::from_utf8_lossy(&stderr_bytes);

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
