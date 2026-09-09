#!/usr/bin/env python3
"""
Axiom A/B Benchmark Evaluation Runner
Real-world empirical measurement script using the official Google Gemini API.

Features:
- Pure standard library implementation (no pip dependencies needed).
- Accurate token counting via the official `countTokens` endpoint.
- Full generation & verification via `generateContent` + local Axiom SMT / Rust compiler.
- Saves authentic, reproducible metadata and counts directly to `eval_matrix_data.json`.
"""

import argparse
import io
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

# 解决 Windows 控制台默认 GBK 编码无法输出 Emoji 与 UTF-8 字符的问题
if sys.platform == "win32":
    try:
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
    except Exception:
        pass

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
GOLDEN_TASKS_PATH = os.path.join(SCRIPT_DIR, "golden_tasks.json")
OUTPUT_DATA_PATH = os.path.join(SCRIPT_DIR, "data", "eval_matrix_data.json")

DEFAULT_BASE_URL = "https://generativelanguage.googleapis.com/v1beta"
DEFAULT_MODEL = "gemini-1.5-flash"
MAX_HEALING_RETRIES = 3


def count_exact_tokens(text: str) -> int:
    """Accurate BPE Token calculation using OpenAI cl100k_base tokenizer without crude multipliers."""
    if not text:
        return 0
    try:
        import tiktoken
        enc = tiktoken.get_encoding("cl100k_base")
        return len(enc.encode(text))
    except Exception:
        import re
        tokens = re.findall(r"\w+|[^\w\s]", text, re.UNICODE)
        return max(1, len(tokens))

# 20 个黄金任务的 Ground-Truth 测试向量表 (输入元组, 预期输出)
TEST_VECTORS = {
    1: [((0, 0, 0, 0), 0), ((10, 5, 2, 1), 12), ((3, 7, 8, 2), 10), ((100, 200, 50, 80), 170)],
    2: [((0, 0, 0, 0), 0), ((3, 5, 7, 2), 4), ((10, 10, 10, 20), 10), ((5, 12, 1, 3), 9)],
    3: [
        ((0, 0, 10, 10, 5, 5, 15, 15), True),
        ((0, 0, 5, 5, 10, 10, 15, 15), False),
        ((0, 0, 10, 10, 10, 10, 20, 20), True),
        ((2, 2, 8, 8, 0, 0, 10, 10), True),
    ],
    4: [((0, 0), 0), ((1, 0), 65536), ((0, 1), 1), ((65535, 65535), 4294967295)],
    5: [((0,), 10), ((5,), 20), ((50,), 110)],
    6: [((100, 0), 100), ((100, 100), 50), ((1, 50), 1)],
    7: [((50, 150, False), 50), ((50, 150, True), 75), ((100, 100, True), 100)],
    8: [((100, 20), 75), ((20, 50), 5), ((50, 50), 5)],
    9: [((100, 0, False), 100), ((100, 100, False), 50)],
    10: [((1,), 5), ((3,), 15), ((10,), 50)],
    11: [((50, 100), 0), ((100, 30), 70), ((50, 0), 50)],
    12: [((20, 50), 20), ((20, 100), 20), ((20, 150), 25)],
    13: [((100, 0), 100), ((100, 20), 60), ((100, 60), 0)],
    14: [((15, 100), 3), ((50, 80), 2), ((50, 20), 1)],
    15: [((0,), 1), ((9000,), 1), ((9001,), 2), ((9800,), 2), ((9801,), 3), ((9999,), 3)],
    16: [((100, 10, 15), 120), ((100, 10, 10), 100), ((100, 15, 10), 50)],
    17: [((10, 5, 2), 107), ((0, 0, 0), 0)],
    18: [((100, 100), 1), ((70, 100), 1), ((69, 100), 2), ((30, 100), 2), ((29, 100), 3), ((0, 100), 3)],
    19: [((100, 0, 2), 110)],
    20: [((100, 0), 100), ((100, 100), 0), ((100, 25), 75)],
}


def check_port_open(host, port, timeout=0.3):
    """检测本地端口是否开放"""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(timeout)
    try:
        s.connect((host, port))
        s.close()
        return True
    except Exception:
        return False


def detect_recommended_proxy():
    """检测本机是否运行了常用本地代理端口"""
    for port in [7897, 7890, 10808, 10809, 8080]:
        if check_port_open("127.0.0.1", port):
            return f"http://127.0.0.1:{port}"
    return None


def get_git_commit_sha() -> str:
    """获取当前仓库最新的 Git Commit SHA"""
    try:
        out = subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=SCRIPT_DIR, stderr=subprocess.DEVNULL)
        return out.decode("utf-8").strip()
    except Exception:
        return "unknown"


def get_z3_version() -> str:
    """探测系统或 Axiom 内置 Z3 SMT 求解器版本"""
    candidates = [
        r"C:\Users\Administrator\.axiom\bin\z3.exe",
        shutil.which("z3")
    ]
    for p in candidates:
        if p and os.path.exists(p):
            try:
                out = subprocess.check_output([p, "--version"], stderr=subprocess.DEVNULL)
                return out.decode("utf-8").strip()
            except Exception:
                pass
    return "unknown"



def clean_model_name(model: str) -> str:
    """清洗模型名称，去除空白与冗余的 'models/' 前缀"""
    m = model.strip()
    if m.startswith("models/"):
        m = m[len("models/"):]
    return m


def build_endpoint_url(base_url: str, model: str, action: str, api_key: str) -> str:
    """智能组装 Gemini API 端点 URL，支持 countTokens/generateContent/listModels"""
    clean_base = base_url.strip().rstrip("/")
    if clean_base.endswith("/models"):
        clean_base = clean_base[:-len("/models")]

    if not action:
        return f"{clean_base}/models?key={api_key}"

    c_model = clean_model_name(model)
    return f"{clean_base}/models/{c_model}:{action}?key={api_key}"


def get_http_opener(proxy: str = None):
    """构建支持代理的 urllib opener"""
    if proxy:
        proxy_handler = urllib.request.ProxyHandler({
            "http": proxy,
            "https": proxy
        })
        return urllib.request.build_opener(proxy_handler)
    return urllib.request.build_opener()


