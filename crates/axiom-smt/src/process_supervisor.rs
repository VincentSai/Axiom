//! Cross-platform OS process supervision, atomic tree destruction, and deadline watchdog.
//!
//! Provides strict fail-closed isolation for external solvers (such as Z3):
//! - Windows: spawns child suspended (`CREATE_SUSPENDED`), assigns to Win32 Job Object
//!   with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, and resumes the main thread, closing
//!   the race condition window where grandchildren could escape.
//! - Unix: creates a new process group (`process_group(0)`) and terminates the entire
//!   tree directly via `libc::kill(-pgid, SIGKILL)` without shelling out.
//! - Enforces an absolute deadline across stdin writing, process exit, and stdout/stderr
//!   reader drain, avoiding hung thread leaks.

use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Unified process timeout and polling watchdog policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutPolicy {
    pub timeout: Duration,
    pub poll_interval: Duration,
}

impl TimeoutPolicy {
    pub const DEFAULT_VERSION_PROBE: Self = Self {
        timeout: Duration::from_millis(3000),
        poll_interval: Duration::from_millis(10),
    };

    pub const DEFAULT_SOLVER_QUERY: Self = Self {
        timeout: Duration::from_millis(10000),
        poll_interval: Duration::from_millis(20),
    };

    pub fn from_millis(ms: u32) -> Self {
        Self {
            timeout: Duration::from_millis(ms as u64),
            poll_interval: Duration::from_millis(20),
        }
    }
}

