//! Integration with external Z3 / SMT-LIB2 solvers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::process_supervisor::{execute_process_until_deadline, TimeoutPolicy};
use crate::CounterExample;

/// Authoritative identity and cryptographic fingerprint of the verification solver backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverIdentity {
    /// Solver backend descriptor (e.g. "Microsoft Z3 (SMT-LIB2)" or "internal_symbolic")
    pub backend: String,
    /// Absolute canonical path to the solver binary, if an external executable is used.
    pub path: Option<String>,
    /// Semantic version string returned by the solver.
    pub version: String,
    /// Cryptographic SHA-256 digest of the solver binary file.
    pub sha256: Option<String>,
    /// Whether the solver binary was located and is ready for execution.
    pub available: bool,
}

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

/// Queries and returns the canonical identity and cryptographic fingerprint of the system solver.
pub fn resolve_solver_identity() -> SolverIdentity {
    if let Some(z3_path) = find_z3_executable() {
        let version = get_z3_version(&z3_path);
        let sha256 = compute_file_sha256(&z3_path);
        let path_str = z3_path.to_string_lossy().to_string();
        SolverIdentity {
            backend: "Microsoft Z3 (SMT-LIB2)".to_string(),
            path: Some(path_str),
            version,
            sha256,
            available: true,
        }
    } else {
        SolverIdentity {
            backend: "internal_symbolic".to_string(),
            path: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            sha256: None,
            available: false,
        }
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
    // [Spec 4] 全局精确绝对截止时间控制，严格等于调用方设定的 timeout_ms
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms as u64);
    let poll_interval = std::time::Duration::from_millis(5);

    let mut cmd = Command::new(z3_path);
    cmd.arg("-in")
        .arg("-smt2")
        .arg("-model")
        .arg(format!("-t:{}", timeout_ms));

    // 移除脚本中手写的 `(get-model)`。
    // 在 SMT-LIB2 标准中，若证明不可满足 (unsat)，显式执行 (get-model) 会触发
    // "model is not available" 错误并使 Z3 进程以非零状态码退出。
    // 配合命令行参数 `-model`，Z3 会在满足 (sat) 时自动输出模型，
    // 而在证明成立 (unsat) 时干净地以状态码 0 退出，从而支持严格 fail-closed 校验 out.status.success()。
    let cleaned_query = smt2_query.replace("(get-model)", "");

    let out = execute_process_until_deadline(
        cmd,
        Some(cleaned_query.as_bytes()),
        deadline,
        poll_interval,
    )?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // [P1 Spec 1] 强制 fail-closed 退出状态校验：任何非 0 退出（SIGSEGV、OOM、崩溃等）直接拒识为 Err
    if !out.status.success() {
        let code_str = match out.status.code() {
            Some(c) => format!("exit code {}", c),
            None => "terminated by signal".to_string(),
        };
        return Err(format!(
            "z3 process failed with {}: {}",
            code_str,
            stderr.trim()
        ));
    }

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

/// A sealed, read-only solver binary stored in a content-addressed sandbox path.
/// Eliminates TOCTOU completely by ensuring that version probe, vacuity checks,
/// and SMT-LIB2 queries all execute against this immutable, byte-verified sandbox file.
/// 不可变的沙箱求解器密封产物（仅包含内容寻址的字段，解耦调用方来源路径）。
#[derive(Debug, Clone)]
pub struct SealedSolverArtifact {
    pub sealed_path: PathBuf,
    pub sha256: String,
    pub version: String,
}

/// A sealed, read-only solver binary stored in a content-addressed sandbox path.
/// Eliminates TOCTOU completely by ensuring that version probe, vacuity checks,
/// and SMT-LIB2 queries all execute against this immutable, byte-verified sandbox file.
#[derive(Debug, Clone)]
pub struct SealedSolverBinary {
    pub source_path: PathBuf,
    pub sealed_path: PathBuf,
    pub sha256: String,
    pub version: String,
}

static SEAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static SEALED_CACHE: std::sync::OnceLock<std::sync::RwLock<HashMap<String, SealedSolverArtifact>>> =
    std::sync::OnceLock::new();

fn get_sealed_cache() -> &'static std::sync::RwLock<HashMap<String, SealedSolverArtifact>> {
    SEALED_CACHE.get_or_init(|| std::sync::RwLock::new(HashMap::new()))
}

#[cfg(windows)]
mod os_file_lock {
    use std::fs::File;
    use std::os::windows::io::AsRawHandle;