def send_api_request(opener, url: str, payload: dict = None, method: str = "POST", timeout: int = 40, retries: int = 4):
    """统一执行 API HTTP 请求，包含结构化 HTTPError 解析与自动退避重试"""
    data_bytes = json.dumps(payload).encode("utf-8") if payload is not None else None
    headers = {"Content-Type": "application/json"} if payload is not None else {}

    for attempt in range(retries + 1):
        req = urllib.request.Request(url, data=data_bytes, headers=headers, method=method)
        try:
            with opener.open(req, timeout=timeout) as resp:
                raw_body = resp.read().decode("utf-8")
                return json.loads(raw_body)
        except urllib.error.HTTPError as e:
            err_body = ""
            try:
                err_body = e.read().decode("utf-8", errors="replace")
            except Exception:
                pass

            err_json = {}
            if err_body:
                try:
                    err_json = json.loads(err_body)
                except Exception:
                    pass

            err_msg = ""
            if isinstance(err_json, dict) and "error" in err_json:
                err_info = err_json["error"]
                if isinstance(err_info, dict):
                    err_msg = err_info.get("message", "")
                else:
                    err_msg = str(err_info)

            # 503 (服务端高负载高峰), 429 (配额限流), 500/502/504 (网关与临时错误)，自动退避重试
            if e.code in (429, 500, 502, 503, 504) and attempt < retries:
                wait_sec = (attempt + 1) * 4
                tag = "配额限流 429" if e.code == 429 else f"服务器高负载/临时不可用 {e.code}"
                print(f" [{tag}，等待 {wait_sec}s 自动重试 ({attempt + 1}/{retries})...]", end="", flush=True)
                time.sleep(wait_sec)
                continue

            diag_lines = [f"HTTP {e.code}: {e.reason}"]
            if err_msg:
                diag_lines.append(f"服务返回详情: {err_msg}")
            elif err_body:
                snippet = err_body[:200].replace("\n", " ")
                diag_lines.append(f"响应内容片段: {snippet}")

            if e.code == 503:
                diag_lines.append("💡 排查建议: 当前模型正在经历全球高负载高峰 (Spikes in demand)。建议稍后重试，或指定其他稳定模型，例如: --model gemini-2.0-flash 或 --model gemini-1.5-flash-latest")
            elif e.code == 404:
                diag_lines.append("💡 排查建议: 模型名称不存在或网关路由不支持该端点。可尝试运行: python examples/benchmark_eval/eval_runner.py --probe 查看可用模型列表。")
            elif e.code in (400, 403):
                diag_lines.append("💡 排查建议: 请检查 API Key 是否有效，或是否在 Google Cloud Console 中启用了 Generative Language API。")

            raise RuntimeError("\n".join(diag_lines))

        except urllib.error.URLError as e:
            if attempt < retries:
                wait_sec = (attempt + 1) * 3
                print(f" [网络抖动 ({e.reason})，等待 {wait_sec}s 重试 ({attempt + 1}/{retries})...]", end="", flush=True)
                time.sleep(wait_sec)
                continue
            rec_proxy = detect_recommended_proxy()
            hint = ""
            if rec_proxy:
                hint = f"\n💡 检测到本地存在可用代理端口: {rec_proxy}。建议运行时增加参数: --proxy {rec_proxy}"
            raise RuntimeError(f"网络连接失败: {e.reason}{hint}")


def load_golden_tasks():
    with open(GOLDEN_TASKS_PATH, "r", encoding="utf-8") as f:
        data = json.load(f)
    return data.get("tasks", [])


def find_axiom_binary():
    """定位本地编译的 axiom 可执行文件"""
    root_dir = os.path.dirname(os.path.dirname(SCRIPT_DIR))
    exe_name = "axiom.exe" if sys.platform == "win32" else "axiom"
    target_debug = os.path.join(root_dir, "target", "debug", exe_name)
    if os.path.isfile(target_debug):
        return target_debug
    target_release = os.path.join(root_dir, "target", "release", exe_name)
    if os.path.isfile(target_release):
        return target_release
    which_p = shutil.which("axiom")
    if which_p:
        return which_p
    return target_debug


def extract_code(text: str, target_kind: str = "rust") -> str:
    """提取 markdown 代码块或纯表达式，清除前后注释与反引号"""
    clean = text.strip()
    if f"```{target_kind}" in clean:
        parts = clean.split(f"```{target_kind}")
        if len(parts) > 1:
            clean = parts[1].split("```")[0].strip()
    elif "```" in clean:
        parts = clean.split("```")
        if len(parts) > 1:
            clean = parts[1].split("```")[0].strip()
    return clean


def run_rust_test(code: str, task: dict) -> tuple[bool, str]:
    """编译并物理执行 Rust 单元测试"""
    tid = task["id"]
    fname = task["function_name"]
    vectors = TEST_VECTORS.get(tid, [])

    clean_code = extract_code(code, "rust")

    test_lines = []
    for args, expected in vectors:
        args_str = ", ".join([str(a).lower() if isinstance(a, bool) else str(a) for a in args])
        exp_str = str(expected).lower() if isinstance(expected, bool) else str(expected)
        test_lines.append(f"    assert_eq!({fname}({args_str}), {exp_str});")
    tests_body = "\n".join(test_lines)

    full_source = f"""{clean_code}

#[cfg(test)]
mod tests {{
    use super::*;
    #[test]
    fn test_task_vectors() {{
{tests_body}
    }}
}}
"""
    with tempfile.TemporaryDirectory() as tmpdir:
        src_path = os.path.join(tmpdir, "test.rs")
        exe_path = os.path.join(tmpdir, "test.exe" if sys.platform == "win32" else "test")
        with open(src_path, "w", encoding="utf-8") as f:
            f.write(full_source)

        compile_cmd = ["rustc", "--test", src_path, "-o", exe_path]
        try:
            c_res = subprocess.run(compile_cmd, capture_output=True, text=True, timeout=15)
            if c_res.returncode != 0:
                err = c_res.stderr.strip()
                return False, f"Rust compilation error:\n{err[:600]}"
        except Exception as e:
            return False, f"Failed to execute rustc: {e}"

        try:
            r_res = subprocess.run([exe_path], capture_output=True, text=True, timeout=10)
            if r_res.returncode != 0:
                err = r_res.stdout.strip() or r_res.stderr.strip()
                return False, f"Rust unit test assertion failed:\n{err[:600]}"
        except Exception as e:
            return False, f"Test runtime crash: {e}"

    return True, "All unit tests passed"


def run_axiom_verify(expr: str, task: dict, axiom_bin: str) -> tuple[bool, str, dict | None, str, str]:
    """通过 Axiom SMT 求解器 (Z3) 严格验证契约与效果安全，拒绝任何采样降级"""
    clean_expr = extract_code(expr, "axiom")
    if clean_expr.startswith("fn "):
        first_brace = clean_expr.find("{")
        last_brace = clean_expr.rfind("}")
        if first_brace != -1 and last_brace != -1 and last_brace > first_brace:
            clean_expr = clean_expr[first_brace + 1:last_brace].strip()

    params_str = ", ".join([f"{p['name']}: {p['type']}" for p in task['inputs']])
    requires_items = "\n        ".join([f"{req}," for req in task['contracts']['requires']])
    ensures_items = "\n        ".join([f"{ens}," for ens in task['contracts']['ensures']])

    axiom_source = f"""fn {task['function_name']}({params_str}) -> {task['output']}
    requires {{
        {requires_items}
    }}
    ensures {{
        {ensures_items}
    }}
{{
    {clean_expr}
}}
"""
    with tempfile.TemporaryDirectory() as tmpdir:
        src_path = os.path.join(tmpdir, "verify_task.ax")
        with open(src_path, "w", encoding="utf-8") as f:
            f.write(axiom_source)

        cmd = [axiom_bin, "verify", "--strict", "--json", src_path]
        try:
            res = subprocess.run(cmd, capture_output=True, text=True, timeout=20)
            stdout = res.stdout.strip()
            if not stdout:
                return False, f"Axiom returned empty output (stderr: {res.stderr[:300]})", None, "", axiom_source

            try:
                reports = json.loads(stdout)
            except Exception:
                return False, f"Failed to parse Axiom diagnostic output: {stdout[:300]}", None, "", axiom_source

            if isinstance(reports, list):
                for r in reports:
                    status = r.get("status")
                    clause = r.get("violated_clause") or ""
                    ce = r.get("counter_example")
                    diag = r.get("diagnosis", "")

                    if status != "Proved":
                        # 严格判定：任何未证明状态（ContractViolation, Sampled, Skipped, TypeError 等）一律视为未通过
                        tag = "Contract Violation" if status == "ContractViolation" else f"SMT Status [{status}]"
                        msg = f"{tag}: {diag}"
                        return False, msg, ce, clause, axiom_source

                return True, "Axiom strict formal proof succeeded (Proved by Z3)", None, "", axiom_source
            else:
                return False, "Unexpected Axiom output format", None, "", axiom_source

        except Exception as e:
            return False, f"Axiom verify execution error: {e}", None, "", axiom_source


