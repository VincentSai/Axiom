// 受控假进程：忽略任何命令行参数，自身进入长睡眠并启动子进程，用于看门狗超时测试
use std::time::Duration;

fn main() {
    // 启动一个子进程，形成真实的进程树
    #[cfg(windows)]
    let child = std::process::Command::new("cmd")
        .args([
            "/C",
            "powershell -NoProfile -Command \"Start-Sleep -Seconds 60\"",
        ])
        .spawn()
        .ok();

    #[cfg(not(windows))]
    let child = std::process::Command::new("sleep").arg("60").spawn().ok();

    // 自身长睡眠 60 秒
    std::thread::sleep(Duration::from_secs(60));

    if let Some(mut c) = child {
        let _ = c.kill();
    }
}