    const LOCKFILE_EXCLUSIVE_LOCK: u32 = 0x00000002;
    const LOCKFILE_FAIL_IMMEDIATELY: u32 = 0x00000001;

    extern "system" {
        fn LockFileEx(
            hFile: *mut std::ffi::c_void,
            dwFlags: u32,
            dwReserved: u32,
            nNumberOfBytesToLockLow: u32,
            nNumberOfBytesToLockHigh: u32,
            lpOverlapped: *mut std::ffi::c_void,
        ) -> i32;
        fn UnlockFileEx(
            hFile: *mut std::ffi::c_void,
            dwReserved: u32,
            nNumberOfBytesToLockLow: u32,
            nNumberOfBytesToLockHigh: u32,
            lpOverlapped: *mut std::ffi::c_void,
        ) -> i32;
    }

    pub fn try_lock_exclusive(file: &File) -> bool {
        let handle = file.as_raw_handle();
        let mut overlapped = [0u8; 32];
        let res = unsafe {
            LockFileEx(
                handle,
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                1,
                0,
                overlapped.as_mut_ptr() as *mut std::ffi::c_void,
            )
        };
        res != 0
    }

    pub fn unlock(file: &File) {
        let handle = file.as_raw_handle();
        let mut overlapped = [0u8; 32];
        unsafe {
            let _ = UnlockFileEx(
                handle,
                0,
                1,
                0,
                overlapped.as_mut_ptr() as *mut std::ffi::c_void,
            );
        }
    }
}

#[cfg(unix)]
mod os_file_lock {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;

    pub fn try_lock_exclusive(file: &File) -> bool {
        let fd = file.as_raw_fd();
        unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) == 0 }
    }

    pub fn unlock(file: &File) {
        let fd = file.as_raw_fd();
        unsafe {
            let _ = libc::flock(fd, libc::LOCK_UN);
        }
    }
}

#[cfg(not(any(windows, unix)))]
mod os_file_lock {
    use std::fs::File;
    pub fn try_lock_exclusive(_file: &File) -> bool {
        true
    }
    pub fn unlock(_file: &File) {}
}

struct CrossProcessFileLock {
    file: std::fs::File,
}

impl Drop for CrossProcessFileLock {
    fn drop(&mut self) {
        os_file_lock::unlock(&self.file);
    }
}

impl SealedSolverBinary {
    /// Copies and seals a solver executable into an immutable, SHA-256-verified sandbox directory.
    /// Employs a process-wide mutex, high-entropy unique temporary files, and an "AlreadyExists
    /// verify-and-reuse winner" atomic protocol to ensure multi-threaded and multi-process concurrency safety.
    pub fn seal_from_path(source_path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(source_path).map_err(|e| {
            format!(
                "failed to read solver binary at {}: {e}",
                source_path.display()
            )
        })?;

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256 = format!("{:x}", hasher.finalize());

        // 1. 快路径：先查进程内读写锁只读缓存，命中且物理文件有效直接现场组装传入的 source_path 返回
        if let Ok(cache) = get_sealed_cache().read() {
            if let Some(artifact) = cache.get(&sha256) {
                if artifact.sealed_path.is_file()
                    && compute_file_sha256(&artifact.sealed_path).as_deref() == Some(&sha256)
                {
                    return Ok(Self {
                        source_path: source_path.to_path_buf(),
                        sealed_path: artifact.sealed_path.clone(),
                        sha256: artifact.sha256.clone(),
                        version: artifact.version.clone(),
                    });
                }
            }
        }

        // 2. 慢路径：获取进程内互斥锁进行二次检测
        let _guard = SEAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(cache) = get_sealed_cache().read() {
            if let Some(artifact) = cache.get(&sha256) {
                if artifact.sealed_path.is_file()
                    && compute_file_sha256(&artifact.sealed_path).as_deref() == Some(&sha256)
                {
                    return Ok(Self {
                        source_path: source_path.to_path_buf(),
                        sealed_path: artifact.sealed_path.clone(),
                        sha256: artifact.sha256.clone(),
                        version: artifact.version.clone(),
                    });
                }
            }
        }

        let temp_dir = std::env::temp_dir();
        let sandbox_dir = temp_dir.join("axiom_sealed_solvers").join(&sha256);
        let filename = if cfg!(windows) { "z3.exe" } else { "z3" };
        std::fs::create_dir_all(&sandbox_dir)
            .map_err(|e| format!("failed to create sealed solver directory: {e}"))?;

        let sealed_path = sandbox_dir.join(filename);

        // 3. 跨进程排他锁协议 (基于操作系统内核文件锁，进程退出/异常终止时由 OS 内核自动释放，杜绝 ABA 误删与慢进程驱逐)
        let lock_file_path = sandbox_dir.join(".seal.lock");
        let lock_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_file_path)
            .map_err(|e| {
                format!(
                    "failed to open cross-process lock file at {}: {e}",
                    lock_file_path.display()
                )
            })?;