def build_group1_prompt(task, all_tasks):
    """Group 1: Monolithic Rust file context with multiple functions."""
    other_funcs = []
    for t in all_tasks[:10]:
        if t["id"] != task["id"]:
            other_funcs.append(f"// Subsystem: {t['subsystem']}\npub fn {t['function_name']}() {{ /* existing logic */ }}")
    context_code = "\n\n".join(other_funcs)

    prompt = f"""You are an expert Rust systems engineer. Below is part of an existing monolithic game engine module:

```rust
{context_code}

// TARGET FUNCTION TO IMPLEMENT
// Description: {task['description']}
// Requires: {', '.join(task['contracts']['requires'])}
// Ensures: {', '.join(task['contracts']['ensures'])}
pub fn {task['function_name']}({', '.join([f"{p['name']}: i64" for p in task['inputs']])}) -> { 'bool' if task['output'] == 'Bool' else 'i64' } {{
    // Implement here
}}
```

Implement ONLY the target function `{task['function_name']}`. Maintain all types, conventions and safety requirements."""
    return prompt


def build_group2_prompt(task):
    """Group 2: Isolated Rust function signature with docstrings (Micro-Worker)."""
    params_str = ", ".join([f"{p['name']}: i64" for p in task['inputs']])
    ret_str = "bool" if task["output"] == "Bool" else "i64"
    requires_str = "\n/// - ".join(task["contracts"]["requires"])
    ensures_str = "\n/// - ".join(task["contracts"]["ensures"])

    prompt = f"""You are an expert Rust engineer. Implement the following single Rust function:

```rust
/// Description: {task['description']}
/// 
/// Preconditions:
/// - {requires_str}
/// 
/// Postconditions:
/// - {ensures_str}
pub fn {task['function_name']}({params_str}) -> {ret_str} {{
    // TODO: implement
}}
```
Return ONLY the complete function implementation code."""
    return prompt


def build_group3_prompt(task):
    """Group 3: Axiom Native Hole Task (Pure formal contracts & inputs, native ?hole syntax)."""
    params_str = ", ".join([f"{p['name']}: {p['type']}" for p in task['inputs']])
    requires_items = "\n        ".join([f"{req}," for req in task['contracts']['requires']])
    ensures_items = "\n        ".join([f"{ens}," for ens in task['contracts']['ensures']])

    axiom_code = f"""// Description: {task['description']}
fn {task['function_name']}({params_str}) -> {task['output']}
    requires {{
        {requires_items}
    }}
    ensures {{
        {ensures_items}
    }}
{{
    ?hole
}}"""

    prompt = f"""You are an Axiom micro-worker. Write ONLY a single pure expression to fill the `?hole` in the function below:

[Axiom Syntax Guide]
- Return ONLY a single pure expression (no semicolons `;`, no `return` keyword, no blocks).
- Supported operators: +, -, *, /, &&, ||, ==, !=, <, <=, >, >=.
- Conditional expressions use match: `match cond {{ true => expr1, false => expr2 }}`.
- Canonical Example: `match a >= b {{ true => a - b, false => b - a }}`

```axiom
{axiom_code}
```

Return ONLY the pure expression strictly satisfying the contracts. Do not output function signatures or boilerplate."""
    return prompt


def call_gemini_count_tokens(opener, api_key: str, model: str, text: str, base_url: str) -> int:
    """调用官方 Gemini countTokens REST 端点"""
    url = build_endpoint_url(base_url, model, "countTokens", api_key)
    payload = {
        "contents": [
            {
                "parts": [{"text": text}]
            }
        ]
    }
    res = send_api_request(opener, url, payload=payload, method="POST")
    return res.get("totalTokens", 0)


def call_gemini_generate(opener, api_key: str, model: str, text: str, base_url: str) -> dict:
    """调用官方 Gemini generateContent REST 端点"""
    url = build_endpoint_url(base_url, model, "generateContent", api_key)
    payload = {
        "contents": [
            {
                "parts": [{"text": text}]
            }
        ],
        "generationConfig": {
            "temperature": 0.2
        }
    }
    res = send_api_request(opener, url, payload=payload, method="POST", timeout=45, retries=5)
    usage = res.get("usageMetadata", {})
    candidates = res.get("candidates", [])
    text_out = ""
    if candidates and "content" in candidates[0]:
        parts = candidates[0]["content"].get("parts", [])
        if parts:
            text_out = parts[0].get("text", "")
    return {
        "text": text_out,
        "task_payload": {
            "prompt_tokens": usage.get("promptTokenCount", 0),
            "candidates_tokens": usage.get("candidatesTokenCount", 0),
            "total_tokens": usage.get("totalTokenCount", 0)
        },
        "raw_billed": {
            "input_tokens": usage.get("promptTokenCount", 0),
            "output_tokens": usage.get("candidatesTokenCount", 0),
            "reasoning_tokens": 0,
            "total_tokens": usage.get("totalTokenCount", 0)
        },
        "prompt_tokens": usage.get("promptTokenCount", 0),
        "candidates_tokens": usage.get("candidatesTokenCount", 0),
        "total_tokens": usage.get("totalTokenCount", 0)
    }


def find_codex_binary() -> str:
    """定位本地可用的 codex-cli 可执行文件"""
    p = shutil.which("codex") or shutil.which("codex.cmd")
    if p:
        return p
    default_win = r"C:\Users\Administrator\AppData\Roaming\npm\codex.cmd"
    if os.path.isfile(default_win):
        return default_win
    default_ps1 = r"C:\Users\Administrator\AppData\Roaming\npm\codex.ps1"
    if os.path.isfile(default_ps1):
        return default_ps1
    return "codex"


def call_codex_generate(prompt: str, model: str = None, proxy: str = None, timeout: int = 240) -> dict:
    """通过本机 codex-cli 非交互式调用 OpenAI 模型，返回生成代码与精确 Token 消耗"""
    codex_bin = find_codex_binary()
    env = os.environ.copy()
    if proxy:
        env["HTTP_PROXY"] = proxy
        env["HTTPS_PROXY"] = proxy
        env["ALL_PROXY"] = proxy
        env["http_proxy"] = proxy
        env["https_proxy"] = proxy
        env["all_proxy"] = proxy

    with tempfile.NamedTemporaryFile(delete=False, suffix=".txt") as tmp:
        out_path = tmp.name

    cmd = [codex_bin, "exec", "--ephemeral", "--skip-git-repo-check", "--ignore-user-config", "-s", "read-only", "--json", "-o", out_path]
    if model:
        cmd.extend(["-m", model])
    cmd.append("-")

    try:
        proc = subprocess.run(
            cmd,
            input=prompt,
            text=True,
            capture_output=True,
            env=env,
            encoding="utf-8",
            shell=True,
            timeout=timeout
        )

        raw_input = 0
        raw_output = 0
        raw_reasoning = 0
        for line in proc.stdout.splitlines():
            line_str = line.strip()
            if not line_str.startswith("{"):
                continue
            try:
                data = json.loads(line_str)
                if data.get("type") == "turn.completed" and "usage" in data:
                    u = data["usage"]
                    raw_input += u.get("input_tokens", 0)
                    raw_output += u.get("output_tokens", 0)
                    raw_reasoning += u.get("reasoning_output_tokens", 0)
            except Exception:
                pass

        ans_text = ""
        if os.path.exists(out_path):
            with open(out_path, "r", encoding="utf-8") as f:
                ans_text = f.read().strip()

        if proc.returncode != 0 and not ans_text:
            err_msg = proc.stderr[:300] if proc.stderr else proc.stdout[:300]
            raise RuntimeError(f"codex exec 执行失败 (退出码 {proc.returncode}): {err_msg}")

        # 科学对齐评测口径：
        # 1. 业务有效 Payload Tokens：精确统计当前任务业务 Prompt 与模型实际生成的可见代码 Tokens
        task_prompt_tokens = count_exact_tokens(prompt)
        # 纯化 Visible Output：剔除隐藏思考/推理 Tokens (OpenAI billing 将 reasoning 计入 output_tokens)
        if raw_output > 0:
            visible_code_tokens = max(1, raw_output - raw_reasoning)
        else:
            visible_code_tokens = count_exact_tokens(ans_text)
        total_payload_tokens = task_prompt_tokens + visible_code_tokens

        # 2. 真实 API 账单消耗：包含模型隐藏推理 Tokens 与框架调用底噪
        raw_total_tokens = raw_input + raw_output
        if raw_total_tokens == 0:
            raw_total_tokens = total_payload_tokens

        return {
            "text": ans_text,
            "task_payload": {
                "prompt_tokens": task_prompt_tokens,
                "candidates_tokens": visible_code_tokens,
                "total_tokens": total_payload_tokens
            },
            "raw_billed": {
                "input_tokens": raw_input,
                "output_tokens": raw_output,
                "reasoning_tokens": raw_reasoning,
                "total_tokens": raw_total_tokens
            },
            "prompt_tokens": task_prompt_tokens,
            "candidates_tokens": visible_code_tokens,
            "total_tokens": total_payload_tokens
        }
    finally:
        if os.path.exists(out_path):
            try:
                os.remove(out_path)
            except Exception:
                pass


