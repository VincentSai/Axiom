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


def run_axiom_verify(expr: str, task: dict, axiom_bin: str) -> tuple[bool, str, dict | None]:
    """通过 Axiom SMT 求解器验证契约与效果安全"""
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

        cmd = [axiom_bin, "verify", "--json", src_path]
        try:
            res = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
            stdout = res.stdout.strip()
            if not stdout:
                return False, f"Axiom returned empty output (stderr: {res.stderr[:300]})", None

            try:
                reports = json.loads(stdout)
            except Exception:
                return False, f"Failed to parse Axiom diagnostic output: {stdout[:300]}", None

            if isinstance(reports, list):
                for r in reports:
                    status = r.get("status")
                    if status == "ContractViolation":
                        diag = r.get("diagnosis", "Contract violation")
                        clause = r.get("violated_clause", "")
                        ce = r.get("counter_example")
                        msg = f"Contract Violation ({clause}): {diag}"
                        return False, msg, ce
                    elif status in ("TypeError", "EffectViolation", "ParseError", "UnresolvedHole"):
                        diag = r.get("diagnosis", f"Axiom diagnostic error: {status}")
                        return False, diag, None
                    elif status == "Skipped" and r.get("has_contracts"):
                        diag = r.get("diagnosis", "Function contracts skipped (fail-closed)")
                        return False, diag, None

                return True, "Axiom formal verification passed", None
            else:
                return False, "Unexpected Axiom output format", None

        except Exception as e:
            return False, f"Axiom verify execution error: {e}", None


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
        "prompt_tokens": usage.get("promptTokenCount", 0),
        "candidates_tokens": usage.get("candidatesTokenCount", 0),
        "total_tokens": usage.get("totalTokenCount", 0)
    }


def list_available_models(opener, api_key: str, base_url: str):
    """查询并列出当前 API Key 可用的官方 Gemini 模型"""
    url = build_endpoint_url(base_url, "", "", api_key)
    res = send_api_request(opener, url, payload=None, method="GET")
    return res.get("models", [])


def get_supported_count_tokens_models(opener, api_key: str, base_url: str):
    """获取所有支持 countTokens 的官方 Gemini 模型代号列表"""
    all_models = list_available_models(opener, api_key, base_url)
    supported = []
    for m in all_models:
        methods = m.get("supportedGenerationMethods", [])
        if "countTokens" in methods:
            name = m.get("name", "").replace("models/", "").strip()
            supported.append(name)
    return supported


def resolve_best_model(opener, api_key: str, user_model: str, base_url: str):
    """
    智能自愈解析最佳模型:
    1. 若指定模型在支持列表中，直接使用；
    2. 若不在支持列表（例如 Google 升级后 gemini-1.5-flash 需用具体后缀）：
       自动在当前 Key 支持的列表中匹配最适模型并返回自适应提示。
    """
    clean_user = clean_model_name(user_model)
    supported = get_supported_count_tokens_models(opener, api_key, base_url)
    if not supported:
        raise RuntimeError("未在当前 API Key 下检索到任何支持 countTokens 的模型。")

    if clean_user in supported:
        return clean_user, False, ""

    # 优先匹配 1.5-flash 系列 (如 gemini-1.5-flash-latest, gemini-1.5-flash-001, gemini-1.5-flash-002)
    candidates = [s for s in supported if "1.5-flash" in s]
    if not candidates:
        # 其次匹配任意 flash 系列 (如 gemini-2.0-flash)
        candidates = [s for s in supported if "flash" in s]
    if not candidates:
        candidates = supported

    best = candidates[0]
    return best, True, f"目标模型 '{clean_user}' 不支持 countTokens，已自动适配为当前 Key 支持的最佳模型: '{best}'"