        let lock_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut _file_guard = None;
        while std::time::Instant::now() < lock_deadline {
            if os_file_lock::try_lock_exclusive(&lock_file) {
                _file_guard = Some(CrossProcessFileLock { file: lock_file });
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // 强锁保障：若超时未拿到跨进程锁，绝不无锁进入临界区，必须 Fail-Closed 返回错误
        if _file_guard.is_none() {
            return Err(format!(
                "failed to acquire cross-process lock for sealed solver at {} within deadline",
                lock_file_path.display()
            ));
        }

        // 4. “绝不破坏赢家”协议：锁内首先检查目标文件是否已存在且 SHA-256 完全吻合
        let already_valid =
            sealed_path.is_file() && compute_file_sha256(&sealed_path).as_deref() == Some(&sha256);

        if !already_valid {
            // 仅在目标文件不存在或内容已损坏时，清理并原子发布新文件
            if sealed_path.exists() {
                let mut perms = std::fs::metadata(&sealed_path)
                    .map(|m| m.permissions())
                    .ok();
                if let Some(ref mut p) = perms {
                    #[cfg(windows)]
                    p.set_readonly(false);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        p.set_mode(0o700);
                    }
                    let _ = std::fs::set_permissions(&sealed_path, p.clone());
                }
                let _ = std::fs::remove_file(&sealed_path);
            }

            let thread_id = format!("{:?}", std::thread::current().id());
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let temp_write_path = sandbox_dir.join(format!(
                "tmp_{}_{}_{}.bin",
                std::process::id(),
                thread_id.replace(|c: char| !c.is_alphanumeric(), "_"),
                nanos
            ));

            std::fs::write(&temp_write_path, &bytes)
                .map_err(|e| format!("failed to write sealed solver binary: {e}"))?;

            let mut perms = std::fs::metadata(&temp_write_path)
                .map_err(|e| e.to_string())?
                .permissions();
            #[cfg(windows)]
            perms.set_readonly(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                perms.set_mode(0o500);
            }
            let _ = std::fs::set_permissions(&temp_write_path, perms);

            if let Err(e) = std::fs::rename(&temp_write_path, &sealed_path) {
                let _ = std::fs::remove_file(&temp_write_path);
                // 检查是否已被并发竞争赢家成功发布且 SHA-256 完全匹配
                if sealed_path.is_file()
                    && compute_file_sha256(&sealed_path).as_deref() == Some(&sha256)
                {
                    // 并发发布赢家校验通过，优雅复用
                } else {
                    return Err(format!("failed to commit sealed solver binary: {e}"));
                }
            }
        }

        let verified_sha = compute_file_sha256(&sealed_path)
            .ok_or_else(|| "failed to hash sealed solver binary".to_string())?;
        if verified_sha != sha256 {
            return Err(format!(
                "sealed solver binary hash mismatch: expected {sha256}, got {verified_sha}"
            ));
        }

        let version = get_z3_version(&sealed_path);
        if version.is_empty() || version.eq_ignore_ascii_case("unknown") {
            return Err(format!(
                "sealed solver binary at {} failed version probe or reported unknown",
                sealed_path.display()
            ));
        }

        // 5. 写入进程内只读内容缓存（解耦 source_path）
        let artifact = SealedSolverArtifact {
            sealed_path: sealed_path.clone(),
            sha256: sha256.clone(),
            version: version.clone(),
        };

        if let Ok(mut cache) = get_sealed_cache().write() {
            cache.insert(sha256.clone(), artifact);
        }

        let sealed = Self {
            source_path: source_path.to_path_buf(),
            sealed_path,
            sha256,
            version,
        };

        Ok(sealed)
    }

    /// Verifies post-execution cryptographic integrity of the sealed binary file.
    pub fn verify_post_integrity(&self) -> bool {
        match compute_file_sha256(&self.sealed_path) {
            Some(ref h) => h == &self.sha256,
            None => false,
        }
    }
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