/// Execution output capturing exit code and collected standard streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessExecutionOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod win32_job {
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;

    pub(crate) type HANDLE = *mut std::ffi::c_void;
    type BOOL = i32;
    type DWORD = u32;
    type LONG = i32;

    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x0000_2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;
    pub const CREATE_SUSPENDED: DWORD = 0x0000_0004;

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

    #[repr(C)]
    pub(crate) struct THREADENTRY32 {
        pub(crate) dw_size: DWORD,
        pub(crate) cnt_usage: DWORD,
        pub(crate) th32_thread_id: DWORD,
        pub(crate) th32_owner_process_id: DWORD,
        pub(crate) tp_base_pri: LONG,
        pub(crate) tp_delta_pri: LONG,
        pub(crate) dw_flags: DWORD,
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
        fn TerminateProcess(h_process: HANDLE, u_exit_code: u32) -> BOOL;
        pub(crate) fn CloseHandle(h_object: HANDLE) -> BOOL;
        pub(crate) fn CreateToolhelp32Snapshot(dw_flags: DWORD, th32_process_id: DWORD) -> HANDLE;
        pub(crate) fn Thread32First(h_snapshot: HANDLE, lpte: *mut THREADENTRY32) -> BOOL;
        pub(crate) fn Thread32Next(h_snapshot: HANDLE, lpte: *mut THREADENTRY32) -> BOOL;
        fn OpenThread(
            dw_desired_access: DWORD,
            b_inherit_handle: BOOL,
            dw_thread_id: DWORD,
        ) -> HANDLE;
        fn ResumeThread(h_thread: HANDLE) -> DWORD;
        fn CancelSynchronousIo(h_thread: HANDLE) -> BOOL;
        fn GetLastError() -> DWORD;
        pub(crate) fn PeekNamedPipe(
            h_named_pipe: HANDLE,
            lp_buffer: *mut std::ffi::c_void,
            n_buffer_size: DWORD,
            lp_bytes_read: *mut DWORD,
            lp_total_bytes_avail: *mut DWORD,
            lp_bytes_left_this_message: *mut DWORD,
        ) -> BOOL;
    }

    /// 非阻塞探测管道中是否有就绪的可用字节数，避免工作线程盲目陷入阻塞的 ReadFile 系统调用
    pub(crate) fn peek_pipe_avail_bytes(handle: HANDLE) -> Result<u32, i32> {
        let mut total_avail: DWORD = 0;
        let res = unsafe {
            PeekNamedPipe(
                handle,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut total_avail,
                std::ptr::null_mut(),
            )
        };
        if res != 0 {
            Ok(total_avail)
        } else {
            let err = unsafe { GetLastError() };
            Err(err as i32)
        }
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

        pub fn terminate_now(&self) -> bool {
            unsafe { TerminateJobObject(self.job_handle, 1) != 0 }
        }
    }

    /// 取消目标工作线程正在执行的阻塞式同步 I/O，打断 ReadFile / WriteFile 系统调用
    pub fn cancel_thread_synchronous_io(thread_handle: HANDLE) -> bool {
        unsafe { CancelSynchronousIo(thread_handle) != 0 }
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

    /// 在挂起启动后恢复进程主线程
    pub fn resume_suspended_process(pid: u32, process_handle: HANDLE) -> Result<(), String> {
        unsafe {
            // 尝试通过 ntdll!NtResumeProcess 原子恢复进程
            let ntdll_name: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
            extern "system" {
                fn GetModuleHandleW(lp_module_name: *const u16) -> HANDLE;
                fn GetProcAddress(
                    h_module: HANDLE,
                    lp_proc_name: *const u8,
                ) -> *mut std::ffi::c_void;
            }
            let h_ntdll = GetModuleHandleW(ntdll_name.as_ptr());
            if !h_ntdll.is_null() {
                let proc_name = b"NtResumeProcess\0";
                let fn_ptr = GetProcAddress(h_ntdll, proc_name.as_ptr());
                if !fn_ptr.is_null() {
                    let nt_resume: extern "system" fn(HANDLE) -> i32 = std::mem::transmute(fn_ptr);
                    let status = nt_resume(process_handle);
                    if status >= 0 {
                        return Ok(());
                    }
                }
            }

            // 回退到 Toolhelp32Snapshot 枚举线程并恢复
            let snapshot = CreateToolhelp32Snapshot(0x0000_0004, 0); // TH32CS_SNAPTHREAD
            if snapshot.is_null() || snapshot == (-1isize as HANDLE) {
                return Err("failed to create toolhelp snapshot for process resumption".into());
            }

            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dw_size = std::mem::size_of::<THREADENTRY32>() as DWORD;

            let mut resumed = 0;
            if Thread32First(snapshot, &mut entry) != 0 {
                loop {
                    if entry.th32_owner_process_id == pid {
                        let h_thread = OpenThread(0x0002, 0, entry.th32_thread_id); // THREAD_SUSPEND_RESUME
                        if !h_thread.is_null() {
                            ResumeThread(h_thread);
                            CloseHandle(h_thread);
                            resumed += 1;
                        }
                    }
                    if Thread32Next(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);

            if resumed == 0 {
                return Err(format!("no threads found to resume for process {pid}"));
            }
            Ok(())
        }
    }

    /// 强杀指定的原始进程句柄（用于 Fail-Closed 场景）
    pub unsafe fn terminate_raw_process(process_handle: HANDLE) {
        let _ = TerminateProcess(process_handle, 1);
    }
}

#[cfg(unix)]
mod unix_supervisor {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    pub fn configure_process_command(cmd: &mut Command) {
        cmd.process_group(0);
    }

    pub fn kill_process_tree(pid: u32) {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        unsafe {
            // SIGKILL = 9 to entire process group
            let _ = kill(-(pid as i32), 9);
            let _ = kill(pid as i32, 9);
        }
    }
}

/// Unified process supervisor ensuring exclusive, idempotent tree destruction
/// and race-free initialization.
pub struct ProcessSupervisor {
    child: Child,
    #[cfg(windows)]
    job_guard: Option<win32_job::JobObjectGuard>,
    terminated: Arc<AtomicBool>,
}

impl ProcessSupervisor {
    /// Spawns a child process with race-free tree-containment guarantees.
    pub fn spawn(mut cmd: Command) -> Result<Self, String> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use std::os::windows::process::CommandExt;

            // 以挂起模式启动进程，彻底消除派生逃逸窗口
            cmd.creation_flags(win32_job::CREATE_SUSPENDED);
            let child = cmd
                .spawn()
                .map_err(|e| format!("failed to spawn suspended process: {e}"))?;

            let job_guard = match win32_job::JobObjectGuard::new() {
                Some(guard) => guard,
                None => {
                    unsafe {
                        win32_job::terminate_raw_process(child.as_raw_handle() as _);
                    }
                    return Err("failed to create Win32 Job Object".into());
                }
            };

            // 将挂起的进程句柄加入 Job Object
            if !job_guard.assign_process(&child) {
                // Fail-Closed: 强杀未受控子进程并返回错误
                unsafe {
                    win32_job::terminate_raw_process(child.as_raw_handle() as _);
                }
                return Err("failed to assign suspended process to Win32 Job Object".into());
            }

            // 成功受控加入 Job Object 后，恢复进程运行
            if let Err(err) =
                win32_job::resume_suspended_process(child.id(), child.as_raw_handle() as _)
            {
                unsafe {
                    win32_job::terminate_raw_process(child.as_raw_handle() as _);
                }
                return Err(format!(
                    "failed to resume process after job assignment: {err}"
                ));
            }

            Ok(Self {
                child,
                job_guard: Some(job_guard),
                terminated: Arc::new(AtomicBool::new(false)),
            })
        }

        #[cfg(unix)]
        {
            unix_supervisor::configure_process_command(&mut cmd);
            let child = cmd
                .spawn()
                .map_err(|e| format!("failed to spawn process group: {e}"))?;
            Ok(Self {
                child,
                terminated: Arc::new(AtomicBool::new(false)),
            })
        }

        #[cfg(not(any(windows, unix)))]
        {
            let child = cmd
                .spawn()
                .map_err(|e| format!("failed to spawn process: {e}"))?;
            Ok(Self {
                child,
                terminated: Arc::new(AtomicBool::new(false)),
            })
        }
    }

    /// Access mutable reference to underlying Child.
    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Idempotently and forcefully terminates the process tree and releases resources.
    pub fn terminate(&mut self) -> bool {
        if self.terminated.swap(true, Ordering::SeqCst) {
            return true;
        }

        let mut success = true;

        #[cfg(windows)]
        {
            if let Some(ref guard) = self.job_guard {
                if !guard.terminate_now() {
                    success = false;
                }
            }
        }

        #[cfg(unix)]
        {
            unix_supervisor::kill_process_tree(self.child.id());
        }

        let _ = self.child.kill();

        // 简短的有界等待子进程状态回收 (最多 100ms)
        let drain_deadline = Instant::now() + Duration::from_millis(100);
        while Instant::now() < drain_deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        success
    }
}