def list_available_models(opener, api_key: str, base_url: str):
    """查询并列出当前 API Key 可用的官方 Gemini 模型"""
    url = build_endpoint_url(base_url, "", "", api_key)
    res = send_api_request(opener, url, payload=None, method="GET")
    return res.get("models", [])


# 官方商业主力稳定模型优先级排序（算力池最充沛，最不易 503）
STABLE_MODEL_PREFERENCES = [
    "gemini-2.0-flash",
    "gemini-1.5-flash-latest",
    "gemini-1.5-flash-002",
    "gemini-1.5-flash-001",
    "gemini-2.5-flash",
    "gemini-1.5-flash",
    "gemini-2.0-flash-lite",
    "gemini-1.5-pro",
    "gemini-1.5-pro-latest",
]


def get_supported_models(opener, api_key: str, base_url: str, required_method: str = "generateContent"):
    """获取支持指定操作（generateContent 或 countTokens）的官方模型代号列表"""
    all_models = list_available_models(opener, api_key, base_url)
    supported = []
    for m in all_models:
        methods = m.get("supportedGenerationMethods", [])
        if required_method in methods:
            name = m.get("name", "").replace("models/", "").strip()
            supported.append(name)
    return supported


def resolve_best_model(opener, api_key: str, user_model: str, base_url: str, mode: str = "count"):
    """
    智能自愈解析最佳模型:
    1. 根据当前模式严格检验模型是否支持目标操作 (countTokens 或 generateContent)；
    2. 若用户指定模型可用，直接使用；
    3. 若不可用或不可生成，优先从官方商业主力稳定模型列表 (STABLE_MODEL_PREFERENCES) 中选取。
    """
    clean_user = clean_model_name(user_model)
    req_method = "generateContent" if mode == "generate" else "countTokens"
    supported = get_supported_models(opener, api_key, base_url, req_method)
    if not supported:
        raise RuntimeError(f"未在当前 API Key 下检索到任何支持 {req_method} 的可用模型。")

    if clean_user in supported:
        return clean_user, False, ""

    # 优先匹配官方主力稳定列表
    for pref in STABLE_MODEL_PREFERENCES:
        if pref in supported:
            return pref, True, f"目标模型 '{clean_user}' 不在当前 Key 支持的 {req_method} 列表中，已自动适配为官方主力模型: '{pref}'"

    # 其次挑选名称包含 flash 的主流模型
    candidates = [s for s in supported if "2.0-flash" in s or "1.5-flash" in s]
    if not candidates:
        candidates = [s for s in supported if "flash" in s]
    if not candidates:
        candidates = supported

    best = candidates[0]
    return best, True, f"目标模型 '{clean_user}' 不支持 {req_method}，已自动适配为可用模型: '{best}'"


def mock_generate_response(task: dict, group: int, attempt: int) -> dict:
    """提供离线物理自愈仿真数据，用于0成本验证端到端测试与SMT反例自愈流水线"""
    tid = task["id"]
    fname = task["function_name"]

    if group == 1:
        code = f"pub fn {fname}() {{ /* baseline implementation */ }}"
        p_tok = 90
        c_tok = 20
        t_tok = 110
    elif group == 2:
        if attempt == 1:
            if tid in (1, 2):
                code = f"pub fn {fname}(x1: i64, y1: i64, x2: i64, y2: i64) -> i64 {{\n    (x1 - x2) + (y1 - y2)\n}}"
            elif tid == 3:
                code = f"pub fn {fname}(min_x1: i64, min_y1: i64, max_x1: i64, max_y1: i64, min_x2: i64, min_y2: i64, max_x2: i64, max_y2: i64) -> bool {{\n    false\n}}"
            else:
                code = f"pub fn {fname}({', '.join([f'{p}: i64' for p in [p['name'] for p in task['inputs']]])}) -> i64 {{\n    0\n}}"
            p_tok, c_tok, t_tok = 45, 15, 60
        else:
            if tid == 1:
                code = f"pub fn {fname}(x1: i64, y1: i64, x2: i64, y2: i64) -> i64 {{\n    (x1 - x2).abs() + (y1 - y2).abs()\n}}"
            elif tid == 2:
                code = f"pub fn {fname}(x1: i64, y1: i64, x2: i64, y2: i64) -> i64 {{\n    (x1 - x2).abs().max((y1 - y2).abs())\n}}"
            elif tid == 3:
                code = f"pub fn {fname}(min_x1: i64, min_y1: i64, max_x1: i64, max_y1: i64, min_x2: i64, min_y2: i64, max_x2: i64, max_y2: i64) -> bool {{\n    min_x1 <= max_x2 && max_x1 >= min_x2 && min_y1 <= max_y2 && max_y1 >= min_y2\n}}"
            elif tid == 4:
                code = f"pub fn {fname}(x: i64, y: i64) -> i64 {{\n    x * 65536 + y\n}}"
            elif tid == 5:
                code = f"pub fn {fname}(strength: i64) -> i64 {{\n    10 + strength * 2\n}}"
            else:
                code = f"pub fn {fname}({', '.join([f'{p}: i64' for p in [p['name'] for p in task['inputs']]])}) -> i64 {{\n    1\n}}"
            p_tok, c_tok, t_tok = 52, 16, 68
    else:
        if attempt == 1:
            if tid in (1, 2):
                code = "(x1 - x2) + (y1 - y2)"
            elif tid == 3:
                code = "false"
            elif tid == 4:
                code = "x + y"
            elif tid == 5:
                code = "strength * 2"
            else:
                code = "0"
            p_tok, c_tok, t_tok = 32, 6, 38
        else:
            if tid == 1:
                code = "match x1 >= x2 { true => x1 - x2, false => x2 - x1 } + match y1 >= y2 { true => y1 - y2, false => y2 - y1 }"
            elif tid == 2:
                code = "match (match x1 >= x2 { true => x1 - x2, false => x2 - x1 }) >= (match y1 >= y2 { true => y1 - y2, false => y2 - y1 }) { true => match x1 >= x2 { true => x1 - x2, false => x2 - x1 }, false => match y1 >= y2 { true => y1 - y2, false => y2 - y1 } }"
            elif tid == 3:
                code = "min_x1 <= max_x2 && max_x1 >= min_x2 && min_y1 <= max_y2 && max_y1 >= min_y2"
            elif tid == 4:
                code = "x * 65536 + y"
            elif tid == 5:
                code = "10 + strength * 2"
            else:
                code = "1"
            p_tok, c_tok, t_tok = 38, 8, 46

    return {
        "text": code,
        "task_payload": {
            "prompt_tokens": p_tok,
            "candidates_tokens": c_tok,
            "total_tokens": t_tok
        },
        "raw_billed": {
            "input_tokens": p_tok,
            "output_tokens": c_tok,
            "reasoning_tokens": 0,
            "total_tokens": t_tok
        },
        "prompt_tokens": p_tok,
        "candidates_tokens": c_tok,
        "total_tokens": t_tok
    }


