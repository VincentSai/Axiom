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

/// Output from executing an external process with hard deadline and asynchronous I/O.
#[derive(Debug)]
pub(crate) struct ProcessExecutionOutput {
    pub(crate) status: std::process::ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

/// Locates the Z3 executable on the system.
/// Checks `AXIOM_Z3_PATH`, system `PATH` (with strict 2s timeout watchdog), and default user install paths.
pub fn find_z3_executable() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("AXIOM_Z3_PATH") {
        let p = PathBuf::from(env_path);
        if p.is_file() {
            return Some(p);
        }
    }

    // [Spec 4] Check system PATH with hard deadline watchdog (prevents permanent hang on frozen z3)
    let mut probe_cmd = Command::new("z3");
    probe_cmd.arg("--version");
    let probe_deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
    if let Ok(out) = execute_process_until_deadline(
        probe_cmd,
        None,
        probe_deadline,
        std::time::Duration::from_millis(10),
    ) {
        if out.status.success() {
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

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod win32_job {
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;

    type HANDLE = *mut std::ffi::c_void;
    type BOOL = i32;
    type DWORD = u32;

    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x0000_2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;

    #[repr(C)]
    struct IO_COUNTERS {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: DWORD,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: DWORD,
        affinity: usize,
        priority_class: DWORD,
        scheduling_class: DWORD,
    }

    #[repr(C)]
    struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
        io_info: IO_COUNTERS,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_limit: usize,
        peak_job_memory_limit: usize,
    }

    extern "system" {
        fn CreateJobObjectW(
            lp_job_attributes: *mut std::ffi::c_void,
            lp_name: *const u16,
        ) -> HANDLE;
        fn SetInformationJobObject(
            h_job: HANDLE,
            job_object_info_class: i32,
            lp_job_object_info: *const std::ffi::c_void,
            cb_job_object_info_length: DWORD,
        ) -> BOOL;
        fn AssignProcessToJobObject(h_job: HANDLE, h_process: HANDLE) -> BOOL;
        fn TerminateJobObject(h_job: HANDLE, u_exit_code: u32) -> BOOL;
        fn CloseHandle(h_object: HANDLE) -> BOOL;
    }

    pub struct JobObjectGuard {
        job_handle: HANDLE,
    }

    impl JobObjectGuard {
        pub fn new() -> Option<Self> {
            unsafe {
                let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
                if job.is_null() {
                    return None;
                }

                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

                let ok = SetInformationJobObject(
                    job,
                    JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as DWORD,
                );

                if ok == 0 {
                    CloseHandle(job);
                    return None;
                }

                Some(Self { job_handle: job })
            }
        }

        pub fn assign_process(&self, child: &Child) -> bool {
            let handle = child.as_raw_handle() as HANDLE;
            unsafe { AssignProcessToJobObject(self.job_handle, handle) != 0 }
        }

        pub fn terminate_now(&self) {
            unsafe {
                let _ = TerminateJobObject(self.job_handle, 1);
            }
        }
    }

    impl Drop for JobObjectGuard {
        fn drop(&mut self) {
            unsafe {
                if !self.job_handle.is_null() {
                    let _ = TerminateJobObject(self.job_handle, 1);
                    CloseHandle(self.job_handle);
                }
            }
        }
    }
}

#[inline]
fn configure_process_command(cmd: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let _ = cmd;
}

/// Forcefully terminates a child process and all its descendants to release pipe handles.
fn kill_process_tree(
    child: &mut std::process::Child,
    #[cfg(windows)] job_guard: Option<&win32_job::JobObjectGuard>,
) {
    #[cfg(windows)]
    {
        if let Some(guard) = job_guard {
            guard.terminate_now();
        }
        let pid = child.id();
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }
    #[cfg(unix)]
    {
        let pid = child.id();
        let _ = Command::new("kill")
            .args(["-9", &format!("-{}", pid)])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }
    let _ = child.kill();
    let kill_wait_deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
    while std::time::Instant::now() < kill_wait_deadline {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Unified process execution engine with asynchronous I/O and strict absolute deadline enforcement.
/// Kills process tree and all descendants immediately upon deadline expiry.
pub(crate) fn execute_process_until_deadline(
    mut cmd: Command,
    stdin_data: Option<&[u8]>,
    deadline: std::time::Instant,
    poll_interval: std::time::Duration,
) -> Result<ProcessExecutionOutput, String> {
    use std::io::Read;

    configure_process_command(&mut cmd);
    if stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(windows)]
    let job_guard = win32_job::JobObjectGuard::new();

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn process: {e}"))?;

    #[cfg(windows)]
    if let Some(ref guard) = job_guard {
        guard.assign_process(&child);
    }

    let stdin_join = if let Some(data) = stdin_data {
        if let Some(mut stdin) = child.stdin.take() {
            let bytes = data.to_vec();
            Some(std::thread::spawn(move || {
                let res = stdin.write_all(&bytes);
                drop(stdin);
                res
            }))
        } else {
            None
        }
    } else {
        None
    };

    let mut stdout_handle = child.stdout.take().ok_or("failed to capture stdout pipe")?;
    let mut stderr_handle = child.stderr.take().ok_or("failed to capture stderr pipe")?;

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

    let status_res = TimeoutPolicy::wait_child_until_deadline(
        deadline,
        poll_interval,
        &mut child,
        #[cfg(windows)]
        job_guard.as_ref(),
    );

    if let Some(ref stdin_handle) = stdin_join {
        while !stdin_handle.is_finished() && std::time::Instant::now() < deadline {
            std::thread::sleep(poll_interval);
        }
    }

    if status_res.is_err() {
        kill_process_tree(
            &mut child,
            #[cfg(windows)]
            job_guard.as_ref(),
        );
    }

    while (!stdout_join.is_finished() || !stderr_join.is_finished())
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(poll_interval);
    }

    let is_timed_out = std::time::Instant::now() >= deadline;

    if is_timed_out || status_res.is_err() {
        kill_process_tree(
            &mut child,
            #[cfg(windows)]
            job_guard.as_ref(),
        );
        let drain_deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
        while (!stdout_join.is_finished() || !stderr_join.is_finished())
            && std::time::Instant::now() < drain_deadline
        {
            std::thread::sleep(poll_interval);
        }
        if is_timed_out {
            return Err("timed out waiting for process execution and I/O completion".into());
        }
        status_res?;
        return Err("process exited abnormally".into());
    }

    if let Some(stdin_handle) = stdin_join {
        if stdin_handle.is_finished() {
            match stdin_handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(io_err)) => {
                    kill_process_tree(
                        &mut child,
                        #[cfg(windows)]
                        job_guard.as_ref(),
                    );
                    return Err(format!("failed to write data to process stdin: {io_err}"));
                }
                Err(_) => {
                    kill_process_tree(
                        &mut child,
                        #[cfg(windows)]
                        job_guard.as_ref(),
                    );
                    return Err("process stdin writer thread panicked".into());
                }
            }
        }
    }