impl Drop for ProcessSupervisor {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

enum StreamDrainResult<T> {
    Completed(Result<T, String>),
    TimedOut,
}

struct StreamReaderComm {
    rx: std::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>,
    handle: Option<std::thread::JoinHandle<()>>,
    cancelled: Arc<AtomicBool>,
}

impl StreamReaderComm {
    /// 尝试在指定超时内回收 reader 线程结果。
    /// 若超时，绝不在 terminate 前无界 join，而是返回 TimedOut 保留 handle 供进程树强杀后有界回收。
    fn try_reap(&mut self, timeout: Duration) -> StreamDrainResult<Vec<u8>> {
        match self.rx.recv_timeout(timeout) {
            Ok(Ok(bytes)) => {
                if let Some(h) = self.handle.take() {
                    let _ = h.join();
                }
                StreamDrainResult::Completed(Ok(bytes))
            }
            Ok(Err(e)) => {
                if let Some(h) = self.handle.take() {
                    let _ = h.join();
                }
                StreamDrainResult::Completed(Err(format!("I/O stream read error: {e}")))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                self.cancelled.store(true, Ordering::SeqCst);
                StreamDrainResult::TimedOut
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(h) = self.handle.take() {
                    let _ = h.join();
                }
                StreamDrainResult::Completed(Err("stream channel disconnected unexpectedly".into()))
            }
        }
    }

    /// 在进程树强杀（supervisor.terminate()）后执行有界回收，严禁无界 join 挂死主线程。
    /// 仅在通过 channel 明确确认工作线程退出（Ok 或 Disconnected）时才执行 h.join()。
    /// 支持零宽限立即非阻塞探测；若超时且处于 Windows 平台，尝试打断阻塞 I/O，绝不累加无谓延迟。
    fn bounded_join_after_kill(mut self, grace: Duration) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let initial_exited = if grace.is_zero() {
                matches!(
                    self.rx.try_recv(),
                    Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected)
                )
            } else {
                matches!(
                    self.rx.recv_timeout(grace),
                    Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
                )
            };