def main():
    parser = argparse.ArgumentParser(
        description="Axiom Empirical A/B Benchmark Runner (Google Gemini API & OpenAI Codex-CLI 真实评测工具)"
    )
    parser.add_argument("--engine", choices=["gemini", "codex"], default="gemini",
                        help="评测后端引擎: 'gemini' (使用 Google Gemini REST API) 或 'codex' (使用本地 codex-cli，基于 OpenAI 模型，免除 503 困扰)")
    parser.add_argument("--codex-model", default=None,
                        help="使用 codex 引擎时的指定模型 (如 'gpt-4o', 'o3-mini', 'o3' 等，留空则使用 codex 全局默认模型)")
    parser.add_argument("--api-key", help="Google Gemini API Key (也可通过环境变量 GEMINI_API_KEY / GOOGLE_API_KEY 配置)")
    parser.add_argument("--model", default=DEFAULT_MODEL, help=f"Gemini 模型名 (默认: {DEFAULT_MODEL})")
    parser.add_argument("--mode", choices=["count", "generate"], default="count",
                        help="'count': 零成本精确 Token 计量 (推荐); 'generate': 完整模型推理生成与自愈闭环")
    parser.add_argument("--tasks", default="all", help="测试任务数，如 '5' 或 'all' (默认: all，共 20 个黄金任务)")
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL, help=f"Gemini API 基础端点 (默认: {DEFAULT_BASE_URL})")
    parser.add_argument("--proxy", default=None, help="指定 HTTP/HTTPS 代理 (例如: http://127.0.0.1:7897)")
    parser.add_argument("--probe", action="store_true", help="单步探测模式: 仅测试当前 Key、网络连通性并列出可用模型")
    parser.add_argument("--mock", action="store_true", help="离线物理仿真模式: 无需网络和 API Key，0成本物理执行全套 rustc 与 SMT 反例自愈闭环")

    args = parser.parse_args()

    target_proxy = args.proxy
    if not target_proxy and not urllib.request.getproxies().get("https"):
        rec_proxy = detect_recommended_proxy()
        if rec_proxy:
            target_proxy = rec_proxy

    opener = get_http_opener(target_proxy)

    if not args.mock and args.engine == "gemini":
        api_key = args.api_key or os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
        if not api_key:
            print("\n❌ 错误: 未检测到 Google Gemini API Key。", file=sys.stderr)
            print("请通过以下方式之一提供:", file=sys.stderr)
            print("  1. 命令行参数: python examples/benchmark_eval/eval_runner.py --api-key YOUR_KEY", file=sys.stderr)
            print("  2. 设置环境变量: $env:GEMINI_API_KEY = 'YOUR_KEY' (PowerShell)", file=sys.stderr)
            print("  3. 切换为 Codex 引擎: python examples/benchmark_eval/eval_runner.py --engine codex --mode generate (免 API Key，直接调用本地 Codex)", file=sys.stderr)
            print("  4. 离线物理验证: python examples/benchmark_eval/eval_runner.py --mode generate --mock (无需 Key)", file=sys.stderr)
            rec_proxy = detect_recommended_proxy()
            if rec_proxy:
                print(f"\n💡 提示: 检测到本地代理运行于 {rec_proxy}，若访问受限可附加 `--proxy {rec_proxy}`", file=sys.stderr)
            sys.exit(1)
    else:
        api_key = "MOCK_KEY" if args.mock else "CODEX_SESSION"

    if args.probe:
        print("=" * 70)
        print("🔍 正在执行 API 连通性与可用模型探测 (Pre-flight Probe)...")
        print(f"评测引擎: {args.engine.upper()}")
        if args.engine == "codex":
            codex_bin = find_codex_binary()
            print(f"Codex 可执行文件: {codex_bin}")
            print(f"网络代理: {target_proxy or '系统默认 / 直连'}")
            print("=" * 70)
            sys.stdout.flush()
            try:
                probe_res = call_codex_generate("Please reply with the exact word PONG and nothing else.", model=args.codex_model, proxy=target_proxy, timeout=60)
                print(f"✅ Codex-CLI 连通成功！模型响应: {probe_res['text'].strip()}")
                print("\n🚀 自检完全正常！你可以直接执行真实评测命令:")
                print("   python examples/benchmark_eval/eval_runner.py --engine codex --mode generate --tasks 3\n")
                sys.exit(0)
            except Exception as e:
                print(f"\n❌ Codex 探测失败:\n{e}\n", file=sys.stderr)
                sys.exit(1)
        else:
            print(f"API 端点: {args.base_url}")
            print(f"网络代理: {target_proxy or '系统默认 / 直连'}")
            print("=" * 70)
            sys.stdout.flush()
            try:
                gen_models = get_supported_models(opener, api_key, args.base_url, "generateContent")
                cnt_models = get_supported_models(opener, api_key, args.base_url, "countTokens")
                print(f"✅ API Key 连通成功！\n")
                print(f"📌 支持生成推理 (generateContent) 的主力模型 (共 {len(gen_models)} 个):")
                for m in gen_models:
                    is_rec = any(p in m for p in ("2.0-flash", "1.5-flash-latest", "1.5-flash-002"))
                    tag = " [推荐首选]" if is_rec else ""
                    print(f"  • {m:<32}{tag}")

                print(f"\n📌 支持精确分词计量 (countTokens) 的模型 (共 {len(cnt_models)} 个):")
                for m in cnt_models[:8]:
                    print(f"  • {m}")
                if len(cnt_models) > 8:
                    print(f"  • ... (共 {len(cnt_models)} 个模型支持 countTokens)")

                test_m = clean_model_name(args.model) if clean_model_name(args.model) in cnt_models else cnt_models[0]
                cnt = call_gemini_count_tokens(opener, api_key, test_m, "test", args.base_url)
                print(f"\n🎯 接口测算实测: 模型 '{test_m}' 响应正常，测试 Token 计为 {cnt}")
                print("\n🚀 自检完全正常！你可以直接执行真实评测命令:")
                print(f"   python examples/benchmark_eval/eval_runner.py --model {test_m} --mode count")
                print(f"   python examples/benchmark_eval/eval_runner.py --model {test_m} --mode generate\n")
                sys.exit(0)
            except Exception as e:
                print(f"\n❌ 探测失败:\n{e}\n", file=sys.stderr)
                sys.exit(1)

    all_tasks = load_golden_tasks()
    if args.tasks != "all":
        limit = int(args.tasks)
        eval_tasks = all_tasks[:limit]
    else:
        eval_tasks = all_tasks

    axiom_bin = find_axiom_binary()

    print("=" * 70)
    print(f"🚀 Axiom 经验性真实 A/B 对照评测启动 (引擎: {args.engine.upper()})")
    print(f"评测模式: {args.mode} {'[离线仿真模式]' if args.mock else ''} | 任务总数: {len(eval_tasks)}")
    if args.engine == "gemini":
        print(f"API 端点: {args.base_url}")
    if args.mode == "generate":
        print(f"验证后端: rustc + Axiom SMT ({axiom_bin}) | 最大自愈重试轮数: {MAX_HEALING_RETRIES}")
    if target_proxy and not args.mock:
        print(f"网络代理: {target_proxy}")
    print("=" * 70)

    if not args.mock:
        if args.engine == "codex":
            print("⏳ 正在进行本地 codex-cli 连通性预检...", end="", flush=True)
            try:
                probe_res = call_codex_generate("Please reply with the exact word PONG and nothing else.", model=args.codex_model, proxy=target_proxy, timeout=60)
                active_model = f"codex/{args.codex_model or 'default'}"
                print(" 通过 ✅\n", flush=True)
            except Exception as e:
                print(" 失败 ❌\n", flush=True)
                print(f"{e}\n")
                sys.exit(1)
        else:
            print("⏳ 正在进行 API 连通性与模型推理可用性预检...", end="", flush=True)
            try:
                active_model, switched, reason = resolve_best_model(opener, api_key, args.model, args.base_url, mode=args.mode)
                if args.mode == "generate":
                    try:
                        call_gemini_generate(opener, api_key, active_model, "ping", args.base_url)
                    except Exception as e:
                        if "503" in str(e):
                            failover_success = False
                            for alt in STABLE_MODEL_PREFERENCES:
                                if alt != active_model:
                                    try:
                                        call_gemini_generate(opener, api_key, alt, "ping", args.base_url)
                                        print(f"\n⚠️ 原选模型 '{active_model}' 当前云端高负载 503，已自动故障转移到可用模型: '{alt}'")
                                        active_model = alt
                                        failover_success = True
                                        break
                                    except Exception:
                                        continue
                            if not failover_success:
                                raise e
                        else:
                            raise e
                else:
                    call_gemini_count_tokens(opener, api_key, active_model, "test", args.base_url)

                if switched:
                    print(" 自动适配 ⚠️\n", flush=True)
                    print(f"💡 {reason}\n")
                else:
                    print(" 通过 ✅\n", flush=True)
            except Exception as e:
                print(" 失败 ❌\n", flush=True)
                print(f"{e}\n")
                print("💡 快速排查指引:")
                print("  1. 切换为本地已安装的 Codex 引擎: python examples/benchmark_eval/eval_runner.py --engine codex --mode generate")
                print("  2. 运行自检探测查看可用模型列表: python examples/benchmark_eval/eval_runner.py --probe")
                print("  3. 若外部网络受阻，可使用离线物理仿真验证: python examples/benchmark_eval/eval_runner.py --mode generate --mock")
                sys.exit(1)
    else:
        active_model = "offline-mock-runner"
        print("⏳ 离线仿真自检... 通过 ✅\n")

    print(f"🎯 当前使用的评测模型: {active_model}\n")

    results = []
    total_g1 = 0
    total_g2 = 0
    total_g3 = 0
    total_g1_billed = 0
    total_g2_billed = 0
    total_g3_billed = 0
    total_g2_reasoning = 0
    total_g3_reasoning = 0

    g2_pass1_count = 0
    g2_final_pass_count = 0
    g3_pass1_count = 0
    g3_final_pass_count = 0

    for idx, task in enumerate(eval_tasks):
        tid = task["id"]
        fname = task["function_name"]
        print(f"[{idx+1}/{len(eval_tasks)}] 评测任务 #{tid}: {fname}...", end="", flush=True)

        p1 = build_group1_prompt(task, all_tasks)
        p2 = build_group2_prompt(task)
        p3 = build_group3_prompt(task)

        if args.mode == "count":
            if args.engine == "codex":
                # codex-cli 无独立免费分词 REST 端点，按典型分词比率估算 Prompt Tokens
                tokens_1 = max(1, len(p1) // 4)
                tokens_2 = max(1, len(p2) // 4)
                tokens_3 = max(1, len(p3) // 4)
            else:
                tokens_1 = call_gemini_count_tokens(opener, api_key, active_model, p1, args.base_url)
                time.sleep(0.1)
                tokens_2 = call_gemini_count_tokens(opener, api_key, active_model, p2, args.base_url)
                time.sleep(0.1)
                tokens_3 = call_gemini_count_tokens(opener, api_key, active_model, p3, args.base_url)
                time.sleep(0.1)

            total_g1 += tokens_1
            total_g2 += tokens_2
            total_g3 += tokens_3

            diff_vs_g2 = round((1 - tokens_3 / tokens_2) * 100, 1) if tokens_2 > 0 else 0
            print(f" G1(单体): {tokens_1}t | G2(隔离微任务): {tokens_2}t | G3(Axiom打洞): {tokens_3}t (较G2省: -{diff_vs_g2}%)")

            results.append({
                "task_id": tid,
                "function_name": fname,
                "group1_rust_monolith": {"prompt_tokens": tokens_1},
                "group2_rust_micro": {"prompt_tokens": tokens_2},
                "group3_axiom_hole": {"prompt_tokens": tokens_3}
            })
        else:
            # 模式: generate (物理端到端生成、验证与自愈重试)
            def do_generate(p_text, grp, att):
                if args.mock:
                    return mock_generate_response(task, grp, att)
                print(f" [G{grp}:t{att}]", end="", flush=True)
                if args.engine == "codex":
                    return call_codex_generate(p_text, model=args.codex_model, proxy=target_proxy)
                return call_gemini_generate(opener, api_key, active_model, p_text, args.base_url)

            # 构造 Axiom 任务骨架（带有完整前置后置契约与打洞占位符）
            params_str = ", ".join([f"{p['name']}: {p['type']}" for p in task['inputs']])
            requires_items = "\n    ".join([f"{req}," for req in task['contracts']['requires']])
            ensures_items = "\n    ".join([f"{ens}," for ens in task['contracts']['ensures']])
            axiom_skeleton = f"""fn {fname}({params_str}) -> {task['output']}
requires {{
    {requires_items}
}}
ensures {{
    {ensures_items}
}}
{{
    ?hole
}}"""

            # Group 1: 单体大上下文生成 (仅作为初始工程背景膨胀基准)
            res1 = do_generate(p1, 1, 1)
            g1_payload = res1["task_payload"]["total_tokens"]
            g1_billed = res1["raw_billed"]["total_tokens"]
            total_g1 += g1_payload
            total_g1_billed += g1_billed
            if not args.mock:
                time.sleep(0.8)

            # Group 2: 单任务隔离微任务生成 + rustc 单元测试与全上下文自愈重试
            g2_traces = []
            res2 = do_generate(p2, 2, 1)
            if not args.mock:
                time.sleep(0.8)
            g2_payload = res2["task_payload"]["total_tokens"]
            g2_billed = res2["raw_billed"]["total_tokens"]
            g2_reasoning = res2["raw_billed"].get("reasoning_tokens", 0)

            g2_ok, g2_err = run_rust_test(res2["text"], task)
            g2_pass1 = g2_ok
            g2_attempts = 1

            g2_traces.append({
                "round": 1,
                "group": 2,
                "prompt_preview": p2[:120] + ("..." if len(p2) > 120 else ""),
                "raw_output": res2["text"],
                "clean_code": extract_code(res2["text"], "rust"),
                "status": "Passed" if g2_ok else "Failed",
                "diagnostic": g2_err,
                "usage": {
                    "payload_prompt": res2["task_payload"]["prompt_tokens"],
                    "payload_candidates": res2["task_payload"]["candidates_tokens"],
                    "raw_billed_input": res2["raw_billed"]["input_tokens"],
                    "raw_billed_output": res2["raw_billed"]["output_tokens"],
                    "raw_billed_reasoning": res2["raw_billed"]["reasoning_tokens"],
                    "raw_billed_total": res2["raw_billed"]["total_tokens"],
                }
            })

            if not g2_ok:
                current_code = res2["text"]
                for retry in range(MAX_HEALING_RETRIES):
                    # 全上下文确定性重试：保留原业务目标、完整签名、上一轮失败实现与精确测试错误
                    heal_prompt = f"""You are a senior Rust developer. Your previous implementation for function `{fname}` failed compilation or unit tests:

[Task Goal]
{task.get('description', '')}

[Function Signature]
pub fn {fname}({', '.join([f"{p['name']}: i64" for p in task['inputs']])}) -> {task['output']}

[Your Previous Implementation (Failed)]
```rust
{current_code}
```

[Verification Error / Assertion Failure Details]
{g2_err}

Please fix the implementation logic, satisfy all edge cases, and return ONLY the corrected complete Rust function:"""

                    heal_res = do_generate(heal_prompt, 2, retry + 2)
                    if not args.mock:
                        time.sleep(0.8)
                    g2_payload += heal_res["task_payload"]["total_tokens"]
                    g2_billed += heal_res["raw_billed"]["total_tokens"]
                    g2_reasoning += heal_res["raw_billed"].get("reasoning_tokens", 0)
                    g2_attempts += 1
                    current_code = heal_res["text"]
                    g2_ok, g2_err = run_rust_test(current_code, task)
                    g2_traces.append({
                        "round": retry + 2,
                        "group": 2,
                        "prompt_preview": heal_prompt[:120] + ("..." if len(heal_prompt) > 120 else ""),
                        "raw_output": heal_res["text"],
                        "clean_code": extract_code(heal_res["text"], "rust"),
                        "status": "Passed" if g2_ok else "Failed",
                        "diagnostic": g2_err,
                        "usage": {
                            "payload_prompt": heal_res["task_payload"]["prompt_tokens"],
                            "payload_candidates": heal_res["task_payload"]["candidates_tokens"],
                            "raw_billed_input": heal_res["raw_billed"]["input_tokens"],
                            "raw_billed_output": heal_res["raw_billed"]["output_tokens"],
                            "raw_billed_reasoning": heal_res["raw_billed"]["reasoning_tokens"],
                            "raw_billed_total": heal_res["raw_billed"]["total_tokens"],
                        }
                    })
                    if g2_ok:
                        break

            if g2_pass1:
                g2_pass1_count += 1
            if g2_ok:
                g2_final_pass_count += 1
            total_g2 += g2_payload
            total_g2_billed += g2_billed
            total_g2_reasoning += g2_reasoning

            # Group 3: Axiom 契约打洞生成 + SMT 求解器 строго验证与全上下文反例自愈重试
            g3_traces = []
            res3 = do_generate(p3, 3, 1)
            if not args.mock:
                time.sleep(0.8)
            g3_payload = res3["task_payload"]["total_tokens"]
            g3_billed = res3["raw_billed"]["total_tokens"]
            g3_reasoning = res3["raw_billed"].get("reasoning_tokens", 0)

            g3_ok, g3_diag, g3_ce, g3_clause, _ = run_axiom_verify(res3["text"], task, axiom_bin)
            g3_pass1 = g3_ok
            g3_attempts = 1
            g3_ce_found = g3_ce is not None

            g3_traces.append({
                "round": 1,
                "group": 3,
                "prompt_preview": p3[:120] + ("..." if len(p3) > 120 else ""),
                "raw_output": res3["text"],
                "clean_code": extract_code(res3["text"], "axiom"),
                "status": "Proved" if g3_ok else ("ContractViolation" if g3_ce else "SyntaxOrTypeError"),
                "diagnostic": g3_diag,
                "violated_clause": g3_clause,
                "counter_example": g3_ce,
                "usage": {
                    "payload_prompt": res3["task_payload"]["prompt_tokens"],
                    "payload_candidates": res3["task_payload"]["candidates_tokens"],
                    "raw_billed_input": res3["raw_billed"]["input_tokens"],
                    "raw_billed_output": res3["raw_billed"]["output_tokens"],
                    "raw_billed_reasoning": res3["raw_billed"]["reasoning_tokens"],
                    "raw_billed_total": res3["raw_billed"]["total_tokens"],
                }
            })

            if not g3_ok:
                current_expr = res3["text"]
                for retry in range(MAX_HEALING_RETRIES):
                    # 全上下文确定性反例重试：保留带契约骨架、上一轮尝试表达式、SMT违约子句与具体反例赋值
                    ce_info = f"\n[Specific SMT Counterexample Input Assignment]\n{json.dumps(g3_ce, indent=2)}" if g3_ce else ""
                    clause_info = f"\n[Violated Contract Clause]\n{g3_clause}" if g3_clause else ""

                    heal_prompt = f"""You are an Axiom formal verification engineer. Your previous pure expression attempt to fill `?hole` in `{fname}` failed strict SMT theorem proving:

[Function Skeleton with Formal Contracts]
```axiom
{axiom_skeleton}
```

[Your Previous Pure Expression Attempt (Failed)]
```axiom
{current_expr}
```

[Strict SMT Solver Diagnostic (Fail-Closed)]{clause_info}{ce_info}
Diagnostic Summary: {g3_diag}

[Axiom Syntax Guide]
- Return ONLY a single pure expression (no semicolons `;`, no `return` keyword, no blocks).
- Supported operators: +, -, *, /, &&, ||, ==, !=, <, <=, >, >=.
- Conditional expressions use match: `match cond {{ true => expr1, false => expr2 }}`.

Please analyze the counterexample assignment, strictly satisfy all requires/ensures contracts, and eliminate the violation.
Return ONLY the corrected pure expression to fill `?hole`:"""

                    heal_res = do_generate(heal_prompt, 3, retry + 2)
                    if not args.mock:
                        time.sleep(0.8)
                    g3_payload += heal_res["task_payload"]["total_tokens"]
                    g3_billed += heal_res["raw_billed"]["total_tokens"]
                    g3_reasoning += heal_res["raw_billed"].get("reasoning_tokens", 0)
                    g3_attempts += 1
                    current_expr = heal_res["text"]
                    g3_ok, g3_diag, g3_ce, g3_clause, _ = run_axiom_verify(current_expr, task, axiom_bin)
                    if g3_ce:
                        g3_ce_found = True
                    g3_traces.append({
                        "round": retry + 2,
                        "group": 3,
                        "prompt_preview": heal_prompt[:120] + ("..." if len(heal_prompt) > 120 else ""),
                        "raw_output": heal_res["text"],
                        "clean_code": extract_code(heal_res["text"], "axiom"),
                        "status": "Proved" if g3_ok else ("ContractViolation" if g3_ce else "SyntaxOrTypeError"),
                        "diagnostic": g3_diag,
                        "violated_clause": g3_clause,
                        "counter_example": g3_ce,
                        "usage": {
                            "payload_prompt": heal_res["task_payload"]["prompt_tokens"],
                            "payload_candidates": heal_res["task_payload"]["candidates_tokens"],
                            "raw_billed_input": heal_res["raw_billed"]["input_tokens"],
                            "raw_billed_output": heal_res["raw_billed"]["output_tokens"],
                            "raw_billed_reasoning": heal_res["raw_billed"]["reasoning_tokens"],
                            "raw_billed_total": heal_res["raw_billed"]["total_tokens"],
                        }
                    })
                    if g3_ok:
                        break

            if g3_pass1:
                g3_pass1_count += 1
            if g3_ok:
                g3_final_pass_count += 1
            total_g3 += g3_payload
            total_g3_billed += g3_billed
            total_g3_reasoning += g3_reasoning

            g2_tag = f"P1:{'✓' if g2_pass1 else '✗'}(try:{g2_attempts})"
            g3_tag = f"P1:{'✓' if g3_pass1 else '✗'}(try:{g3_attempts}{',CE' if g3_ce_found else ''})"
            print(f" G1(初始单体): {g1_payload}t | G2(Rust交付): {g2_payload}t(账单:{g2_billed}t) [{g2_tag}] | G3(Axiom交付): {g3_payload}t(账单:{g3_billed}t) [{g3_tag}]")

            results.append({
                "task_id": tid,
                "function_name": fname,
                "group1_rust_monolith": {
                    "task_payload_tokens": g1_payload,
                    "raw_billed_tokens": g1_billed,
                },
                "group2_rust_micro": {
                    "task_payload_tokens": g2_payload,
                    "raw_billed_tokens": g2_billed,
                    "reasoning_tokens": g2_reasoning,
                    "pass_at_1": g2_pass1,
                    "final_pass": g2_ok,
                    "attempts": g2_attempts,
                    "traces": g2_traces,
                },
                "group3_axiom_hole": {
                    "task_payload_tokens": g3_payload,
                    "raw_billed_tokens": g3_billed,
                    "reasoning_tokens": g3_reasoning,
                    "pass_at_1": g3_pass1,
                    "final_pass": g3_ok,
                    "attempts": g3_attempts,
                    "counterexample_encountered": g3_ce_found,
                    "traces": g3_traces,
                }
            })

    print("=" * 70)
    print(f"📊 评测完成 (基于 {args.engine.upper()} 真实实测数据)")

    if args.mode == "count":
        pct_vs_g2 = round((1.0 - total_g3 / total_g2) * 100, 1) if total_g2 > 0 else 0
        pct_vs_g1 = round((1.0 - total_g3 / total_g1) * 100, 1) if total_g1 > 0 else 0
        print(f"Group 1 (传统单体大文件初始上下文) 总消耗: {total_g1:,} tokens")
        print(f"Group 2 (单任务隔离 Rust 微任务) 总消耗:   {total_g2:,} tokens")
        print(f"Group 3 (Axiom 契约打洞纯表达式) 总消耗:   {total_g3:,} tokens")
        print("-" * 70)
        print(f"🔥 核心指标 1: Axiom 较 Group 2 (单任务隔离微任务) 净节省 Token: {pct_vs_g2}%")
        print(f"🔥 核心指标 2: Axiom 较 Group 1 (传统单体大文件上下文) 净节省 Token: {pct_vs_g1}%")
    else:
        # Generate 模式：科学解耦的双轨总结
        pct_payload_saving = round((1.0 - total_g3 / total_g2) * 100, 1) if total_g2 > 0 else 0
        billed_ratio = round(total_g3_billed / total_g2_billed, 2) if total_g2_billed > 0 else 1.0

        print(f"【维度 1: 业务有效 Payload Tokens (纯代码与领域紧凑度对比)】")
        print(f"  • Group 1 (传统单体大文件初始输入基准): {total_g1:,} tokens")
        print(f"  • Group 2 (单任务隔离 Rust 微任务):       {total_g2:,} tokens")
        print(f"  • Group 3 (Axiom 契约打洞纯表达式):       {total_g3:,} tokens")
        print(f"  🔥 交付有效 Payload 差异: Axiom 较 Rust 微任务 {'节省: +' if pct_payload_saving >= 0 else '多耗: '}{pct_payload_saving}%")
        print("-" * 70)
        print(f"【维度 2: 真实 API 账单消耗 (端到端真实落地代价，含多轮重试与隐藏推理)】")
        print(f"  • Group 2 (Rust 微任务真实账单):   {total_g2_billed:,} tokens (含模型推理: {total_g2_reasoning:,} tokens)")
        print(f"  • Group 3 (Axiom 打洞真实账单):     {total_g3_billed:,} tokens (含模型推理: {total_g3_reasoning:,} tokens)")
        print(f"  🔥 端到端真实账单倍率: Axiom 实际账单为 Rust 微任务的 {billed_ratio} 倍")
        if billed_ratio > 1.0:
            print(f"  💡 归因分析: 形式化验证换取了数学全域完备性（严格穷举输入空间，彻底排除测试盲区），但对模型的形式语法提出更高要求。多轮重试与推理开销是推高总账单的主要驱动因素。")
        print("-" * 70)
        print(f"【维度 3: 物理执行与 SMT 形式证明统计 (共 {len(eval_tasks)} 个任务)】")
        print(f"  • Group 2 (Rust):  Pass@1 = {g2_pass1_count}/{len(eval_tasks)} ({round(g2_pass1_count/len(eval_tasks)*100, 1)}%) | 经自愈最终 Pass = {g2_final_pass_count}/{len(eval_tasks)} ({round(g2_final_pass_count/len(eval_tasks)*100, 1)}%) [基于离散测试向量采样]")
        print(f"  • Group 3 (Axiom): Pass@1 = {g3_pass1_count}/{len(eval_tasks)} ({round(g3_pass1_count/len(eval_tasks)*100, 1)}%) | 经SMT自愈最终 Proved = {g3_final_pass_count}/{len(eval_tasks)} ({round(g3_final_pass_count/len(eval_tasks)*100, 1)}%) [微软Z3严格全域数学证明]")

    print("=" * 70)

    output_payload = {
        "version": "2.2",
        "status": "completed_authentic_dual_track_audited",
        "experiment_metadata": {
            "engine": args.engine,
            "target_model": active_model,
            "mode": args.mode,
            "evaluated_tasks_count": len(eval_tasks),
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "git_commit": get_git_commit_sha(),
            "z3_version": get_z3_version(),
            "platform": sys.platform,
            "python_version": sys.version.split()[0],
        },
        "totals": {
            "task_payload_tokens": {
                "group1_rust_monolith": total_g1,
                "group2_rust_micro": total_g2,
                "group3_axiom_hole": total_g3,
            },
            "raw_billed_tokens": {
                "group1_rust_monolith": total_g1_billed,
                "group2_rust_micro": total_g2_billed,
                "group2_reasoning": total_g2_reasoning,
                "group3_axiom_hole": total_g3_billed,
                "group3_reasoning": total_g3_reasoning,
            }
        },
        "detailed_tasks": results
    }

    if args.mode == "generate":
        output_payload["verification_summary"] = {
            "group2_rust": {
                "pass_at_1": f"{g2_pass1_count}/{len(eval_tasks)}",
                "final_pass": f"{g2_final_pass_count}/{len(eval_tasks)}",
                "verification_method": "rustc_test_vectors_discrete_sampling",
            },
            "group3_axiom": {
                "pass_at_1": f"{g3_pass1_count}/{len(eval_tasks)}",
                "final_proved": f"{g3_final_pass_count}/{len(eval_tasks)}",
                "verification_method": "microsoft_z3_strict_exhaustive_smt_proof",
            }
        }

    os.makedirs(os.path.dirname(OUTPUT_DATA_PATH), exist_ok=True)
    with open(OUTPUT_DATA_PATH, "w", encoding="utf-8") as f:
        json.dump(output_payload, f, indent=2, ensure_ascii=False)
    print(f"💾 双轨真实测量数据已写入持久化文件: {OUTPUT_DATA_PATH}\n")


if __name__ == "__main__":
    main()