    let stdout_bytes = if stdout_join.is_finished() {
        stdout_join
            .join()
            .map_err(|_| "stdout reader thread panicked")?
    } else {
        kill_process_tree(
            &mut child,
            #[cfg(windows)]
            job_guard.as_ref(),
        );
        return Err("timed out reading process stdout".into());
    };

    let stderr_bytes = if stderr_join.is_finished() {
        stderr_join
            .join()
            .map_err(|_| "stderr reader thread panicked")?
    } else {
        kill_process_tree(
            &mut child,
            #[cfg(windows)]
            job_guard.as_ref(),
        );
        return Err("timed out reading process stderr".into());
    };

    let status = status_res?;

    Ok(ProcessExecutionOutput {
        status,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    })
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

    /// Waits for the child process to exit, terminating it and its process tree if it exceeds `deadline`.
    #[allow(dead_code)]
    pub(crate) fn wait_child_until_deadline(
        deadline: std::time::Instant,
        poll_interval: std::time::Duration,
        child: &mut std::process::Child,
        #[cfg(windows)] job_guard: Option<&win32_job::JobObjectGuard>,
    ) -> Result<std::process::ExitStatus, String> {
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        kill_process_tree(
                            child,
                            #[cfg(windows)]
                            job_guard,
                        );
                        return Err(
                            "child process hard-killed: execution time exceeded deadline threshold"
                                .into(),
                        );
                    }
                    std::thread::sleep(poll_interval);
                }
                Err(e) => {
                    kill_process_tree(
                        child,
                        #[cfg(windows)]
                        job_guard,
                    );
                    return Err(format!("error polling child process: {}", e));
                }
            }
        }
    }

    /// Waits for the child process to exit, terminating it and its process tree if it exceeds `self.timeout`.
    #[allow(dead_code)]
    pub(crate) fn wait_child_with_watchdog(
        &self,
        child: &mut std::process::Child,
    ) -> Result<std::process::ExitStatus, String> {
        let deadline = std::time::Instant::now() + self.timeout;
        Self::wait_child_until_deadline(
            deadline,
            self.poll_interval,
            child,
            #[cfg(windows)]
            None,
        )
    }
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