            if initial_exited {
                let _ = h.join();
                return;
            }

            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                let thread_raw = h.as_raw_handle() as win32_job::HANDLE;
                win32_job::cancel_thread_synchronous_io(thread_raw);
                if !grace.is_zero() {
                    let short_wait = grace.min(Duration::from_millis(10));
                    if let Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) =
                        self.rx.recv_timeout(short_wait)
                    {
                        let _ = h.join();
                        return;
                    }
                } else if let Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) =
                    self.rx.try_recv()
                {
                    let _ = h.join();
                    return;
                }
            }

            eprintln!("WARN: process stream reader thread did not terminate within bounded grace; abandoning join to prevent hang");
        }
    }

    #[cfg(test)]
    fn reap_with_timeout(mut self, timeout: Duration) -> Result<Vec<u8>, String> {
        match self.try_reap(timeout) {
            StreamDrainResult::Completed(res) => res,
            StreamDrainResult::TimedOut => {
                self.bounded_join_after_kill(Duration::from_millis(100));
                Err("timed out draining process stream".into())
            }
        }
    }
}

/// Helper to read a stream into a channel with bounded timeout support, non-blocking pipe cancellation, and strict JoinHandle tracking.
fn spawn_stream_reader<R: Read + Send + 'static>(
    mut reader: R,
    #[allow(unused_variables)] raw_handle: Option<isize>,
) -> StreamReaderComm {
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_clone = cancelled.clone();
    let (tx, rx) = mpsc::sync_channel(1);

    let handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut result = Ok(());

        loop {
            if cancelled_clone.load(Ordering::Relaxed) {
                result = Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "stream read cancelled by deadline supervisor",
                ));
                break;
            }

            #[cfg(windows)]
            if let Some(h_val) = raw_handle {
                let h = h_val as win32_job::HANDLE;
                match win32_job::peek_pipe_avail_bytes(h) {
                    Ok(0) => {
                        // 暂无可用字节：休眠 5ms 让出 CPU，下一次循环第一时间检测 cancelled 标志
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Ok(_) => {
                        // 管道中已有字节就绪，后续 reader.read 绝不阻塞挂起
                    }
                    Err(109) => {
                        // 109 = ERROR_BROKEN_PIPE：写端已全部退出关闭且无残留数据
                        break;
                    }
                    Err(_) => {
                        // 管道已被关闭或异常，直接退出
                        break;
                    }
                }
            }

            match reader.read(&mut chunk) {
                Ok(0) => break, // EOF
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        let final_res = match result {
            Ok(()) => Ok(buf),
            Err(e) => Err(e),
        };
        let _ = tx.send(final_res);
    });

    StreamReaderComm {
        rx,
        handle: Some(handle),
        cancelled,
    }
}

struct StdinWriterComm {
    rx: std::sync::mpsc::Receiver<Result<(), std::io::Error>>,
    handle: Option<std::thread::JoinHandle<()>>,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl StdinWriterComm {
    /// 尝试在指定超时内回收 writer 线程结果。
    /// 若超时，绝不在 terminate 前无界 join，而是返回 TimedOut 保留 handle 供进程树强杀后有界回收。
    fn try_reap(&mut self, timeout: Duration) -> StreamDrainResult<()> {
        match self.rx.recv_timeout(timeout) {
            Ok(Ok(())) => {
                if let Some(h) = self.handle.take() {
                    let _ = h.join();
                }
                StreamDrainResult::Completed(Ok(()))
            }
            Ok(Err(io_err)) => {
                if let Some(h) = self.handle.take() {
                    let _ = h.join();
                }
                StreamDrainResult::Completed(Err(format!(
                    "failed to write data to process stdin: {io_err}"
                )))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                self.cancelled.store(true, Ordering::SeqCst);
                StreamDrainResult::TimedOut
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(h) = self.handle.take() {
                    let _ = h.join();
                }
                StreamDrainResult::Completed(Err(
                    "stdin writer channel disconnected unexpectedly".into()
                ))
            }
        }
    }

    /// 在进程树强杀（supervisor.terminate()）后执行有界回收，严禁无界 join 挂死主线程。
    /// 仅在通过 channel 明确确认工作线程退出（Ok 或 Disconnected）时才执行 h.join()。
    /// 支持零宽限立即非阻塞探测；若超时且处于 Windows 平台，尝试打断阻塞 I/O，绝不累加无谓延迟。
    fn bounded_join_after_kill(mut self, grace: Duration) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let initial_exited = if grace.is_zero() {
                matches!(
                    self.rx.try_recv(),
                    Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected)
                )
            } else {
                matches!(
                    self.rx.recv_timeout(grace),
                    Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
                )
            };