    #[test]
    fn test_solver_identity_resolution() {
        let id = resolve_solver_identity();
        assert!(!id.backend.is_empty());
        assert!(!id.version.is_empty());
        if id.available {
            assert!(id.path.is_some());
            assert!(id.sha256.is_some());
        }
    }

    #[test]
    fn test_run_z3_query_non_zero_exit_fails_closed() {
        let temp_dir = std::env::temp_dir();
        #[cfg(windows)]
        let script_path = temp_dir.join("axiom_test_fail_solver.cmd");
        #[cfg(not(windows))]
        let script_path = temp_dir.join("axiom_test_fail_solver.sh");

        #[cfg(windows)]
        std::fs::write(&script_path, "@exit 42\r\n").unwrap();
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&script_path, "#!/bin/sh\nexit 42\n").unwrap();
            let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms).unwrap();
        }

        let res = run_z3_query_with_timeout(&script_path, "(check-sat)", 2000);
        let _ = std::fs::remove_file(&script_path);

        assert!(res.is_err(), "Non-zero exit code must fail-closed");
        let err = res.unwrap_err();
        assert!(
            err.contains("42"),
            "Error message must report exit code 42, got: {}",
            err
        );
    }

    #[test]
    fn test_sealed_solver_binary_sandbox() {
        if let Some(z3_path) = find_z3_executable() {
            let original_sha = compute_file_sha256(&z3_path).expect("hash original z3");
            let sealed = SealedSolverBinary::seal_from_path(&z3_path)
                .expect("seal z3 executable should succeed");

            assert_eq!(sealed.sha256, original_sha);
            assert!(sealed.sealed_path.is_file());
            assert!(!sealed.version.is_empty() && !sealed.version.eq_ignore_ascii_case("unknown"));
            assert!(sealed.verify_post_integrity());

            // 再次密封同一文件验证幂等性与重用
            let sealed_cached = SealedSolverBinary::seal_from_path(&z3_path)
                .expect("re-sealing cached z3 should succeed");
            assert_eq!(sealed_cached.sha256, original_sha);
            assert_eq!(sealed_cached.sealed_path, sealed.sealed_path);
        }
    }

    #[test]
    fn test_sealed_solver_concurrent_seal() {
        if let Some(z3_path) = find_z3_executable() {
            let mut handles = Vec::new();
            for _ in 0..8 {
                let p = z3_path.clone();
                handles.push(std::thread::spawn(move || {
                    SealedSolverBinary::seal_from_path(&p)
                }));
            }

            let mut sealed_paths = Vec::new();
            for h in handles {
                let res = h.join().expect("thread should not panic");
                let sealed = res.expect("concurrent seal must succeed");
                assert!(sealed.verify_post_integrity());
                sealed_paths.push(sealed.sealed_path);
            }

            // 所有线程获得的 sealed_path 应当完全相同
            for p in &sealed_paths[1..] {
                assert_eq!(p, &sealed_paths[0]);
            }
        }
    }

    #[test]
    fn test_sealed_solver_provenance_decoupling() {
        if let Some(z3_path) = find_z3_executable() {
            let temp_dir = std::env::temp_dir();
            let copy_a = temp_dir.join("axiom_test_solver_a.exe");
            let copy_b = temp_dir.join("axiom_test_solver_b.exe");

            let _ = std::fs::copy(&z3_path, &copy_a);
            let _ = std::fs::copy(&z3_path, &copy_b);

            let sealed_a =
                SealedSolverBinary::seal_from_path(&copy_a).expect("seal copy a should succeed");
            let sealed_b =
                SealedSolverBinary::seal_from_path(&copy_b).expect("seal copy b should succeed");

            let _ = std::fs::remove_file(&copy_a);
            let _ = std::fs::remove_file(&copy_b);

            // 无论缓存是否命中，source_path 必须严格对应各自传入的真实来源路径，绝不可跨调用污染
            assert_eq!(sealed_a.source_path, copy_a);
            assert_eq!(sealed_b.source_path, copy_b);
            assert_ne!(sealed_a.source_path, sealed_b.source_path);

            // 但内容寻址的密封沙箱路径和哈希应当完全一致
            assert_eq!(sealed_a.sealed_path, sealed_b.sealed_path);
            assert_eq!(sealed_a.sha256, sealed_b.sha256);
        }
    }
}