def main():
    parser = argparse.ArgumentParser(
        description="Axiom Empirical A/B Benchmark Runner (Google Gemini API 真实评测工具)"
    )
    parser.add_argument("--api-key", help="Google Gemini API Key (也可通过环境变量 GEMINI_API_KEY / GOOGLE_API_KEY 配置)")
    parser.add_argument("--model", default=DEFAULT_MODEL, help=f"Gemini 模型名 (默认: {DEFAULT_MODEL})")
    parser.add_argument("--mode", choices=["count", "generate"], default="count",
                        help="'count': 零成本精确 Token 计量 (推荐); 'generate': 完整模型推理生成")
    parser.add_argument("--tasks", default="all", help="测试任务数，如 '5' 或 'all' (默认: all，共 20 个黄金任务)")
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL, help=f"Gemini API 基础端点 (默认: {DEFAULT_BASE_URL})")
    parser.add_argument("--proxy", default=None, help="指定 HTTP/HTTPS 代理 (例如: http://127.0.0.1:7897)")
    parser.add_argument("--probe", action="store_true", help="单步探测模式: 仅测试当前 Key、网络连通性并列出可用模型")

    args = parser.parse_args()

    api_key = args.api_key or os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
    if not api_key:
        print("\n❌ 错误: 未检测到 Google Gemini API Key。", file=sys.stderr)
        print("请通过以下方式之一提供:", file=sys.stderr)
        print("  1. 命令行参数: python examples/benchmark_eval/eval_runner.py --api-key YOUR_KEY", file=sys.stderr)
        print("  2. 设置环境变量: $env:GEMINI_API_KEY = 'YOUR_KEY' (PowerShell)", file=sys.stderr)
        rec_proxy = detect_recommended_proxy()
        if rec_proxy:
            print(f"\n💡 提示: 检测到本地代理运行于 {rec_proxy}，若访问受限可附加 `--proxy {rec_proxy}`", file=sys.stderr)
        sys.exit(1)

    target_proxy = args.proxy
    if not target_proxy and not urllib.request.getproxies().get("https"):
        rec_proxy = detect_recommended_proxy()
        if rec_proxy:
            target_proxy = rec_proxy

    opener = get_http_opener(target_proxy)

    if args.probe:
        print("=" * 70)
        print("🔍 正在执行 API 连通性与可用模型探测 (Pre-flight Probe)...")
        print(f"API 端点: {args.base_url}")
        print(f"网络代理: {target_proxy or '系统默认 / 直连'}")
        print("=" * 70)
        sys.stdout.flush()
        try:
            supported_models = get_supported_count_tokens_models(opener, api_key, args.base_url)
            print(f"✅ API Key 连通成功！当前 Key 支持 countTokens 的模型 (共 {len(supported_models)} 个):\n")
            for m in supported_models:
                is_rec = ("1.5-flash" in m or "flash" in m)
                tag = " [推荐首选]" if is_rec else ""
                print(f"  • {m:<32}{tag}")

            test_m = clean_model_name(args.model) if clean_model_name(args.model) in supported_models else supported_models[0]
            cnt = call_gemini_count_tokens(opener, api_key, test_m, "test", args.base_url)
            print(f"\n🎯 接口测算实测: 模型 '{test_m}' 响应正常，测试 Token 计为 {cnt}")
            print("\n🚀 自检完全正常！你可以直接执行真实评测命令:")
            print(f"   python examples/benchmark_eval/eval_runner.py --model {test_m} --mode count\n")
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
    print("🚀 Axiom 经验性真实 A/B 对照评测启动 (Empirical Benchmark Runner)")
    print(f"评测模式: {args.mode} | 任务总数: {len(eval_tasks)} | API 端点: {args.base_url}")
    if args.mode == "generate":
        print(f"验证后端: rustc + Axiom SMT ({axiom_bin}) | 最大自愈重试轮数: {MAX_HEALING_RETRIES}")
    if target_proxy:
        print(f"网络代理: {target_proxy}")
    print("=" * 70)

    print("⏳ 正在进行 API 连通性与模型可用性预检...", end="", flush=True)
    try:
        active_model, switched, reason = resolve_best_model(opener, api_key, args.model, args.base_url)
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
        print("  1. 运行自检探测查看可用模型列表: python examples/benchmark_eval/eval_runner.py --probe")
        if target_proxy:
            print(f"  2. 当前已启用代理: {target_proxy}")
        else:
            rec = detect_recommended_proxy()
            if rec:
                print(f"  2. 尝试添加本地代理: --proxy {rec}")
        sys.exit(1)

    print(f"🎯 当前使用的评测模型: {active_model}\n")

    results = []
    total_g1 = 0
    total_g2 = 0
    total_g3 = 0

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
            # Group 1: 单体大上下文生成 (基准对照)
            res1 = call_gemini_generate(opener, api_key, active_model, p1, args.base_url)
            total_g1 += res1["total_tokens"]
            time.sleep(0.8)

            # Group 2: 单任务隔离微任务生成 + rustc 单元测试与编译错误自愈重试
            res2 = call_gemini_generate(opener, api_key, active_model, p2, args.base_url)
            time.sleep(0.8)
            g2_tokens = res2["total_tokens"]
            g2_ok, g2_err = run_rust_test(res2["text"], task)
            g2_pass1 = g2_ok
            g2_attempts = 1

            if not g2_ok:
                current_code = res2["text"]
                for retry in range(MAX_HEALING_RETRIES):
                    heal_prompt = f"""Your previous Rust implementation for `{fname}` failed compilation or unit tests:

{g2_err}

Please fix the implementation and return ONLY the corrected complete Rust function:"""
                    heal_res = call_gemini_generate(opener, api_key, active_model, heal_prompt, args.base_url)
                    time.sleep(0.8)
                    g2_tokens += heal_res["total_tokens"]
                    g2_attempts += 1
                    current_code = heal_res["text"]
                    g2_ok, g2_err = run_rust_test(current_code, task)
                    if g2_ok:
                        break

            if g2_pass1:
                g2_pass1_count += 1
            if g2_ok:
                g2_final_pass_count += 1
            total_g2 += g2_tokens

            # Group 3: Axiom 契约打洞生成 + SMT 求解器验证与反例自愈重试
            res3 = call_gemini_generate(opener, api_key, active_model, p3, args.base_url)
            time.sleep(0.8)
            g3_tokens = res3["total_tokens"]
            g3_ok, g3_diag, g3_ce = run_axiom_verify(res3["text"], task, axiom_bin)
            g3_pass1 = g3_ok
            g3_attempts = 1
            g3_ce_found = g3_ce is not None

            if not g3_ok:
                current_expr = res3["text"]
                for retry in range(MAX_HEALING_RETRIES):
                    ce_info = f"\nSpecific SMT Counterexample inputs: {json.dumps(g3_ce)}" if g3_ce else ""
                    heal_prompt = f"""Axiom formal verification failed for `{fname}`:
Diagnostic: {g3_diag}{ce_info}

Please refine your pure expression to satisfy all formal contracts and eliminate counterexamples.
Return ONLY the corrected pure expression to fill `?hole`:"""
                    heal_res = call_gemini_generate(opener, api_key, active_model, heal_prompt, args.base_url)
                    time.sleep(0.8)
                    g3_tokens += heal_res["total_tokens"]
                    g3_attempts += 1
                    current_expr = heal_res["text"]
                    g3_ok, g3_diag, g3_ce = run_axiom_verify(current_expr, task, axiom_bin)
                    if g3_ce:
                        g3_ce_found = True
                    if g3_ok:
                        break

            if g3_pass1:
                g3_pass1_count += 1
            if g3_ok:
                g3_final_pass_count += 1
            total_g3 += g3_tokens

            g2_tag = f"P1:{'✓' if g2_pass1 else '✗'}(try:{g2_attempts})"
            g3_tag = f"P1:{'✓' if g3_pass1 else '✗'}(try:{g3_attempts}{',CE' if g3_ce_found else ''})"
            print(f" G1: {res1['total_tokens']}t | G2: {g2_tokens}t [{g2_tag}] | G3: {g3_tokens}t [{g3_tag}]")

            results.append({
                "task_id": tid,
                "function_name": fname,
                "group1_rust_monolith": {
                    "total_tokens": res1["total_tokens"],
                },
                "group2_rust_micro": {
                    "total_tokens": g2_tokens,
                    "pass_at_1": g2_pass1,
                    "final_pass": g2_ok,
                    "attempts": g2_attempts,
                },
                "group3_axiom_hole": {
                    "total_tokens": g3_tokens,
                    "pass_at_1": g3_pass1,
                    "final_pass": g3_ok,
                    "attempts": g3_attempts,
                    "counterexample_encountered": g3_ce_found,
                }
            })

    pct_vs_g2 = round((1.0 - total_g3 / total_g2) * 100, 1) if total_g2 > 0 else 0
    pct_vs_g1 = round((1.0 - total_g3 / total_g1) * 100, 1) if total_g1 > 0 else 0

    print("=" * 70)
    print("📊 评测完成 (基于 Google 官方 Gemini API 真实实测数据)")
    print(f"Group 1 (传统单体大文件长上下文) 总消耗: {total_g1:,} tokens")
    print(f"Group 2 (单任务隔离 Rust 微任务) 总消耗:   {total_g2:,} tokens")
    print(f"Group 3 (Axiom 契约打洞纯表达式) 总消耗:   {total_g3:,} tokens")
    print("-" * 70)
    print(f"🔥 核心指标 1: Axiom 较 Group 2 (单任务隔离微任务) 净节省 Token: {pct_vs_g2}%")
    print(f"🔥 核心指标 2: Axiom 较 Group 1 (传统单体大文件上下文) 净节省 Token: {pct_vs_g1}%")
    if args.mode == "generate":
        print("-" * 70)
        print(f"🛠️ 物理执行验证与自愈统计 (共 {len(eval_tasks)} 个任务):")
        print(f"  • Group 2 (Rust): Pass@1 = {g2_pass1_count}/{len(eval_tasks)} ({round(g2_pass1_count/len(eval_tasks)*100, 1)}%) | 经自愈最终 Pass = {g2_final_pass_count}/{len(eval_tasks)} ({round(g2_final_pass_count/len(eval_tasks)*100, 1)}%)")
        print(f"  • Group 3 (Axiom): Pass@1 = {g3_pass1_count}/{len(eval_tasks)} ({round(g3_pass1_count/len(eval_tasks)*100, 1)}%) | 经SMT自愈最终 Pass = {g3_final_pass_count}/{len(eval_tasks)} ({round(g3_final_pass_count/len(eval_tasks)*100, 1)}%)")
    print("=" * 70)

    output_payload = {
        "version": "2.0",
        "status": "completed_authentic",
        "experiment_metadata": {
            "target_model": active_model,
            "mode": args.mode,
            "evaluated_tasks_count": len(eval_tasks),
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S")
        },
        "totals": {
            "group1_rust_monolith_tokens": total_g1,
            "group2_rust_micro_tokens": total_g2,
            "group3_axiom_hole_tokens": total_g3,
            "net_savings_vs_micro": f"{pct_vs_g2}%",
            "net_savings_vs_monolith": f"{pct_vs_g1}%"
        },
        "detailed_tasks": results
    }

    if args.mode == "generate":
        output_payload["verification_summary"] = {
            "group2_rust": {
                "pass_at_1": f"{g2_pass1_count}/{len(eval_tasks)}",
                "final_pass": f"{g2_final_pass_count}/{len(eval_tasks)}",
            },
            "group3_axiom": {
                "pass_at_1": f"{g3_pass1_count}/{len(eval_tasks)}",
                "final_pass": f"{g3_final_pass_count}/{len(eval_tasks)}",
            }
        }

    os.makedirs(os.path.dirname(OUTPUT_DATA_PATH), exist_ok=True)
    with open(OUTPUT_DATA_PATH, "w", encoding="utf-8") as f:
        json.dump(output_payload, f, indent=2, ensure_ascii=False)
    print(f"💾 真实测量数据已写入持久化文件: {OUTPUT_DATA_PATH}\n")


if __name__ == "__main__":
    main()