            if initial_exited {
                let _ = h.join();
                return;
            }

            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                let thread_raw = h.as_raw_handle() as win32_job::HANDLE;
                win32_job::cancel_thread_synchronous_io(thread_raw);
                if !grace.is_zero() {
                    let short_wait = grace.min(Duration::from_millis(10));
                    if let Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) =
                        self.rx.recv_timeout(short_wait)
                    {
                        let _ = h.join();
                        return;
                    }
                } else if let Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) =
                    self.rx.try_recv()
                {
                    let _ = h.join();
                    return;
                }
            }

            eprintln!("WARN: process stdin writer thread did not terminate within bounded grace; abandoning join to prevent hang");
        }
    }

    #[cfg(test)]
    fn reap_with_timeout(mut self, timeout: Duration) -> Result<(), String> {
        match self.try_reap(timeout) {
            StreamDrainResult::Completed(res) => res,
            StreamDrainResult::TimedOut => {
                self.bounded_join_after_kill(Duration::from_millis(100));
                Err("timed out writing data to process stdin".into())
            }
        }
    }
}

fn spawn_stdin_writer(mut stdin: std::process::ChildStdin, bytes: Vec<u8>) -> StdinWriterComm {
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cancelled_clone = cancelled.clone();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);

    let handle = std::thread::spawn(move || {
        let chunk_size = 64 * 1024;
        let mut offset = 0;
        let mut result = Ok(());

        while offset < bytes.len() {
            if cancelled_clone.load(std::sync::atomic::Ordering::Relaxed) {
                result = Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "stdin write cancelled by deadline supervisor",
                ));
                break;
            }
            let end = (offset + chunk_size).min(bytes.len());
            match stdin.write_all(&bytes[offset..end]) {
                Ok(()) => offset = end,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        drop(stdin);
        let _ = tx.send(result);
    });

    StdinWriterComm {
        rx,
        handle: Some(handle),
        cancelled,
    }
}

/// 以绝对单调时钟截止时间监管执行子进程，并进行多通道有界 I/O 回收与看门狗强杀。
pub(crate) fn execute_process_until_deadline(
    mut cmd: Command,
    stdin_data: Option<&[u8]>,
    deadline: Instant,
    poll_interval: Duration,
) -> Result<ProcessExecutionOutput, String> {
    if stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut supervisor = ProcessSupervisor::spawn(cmd)?;

    let mut stdin_comm = if let Some(data) = stdin_data {
        if let Some(stdin) = supervisor.child_mut().stdin.take() {
            let bytes = data.to_vec();
            Some(spawn_stdin_writer(stdin, bytes))
        } else {
            None
        }
    } else {
        None
    };

    let stdout_pipe = supervisor
        .child_mut()
        .stdout
        .take()
        .ok_or("failed to capture stdout pipe")?;
    let stderr_pipe = supervisor
        .child_mut()
        .stderr
        .take()
        .ok_or("failed to capture stderr pipe")?;

    #[cfg(windows)]
    let stdout_raw = {
        use std::os::windows::io::AsRawHandle;
        Some(stdout_pipe.as_raw_handle() as isize)
    };
    #[cfg(not(windows))]
    let stdout_raw = None;

    #[cfg(windows)]
    let stderr_raw = {
        use std::os::windows::io::AsRawHandle;
        Some(stderr_pipe.as_raw_handle() as isize)
    };
    #[cfg(not(windows))]
    let stderr_raw = None;

    let mut stdout_comm = spawn_stream_reader(stdout_pipe, stdout_raw);
    let mut stderr_comm = spawn_stream_reader(stderr_pipe, stderr_raw);

    // 循环轮询子进程退出状态直至截止时间
    let mut exit_status: Option<ExitStatus> = None;
    while Instant::now() < deadline {
        match supervisor.child_mut().try_wait() {
            Ok(Some(st)) => {
                exit_status = Some(st);
                break;
            }
            Ok(None) => std::thread::sleep(poll_interval),
            Err(e) => {
                // 等待出错：第一动作立即强杀整个进程树，随后零宽限立即非阻塞回收句柄
                supervisor.terminate();
                let grace = Duration::ZERO;
                if let Some(comm) = stdin_comm {
                    comm.bounded_join_after_kill(grace);
                }
                stdout_comm.bounded_join_after_kill(grace);
                stderr_comm.bounded_join_after_kill(grace);
                return Err(format!("error waiting on child process: {e}"));
            }
        }
    }

    // 检查是否超时
    if exit_status.is_none() {
        // 超时：第一动作立即强杀整个进程树，随后零宽限立即非阻塞回收句柄，彻底消除延迟累加
        supervisor.terminate();
        let grace = Duration::ZERO;
        if let Some(comm) = stdin_comm {
            comm.bounded_join_after_kill(grace);
        }
        stdout_comm.bounded_join_after_kill(grace);
        stderr_comm.bounded_join_after_kill(grace);
        return Err("timed out waiting for process execution and I/O completion".into());
    }

    let status = exit_status.unwrap();

    // 子进程已退出，进入 I/O 回收期（所有流共享同一个单调绝对 deadline）
    let get_io_timeout = |dl: Instant| {
        dl.saturating_duration_since(Instant::now())
            .min(Duration::from_millis(150))
    };
    let calc_grace = |dl: Instant| {
        dl.saturating_duration_since(Instant::now())
            .min(Duration::from_millis(100))
    };

    // 1. 检查并回收 stdin 写入结果
    if let Some(mut comm) = stdin_comm.take() {
        let stdin_timeout = get_io_timeout(deadline);
        match comm.try_reap(stdin_timeout) {
            StreamDrainResult::Completed(Ok(())) => {}
            StreamDrainResult::Completed(Err(err_msg)) => {
                // 若子进程已经以非 0 状态码退出，管道断开通常是子进程提前退出/崩溃关闭读端所致；
                // 此时优先透传真实的退出码和 stderr，避免被底层的管道写入错误掩盖
                if status.success() {
                    supervisor.terminate();
                    let grace = calc_grace(deadline);
                    stdout_comm.bounded_join_after_kill(grace);
                    stderr_comm.bounded_join_after_kill(grace);
                    return Err(err_msg);
                }
            }
            StreamDrainResult::TimedOut => {
                supervisor.terminate();
                let grace = calc_grace(deadline);
                comm.bounded_join_after_kill(grace);
                stdout_comm.bounded_join_after_kill(grace);
                stderr_comm.bounded_join_after_kill(grace);
                return Err(
                    "timed out writing data to process stdin (child or orphan holding pipe)".into(),
                );
            }
        }
    }

    // 2. 严格以 Fail-Closed 方式回收 stdout
    let stdout_timeout = get_io_timeout(deadline);
    let stdout = match stdout_comm.try_reap(stdout_timeout) {
        StreamDrainResult::Completed(Ok(buf)) => buf,
        StreamDrainResult::Completed(Err(e)) => {
            supervisor.terminate();
            let grace = calc_grace(deadline);
            stderr_comm.bounded_join_after_kill(grace);
            return Err(format!("failed to drain process stdout: {e}"));
        }
        StreamDrainResult::TimedOut => {
            supervisor.terminate();
            let grace = calc_grace(deadline);
            stdout_comm.bounded_join_after_kill(grace);
            stderr_comm.bounded_join_after_kill(grace);
            return Err("timed out draining process stdout (orphan holding pipe)".into());
        }
    };

    // 3. 严格以 Fail-Closed 方式回收 stderr
    let stderr_timeout = get_io_timeout(deadline);
    let stderr = match stderr_comm.try_reap(stderr_timeout) {
        StreamDrainResult::Completed(Ok(buf)) => buf,
        StreamDrainResult::Completed(Err(e)) => {
            supervisor.terminate();
            return Err(format!("failed to drain process stderr: {e}"));
        }
        StreamDrainResult::TimedOut => {
            supervisor.terminate();
            let grace = calc_grace(deadline);
            stderr_comm.bounded_join_after_kill(grace);
            return Err("timed out draining process stderr (orphan holding pipe)".into());
        }
    };

    Ok(ProcessExecutionOutput {
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_supervisor_suspended_spawn_and_run() {
        #[cfg(windows)]
        let mut cmd = Command::new("cmd");
        #[cfg(windows)]
        cmd.args(["/C", "echo", "axiom_supervisor_ok"]);

        #[cfg(not(windows))]
        let mut cmd = Command::new("echo");
        #[cfg(not(windows))]
        cmd.arg("axiom_supervisor_ok");

        let deadline = Instant::now() + Duration::from_secs(5);
        let out = execute_process_until_deadline(cmd, None, deadline, Duration::from_millis(10))
            .expect("process execution should succeed");

        assert!(out.status.success());
        let stdout_str = String::from_utf8_lossy(&out.stdout);
        assert!(stdout_str.contains("axiom_supervisor_ok"));
    }

    #[test]
    fn test_process_supervisor_kills_hung_tree_on_timeout() {
        #[cfg(windows)]
        let mut cmd = Command::new("powershell");
        #[cfg(windows)]
        cmd.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"]);

        #[cfg(not(windows))]
        let mut cmd = Command::new("sleep");
        #[cfg(not(windows))]
        cmd.arg("60");

        let start = Instant::now();
        let deadline = start + Duration::from_millis(150);
        let res = execute_process_until_deadline(cmd, None, deadline, Duration::from_millis(10));

        let elapsed = start.elapsed();
        assert!(res.is_err());
        assert!(
            elapsed < Duration::from_millis(1500),
            "watchdog should enforce bounded termination, took {elapsed:?}"
        );
    }

    #[test]
    fn test_process_supervisor_kills_orphan_holding_pipe() {
        // 模拟父进程退出而后代进程长持 stdout 管道的跨平台孤儿测试
        #[cfg(windows)]
        let mut cmd = Command::new("cmd");
        #[cfg(windows)]
        cmd.args([
            "/C",
            "start /b powershell -NoProfile -Command \"Start-Sleep -Seconds 60\" & echo parent_done",
        ]);

        #[cfg(not(windows))]
        let mut cmd = Command::new("sh");
        #[cfg(not(windows))]
        cmd.args(["-c", "(sleep 60 &) ; echo parent_done"]);

        let start = Instant::now();
        let deadline = start + Duration::from_millis(300);
        let _ = execute_process_until_deadline(cmd, None, deadline, Duration::from_millis(10));

        let elapsed = start.elapsed();
        // 验证执行器不会因为孤儿进程长持管道句柄而挂起超过有界时间
        assert!(
            elapsed < Duration::from_millis(2000),
            "orphan holding pipe should not hang reader thread, took {elapsed:?}"
        );
    }

    #[test]
    fn test_process_supervisor_kills_orphan_holding_stdin_writer_without_hanging() {
        // 模拟父进程退出而后代进程长持 stdin 管道，且传入大于管道缓冲区的海量数据
        #[cfg(windows)]
        let mut cmd = Command::new("cmd");
        #[cfg(windows)]
        cmd.args([
            "/C",
            "start /b powershell -NoProfile -Command \"Start-Sleep -Seconds 60\" & exit 0",
        ]);

        #[cfg(not(windows))]
        let mut cmd = Command::new("sh");
        #[cfg(not(windows))]
        cmd.args(["-c", "(sleep 60 &) ; exit 0"]);

        // 生成 2MB 超过通常 4KB~64KB 操作系统管道缓冲区的数据
        let large_payload = vec![b'x'; 2 * 1024 * 1024];

        let start = Instant::now();
        let deadline = start + Duration::from_millis(300);
        let res = execute_process_until_deadline(
            cmd,
            Some(&large_payload),
            deadline,
            Duration::from_millis(10),
        );

        let elapsed = start.elapsed();
        // 必须在严格有界时间内返回（deadline 300ms + 最多 300ms 宽限），绝对不能由于无界 join 延宕至数秒
        assert!(
            elapsed < Duration::from_millis(600),
            "orphan holding stdin should not hang writer thread beyond bounded deadline, expected < 600ms, took {elapsed:?}"
        );
        let _ = res;
    }

    #[test]
    fn test_stdin_writer_comm_cancelled_exits_cleanly() {
        // 验证 StdinWriterComm 在超时取消时能够协同唤醒、安全回收 JoinHandle，绝不发生永久阻塞或线程泄漏
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();

        let handle = std::thread::spawn(move || {
            while !cancelled_clone.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(5));
            }
            let _ = tx.send(Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "stdin write cancelled by deadline supervisor",
            )));
        });

        let comm = StdinWriterComm {
            rx,
            handle: Some(handle),
            cancelled,
        };

        let start = Instant::now();
        let res = comm.reap_with_timeout(Duration::from_millis(50));
        let elapsed = start.elapsed();

        assert!(res.is_err(), "应返回取消/超时错误");
        assert!(
            elapsed < Duration::from_millis(200),
            "reap_with_timeout 必须在短暂期限内完整 join 回收线程并返回，实际耗时 {elapsed:?}"
        );
    }

    #[test]
    fn test_reader_stream_drain_timeout_fails_closed() {
        // 验证当流读取中断或取消时，StreamReaderComm 能够 Fail-Closed 报错且安全回收 JoinHandle
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();

        let handle = std::thread::spawn(move || {
            while !cancelled_clone.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(5));
            }
            let _ = tx.send(Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "read timed out",
            )));
        });

        let comm = StreamReaderComm {
            rx,
            handle: Some(handle),
            cancelled,
        };

        let res = comm.reap_with_timeout(Duration::from_millis(50));
        assert!(
            res.is_err(),
            "未完成的流读取必须 Fail-Closed 返回错误，严禁静默返回空字节"
        );
    }

    #[test]
    fn test_repeated_timeouts_do_not_leak_threads() {
        // [Round 20 P1 Spec 4]: 连续多次触发恶意阻塞管道超时，验证工作线程完全被 join 回收，无累计线程泄漏
        #[cfg(windows)]
        fn get_current_threads() -> u32 {
            unsafe {
                let pid = std::process::id();
                let snapshot = win32_job::CreateToolhelp32Snapshot(0x0000_0004, 0);
                if snapshot.is_null() || snapshot == (-1isize as win32_job::HANDLE) {
                    return 0;
                }
                let mut entry: win32_job::THREADENTRY32 = std::mem::zeroed();
                entry.dw_size = std::mem::size_of::<win32_job::THREADENTRY32>() as u32;
                let mut count = 0;
                if win32_job::Thread32First(snapshot, &mut entry) != 0 {
                    loop {
                        if entry.th32_owner_process_id == pid {
                            count += 1;
                        }
                        if win32_job::Thread32Next(snapshot, &mut entry) == 0 {
                            break;
                        }
                    }
                }
                win32_job::CloseHandle(snapshot);
                count
            }
        }

        #[cfg(windows)]
        let initial_threads = get_current_threads();

        for _ in 0..5 {
            #[cfg(windows)]
            let mut cmd = Command::new("cmd");
            #[cfg(windows)]
            cmd.args([
                "/C",
                "start /b powershell -NoProfile -Command \"Start-Sleep -Seconds 60\" & echo child_running",
            ]);

            #[cfg(not(windows))]
            let mut cmd = Command::new("sh");
            #[cfg(not(windows))]
            cmd.args(["-c", "(sleep 60 &) ; echo child_running"]);

            let start = Instant::now();
            let deadline = start + Duration::from_millis(150);
            let _ = execute_process_until_deadline(cmd, None, deadline, Duration::from_millis(10));
            // 短暂休眠让 OS 线程清理完全收敛
            std::thread::sleep(Duration::from_millis(30));
        }

        #[cfg(windows)]
        {
            let final_threads = get_current_threads();
            // 允许轻微波动（最多 2 个由系统进程池动态生成的线程），绝不能泄漏 5~10 个 reader/writer 线程
            let thread_delta = final_threads.saturating_sub(initial_threads);
            assert!(
                thread_delta <= 2,
                "连续 5 次管道超时导致线程累计泄漏！初始线程: {initial_threads}, 最终线程: {final_threads}, 增量: {thread_delta}"
            );
        }
    }
}
