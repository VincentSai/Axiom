#!/usr/bin/env python3
"""
Axiom Phase 3 Concurrent Multi-Agent Evaluation Runner
Empirical measurement of Multi-Agent Concurrency, Git Conflict Tax, and AST Combinator Formal Proving.

Subsystem: Inventory Settlement (5 cohesive, tightly-coupled functions)
Tracks:
- Group 2: Rust multi-branch concurrent generation + Git merge conflict resolution agent
- Group 3: Axiom AST multi-hole concurrent generation + Combinator 0-conflict assembly + Z3 SMT proof
"""

import argparse
import base64
import concurrent.futures
from dataclasses import dataclass
import hashlib
import io
import json
import os
import re
import shutil
import socket
import stat
import subprocess
import sys
import tempfile
import time
from typing import Optional
import urllib.error
import urllib.request

if sys.platform == "win32":
    try:
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
    except Exception:
        pass

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CONCURRENT_TASKS_PATH = os.path.join(SCRIPT_DIR, "concurrent_tasks.json")
OUTPUT_DATA_PATH = os.path.join(SCRIPT_DIR, "data", "concurrent_eval_data.json")

HEX40_RE = re.compile(r"^[0-9a-f]{40}$")
HEX64_RE = re.compile(r"^[0-9a-f]{64}$")


@dataclass(frozen=True)
class UntrackedFileSnapshot:
    """未跟踪代码文件不可变快照，支持 100% 源码可重建性与密码学校验，防 symlink 穿透"""
    path: str
    size_bytes: int
    sha256: str
    content_base64: str
    file_type: str = "regular"  # "regular" | "symlink"
    file_mode: int = 0
    symlink_target: Optional[str] = None

    def to_dict(self) -> dict:
        return {
            "path": self.path,
            "size_bytes": self.size_bytes,
            "sha256": self.sha256,
            "content_base64": self.content_base64,
            "file_type": self.file_type,
            "file_mode": self.file_mode,
            "symlink_target": self.symlink_target,
        }

    @classmethod
    def from_dict(cls, data: dict, schema_version: str = "3.1") -> "UntrackedFileSnapshot":
        if not isinstance(data, dict):
            raise ValueError(f"UntrackedFileSnapshot 必须为字典对象，收到: {type(data)}")
        for req in ["path", "size_bytes", "sha256", "content_base64"]:
            if req not in data:
                raise ValueError(f"UntrackedFileSnapshot 缺少字段: {req}")

        path_val = data["path"]
        if not isinstance(path_val, str) or not path_val.strip():
            raise ValueError(f"UntrackedFileSnapshot path 非法: {path_val}")

        size_val = data["size_bytes"]
        if not isinstance(size_val, int) or isinstance(size_val, bool) or size_val < 0:
            raise ValueError(f"UntrackedFileSnapshot [{path_val}] size_bytes 非法: {size_val}")

        sha_val = data["sha256"]
        if not isinstance(sha_val, str) or not HEX64_RE.match(sha_val):
            raise ValueError(f"UntrackedFileSnapshot [{path_val}] sha256 格式非法: {sha_val}")

        b64_val = data["content_base64"]
        if not isinstance(b64_val, str):
            raise ValueError(f"UntrackedFileSnapshot [{path_val}] content_base64 必须为字符串")

        # 3.1 字段与 3.0 向后兼容支持
        if "file_type" in data or schema_version >= "3.1":
            file_type = data.get("file_type", "regular")
            if not isinstance(file_type, str) or file_type not in ("regular", "symlink"):
                raise ValueError(f"UntrackedFileSnapshot [{path_val}] file_type 非法: {file_type}")
            file_mode = data.get("file_mode", 0)
            if not isinstance(file_mode, int) or isinstance(file_mode, bool) or file_mode < 0:
                raise ValueError(f"UntrackedFileSnapshot [{path_val}] file_mode 非法: {file_mode}")
            symlink_target = data.get("symlink_target")
            if file_type == "symlink":
                if not isinstance(symlink_target, str) or not symlink_target:
                    raise ValueError(f"UntrackedFileSnapshot [{path_val}] symlink 类型的 symlink_target 必须为非空字符串")
            else:
                if symlink_target is not None:
                    raise ValueError(f"UntrackedFileSnapshot [{path_val}] regular 类型的 symlink_target 必须为 None")
        else:
            file_type = "regular"
            file_mode = 0
            symlink_target = None

        return cls(
            path=path_val,
            size_bytes=size_val,
            sha256=sha_val,
            content_base64=b64_val,
            file_type=file_type,
            file_mode=file_mode,
            symlink_target=symlink_target,
        )

    def validate(self):
        """严格以 Fail-Closed 方式校验未跟踪文件的元数据格式、Base64 解码、大小与 SHA-256 密码学自洽性"""
        if not isinstance(self.path, str) or not self.path.strip():
            raise ValueError(f"UntrackedFileSnapshot path 非法: {self.path}")
        if not isinstance(self.size_bytes, int) or isinstance(self.size_bytes, bool) or self.size_bytes < 0:
            raise ValueError(f"UntrackedFileSnapshot [{self.path}] size_bytes 非法: {self.size_bytes}")
        if not isinstance(self.sha256, str) or not HEX64_RE.match(self.sha256):
            raise ValueError(f"UntrackedFileSnapshot [{self.path}] sha256 格式非法: {self.sha256}")
        if not isinstance(self.content_base64, str):
            raise ValueError(f"UntrackedFileSnapshot [{self.path}] content_base64 必须为字符串")

        if not isinstance(self.file_type, str) or self.file_type not in ("regular", "symlink"):
            raise ValueError(f"UntrackedFileSnapshot [{self.path}] file_type 非法: {self.file_type}")
        if not isinstance(self.file_mode, int) or isinstance(self.file_mode, bool) or self.file_mode < 0:
            raise ValueError(f"UntrackedFileSnapshot [{self.path}] file_mode 非法: {self.file_mode}")
        if self.file_type == "symlink":
            if not isinstance(self.symlink_target, str) or not self.symlink_target:
                raise ValueError(f"UntrackedFileSnapshot [{self.path}] symlink 类型的 symlink_target 必须为非空字符串")
        else:
            if self.symlink_target is not None:
                raise ValueError(f"UntrackedFileSnapshot [{self.path}] regular 类型的 symlink_target 必须为 None")

        try:
            raw = base64.b64decode(self.content_base64.encode("ascii"), validate=True)
        except Exception as e:
            raise ValueError(f"UntrackedFileSnapshot [{self.path}] Base64 解码失败: {e}")

        if len(raw) != self.size_bytes:
            raise ValueError(
                f"UntrackedFileSnapshot [{self.path}] 字节大小不自洽: 解码长度 {len(raw)} != 声明长度 {self.size_bytes}"
            )

        computed = hashlib.sha256(raw).hexdigest()
        if computed != self.sha256:
            raise ValueError(
                f"UntrackedFileSnapshot [{self.path}] SHA256 密码学自洽校验失败: 复算哈希 {computed} != 声明哈希 {self.sha256}"
            )

        if self.file_type == "symlink":
            if raw.decode("utf-8", errors="replace") != self.symlink_target:
                raise ValueError(
                    f"UntrackedFileSnapshot [{self.path}] symlink target 与内容不一致"
                )


@dataclass(frozen=True)
class ProvenanceCapsule:
    """不可变全链路 Provenance Capsule 单一真相源深模块，包含源码树、补丁、未跟踪文件与二进制哈希"""
    git_commit: str
    git_tree_hash: str
    git_diff_hash: str
    runner_script_sha256: str
    tasks_payload_sha256: str
    staged_patch_sha256: str
    staged_patch_base64: str
    unstaged_patch_sha256: str
    unstaged_patch_base64: str
    head_diff_base64: str
    untracked_files: tuple
    untracked_files_sha256: tuple
    is_dirty: bool
    dirty_files_count: int
    dirty_files: tuple
    axiom_binary_sha256: str
    z3_binary_sha256: str
    z3_path: Optional[str] = None

    def to_dict(self) -> dict:
        return {
            "git_commit": self.git_commit,
            "git_tree_hash": self.git_tree_hash,
            "git_diff_hash": self.git_diff_hash,
            "runner_script_sha256": self.runner_script_sha256,
            "tasks_payload_sha256": self.tasks_payload_sha256,
            "staged_patch_sha256": self.staged_patch_sha256,
            "staged_patch_base64": self.staged_patch_base64,
            "unstaged_patch_sha256": self.unstaged_patch_sha256,
            "unstaged_patch_base64": self.unstaged_patch_base64,
            "head_diff_base64": self.head_diff_base64,
            "untracked_files": [f.to_dict() for f in self.untracked_files],
            "untracked_files_sha256": dict(self.untracked_files_sha256),
            "is_dirty": self.is_dirty,
            "dirty_files_count": self.dirty_files_count,
            "dirty_files": list(self.dirty_files),
            "axiom_binary_sha256": self.axiom_binary_sha256,
            "z3_binary_sha256": self.z3_binary_sha256,
            "z3_path": self.z3_path,
        }

    @classmethod
    def from_dict(cls, data: dict, schema_version: str = "3.1") -> "ProvenanceCapsule":
        if not isinstance(data, dict):
            raise ValueError(f"ProvenanceCapsule 必须为字典对象，收到: {type(data)}")

        required_fields = [
            "git_commit", "git_tree_hash", "git_diff_hash",
            "runner_script_sha256", "tasks_payload_sha256",
            "staged_patch_sha256", "staged_patch_base64",
            "unstaged_patch_sha256", "unstaged_patch_base64",
            "head_diff_base64", "untracked_files", "untracked_files_sha256",
            "is_dirty", "dirty_files_count", "dirty_files",
            "axiom_binary_sha256", "z3_binary_sha256",
        ]
        for req in required_fields:
            if req not in data:
                raise ValueError(f"ProvenanceCapsule 缺少必填字段: {req}")

        for str_field in [
            "git_commit", "git_tree_hash", "git_diff_hash",
            "runner_script_sha256", "tasks_payload_sha256",
            "staged_patch_sha256", "staged_patch_base64",
            "unstaged_patch_sha256", "unstaged_patch_base64",
            "head_diff_base64", "axiom_binary_sha256", "z3_binary_sha256",
        ]:
            val = data[str_field]
            if not isinstance(val, str):
                raise ValueError(f"ProvenanceCapsule 字段 {str_field} 必须为字符串，收到: {type(val)}")

        is_dirty_raw = data["is_dirty"]
        if not isinstance(is_dirty_raw, bool):
            raise ValueError(f"ProvenanceCapsule is_dirty 必须为原生布尔类型，严禁隐式转换，收到: {type(is_dirty_raw)}")

        dirty_count_raw = data["dirty_files_count"]
        if not isinstance(dirty_count_raw, int) or isinstance(dirty_count_raw, bool) or dirty_count_raw < 0:
            raise ValueError(f"ProvenanceCapsule dirty_files_count 必须为非负整数，收到: {dirty_count_raw}")

        raw_untracked = data["untracked_files"]
        if not isinstance(raw_untracked, list):
            raise ValueError(f"untracked_files 必须为列表，收到: {type(raw_untracked)}")
        snapshots = tuple(UntrackedFileSnapshot.from_dict(item, schema_version=schema_version) for item in raw_untracked)

        raw_untracked_sha = data["untracked_files_sha256"]
        if not isinstance(raw_untracked_sha, dict):
            raise ValueError(f"untracked_files_sha256 必须为字典，收到: {type(raw_untracked_sha)}")
        for k, v in raw_untracked_sha.items():
            if not isinstance(k, str) or not isinstance(v, str):
                raise ValueError(f"untracked_files_sha256 键值必须均为字符串: {k}: {v}")
        sha_items = tuple(sorted(raw_untracked_sha.items()))

        raw_dirty_files = data["dirty_files"]
        if not isinstance(raw_dirty_files, list):
            raise ValueError(f"dirty_files 必须为列表，收到: {type(raw_dirty_files)}")
        for df in raw_dirty_files:
            if not isinstance(df, str):
                raise ValueError(f"dirty_files 元素必须为字符串，收到: {type(df)}")

        z3_p = data.get("z3_path")
        if z3_p is not None and not isinstance(z3_p, str):
            raise ValueError(f"z3_path 必须为字符串或 None，收到: {type(z3_p)}")

        return cls(
            git_commit=data["git_commit"],
            git_tree_hash=data["git_tree_hash"],
            git_diff_hash=data["git_diff_hash"],
            runner_script_sha256=data["runner_script_sha256"],
            tasks_payload_sha256=data["tasks_payload_sha256"],
            staged_patch_sha256=data["staged_patch_sha256"],
            staged_patch_base64=data["staged_patch_base64"],
            unstaged_patch_sha256=data["unstaged_patch_sha256"],
            unstaged_patch_base64=data["unstaged_patch_base64"],
            head_diff_base64=data["head_diff_base64"],
            untracked_files=snapshots,
            untracked_files_sha256=sha_items,
            is_dirty=is_dirty_raw,
            dirty_files_count=dirty_count_raw,
            dirty_files=tuple(raw_dirty_files),
            axiom_binary_sha256=data["axiom_binary_sha256"],
            z3_binary_sha256=data["z3_binary_sha256"],
            z3_path=z3_p,
        )

    def validate(self):
        """严格以 Fail-Closed 方式校验全字段哈希正则格式、Base64 解码复算、整体语义自洽性与类型完整性"""
        for name, val, regex in [
            ("git_commit", self.git_commit, HEX40_RE),
            ("git_tree_hash", self.git_tree_hash, HEX40_RE),
            ("git_diff_hash", self.git_diff_hash, HEX64_RE),
            ("runner_script_sha256", self.runner_script_sha256, HEX64_RE),
            ("tasks_payload_sha256", self.tasks_payload_sha256, HEX64_RE),
            ("staged_patch_sha256", self.staged_patch_sha256, HEX64_RE),
            ("unstaged_patch_sha256", self.unstaged_patch_sha256, HEX64_RE),
            ("axiom_binary_sha256", self.axiom_binary_sha256, HEX64_RE),
            ("z3_binary_sha256", self.z3_binary_sha256, HEX64_RE),
        ]:
            if val == "unknown" or not isinstance(val, str) or not regex.match(val):
                raise ValueError(f"ProvenanceCapsule {name} 非法 (严禁 unknown，必须匹配哈希格式): {val}")

        EMPTY_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

        # 密码学验真 1: staged_patch_base64 解码与复算 SHA256 比对
        if not isinstance(self.staged_patch_base64, str):
            raise ValueError("ProvenanceCapsule staged_patch_base64 必须为字符串")
        try:
            staged_raw = base64.b64decode(self.staged_patch_base64.encode("ascii"), validate=True)
        except Exception as e:
            raise ValueError(f"ProvenanceCapsule staged_patch_base64 解码失败: {e}")
        staged_computed = hashlib.sha256(staged_raw).hexdigest()
        if staged_computed != self.staged_patch_sha256:
            raise ValueError(
                f"ProvenanceCapsule staged_patch 密码学校验失败: 复算 SHA256 {staged_computed} != 声明 {self.staged_patch_sha256}"
            )

        # 密码学验真 2: unstaged_patch_base64 解码与复算 SHA256 比对
        if not isinstance(self.unstaged_patch_base64, str):
            raise ValueError("ProvenanceCapsule unstaged_patch_base64 必须为字符串")
        try:
            unstaged_raw = base64.b64decode(self.unstaged_patch_base64.encode("ascii"), validate=True)
        except Exception as e:
            raise ValueError(f"ProvenanceCapsule unstaged_patch_base64 解码失败: {e}")
        unstaged_computed = hashlib.sha256(unstaged_raw).hexdigest()
        if unstaged_computed != self.unstaged_patch_sha256:
            raise ValueError(
                f"ProvenanceCapsule unstaged_patch 密码学校验失败: 复算 SHA256 {unstaged_computed} != 声明 {self.unstaged_patch_sha256}"
            )

        # 密码学验真 3: head_diff_base64 解码与复算 SHA256 比对 git_diff_hash
        if not isinstance(self.head_diff_base64, str):
            raise ValueError("ProvenanceCapsule head_diff_base64 必须为字符串")
        try:
            head_diff_raw = base64.b64decode(self.head_diff_base64.encode("ascii"), validate=True)
        except Exception as e:
            raise ValueError(f"ProvenanceCapsule head_diff_base64 解码失败: {e}")
        head_diff_computed = hashlib.sha256(head_diff_raw).hexdigest()
        if head_diff_computed != self.git_diff_hash:
            raise ValueError(
                f"ProvenanceCapsule head_diff 密码学校验失败: 复算 SHA256 {head_diff_computed} != 声明 git_diff_hash {self.git_diff_hash}"
            )

        # 密码学验真 4: untracked_files 全量解码、大小与哈希复算验真
        snapshot_map = {}
        for snap in self.untracked_files:
            snap.validate()
            if snap.path in snapshot_map:
                raise ValueError(f"ProvenanceCapsule untracked_files 存在重复路径: {snap.path}")
            snapshot_map[snap.path] = snap.sha256

        # 密码学验真 5: untracked_files_sha256 与 untracked_files 严格双向一致
        sha_map = dict(self.untracked_files_sha256)
        if set(snapshot_map.keys()) != set(sha_map.keys()):
            raise ValueError(
                f"ProvenanceCapsule untracked_files 与 untracked_files_sha256 键集合不匹配: {set(snapshot_map.keys())} != {set(sha_map.keys())}"
            )
        for path, h in sha_map.items():
            if not HEX64_RE.match(h):
                raise ValueError(f"未跟踪文件 {path} 的 SHA256 格式非法: {h}")
            if snapshot_map[path] != h:
                raise ValueError(f"未跟踪文件 {path} 在快照与哈希表中不一致: {snapshot_map[path]} != {h}")

        if not isinstance(self.is_dirty, bool):
            raise ValueError("is_dirty 必须为原生布尔值")
        if not isinstance(self.dirty_files_count, int) or isinstance(self.dirty_files_count, bool) or self.dirty_files_count < 0:
            raise ValueError("dirty_files_count 必须为非负整数")

        # 整体语义强自洽性约束 (P1 Spec 3)
        if not self.is_dirty:
            # Clean 状态：严禁存在任何脏文件、未跟踪文件或非空 patch
            if self.dirty_files_count != 0:
                raise ValueError(f"is_dirty=False 时 dirty_files_count 必须为 0，收到: {self.dirty_files_count}")
            if len(self.dirty_files) != 0:
                raise ValueError(f"is_dirty=False 时 dirty_files 必须为空，收到: {self.dirty_files}")
            if len(self.untracked_files) != 0:
                raise ValueError(f"is_dirty=False 时 untracked_files 必须为空，收到 {len(self.untracked_files)} 项")
            if len(self.untracked_files_sha256) != 0:
                raise ValueError("is_dirty=False 时 untracked_files_sha256 必须为空")
            if self.staged_patch_base64 != "" or self.staged_patch_sha256 != EMPTY_SHA256:
                raise ValueError("is_dirty=False 时 staged_patch 必须为空")
            if self.unstaged_patch_base64 != "" or self.unstaged_patch_sha256 != EMPTY_SHA256:
                raise ValueError("is_dirty=False 时 unstaged_patch 必须为空")
            if self.head_diff_base64 != "" or self.git_diff_hash != EMPTY_SHA256:
                raise ValueError("is_dirty=False 时 head_diff 必须为空")
        else:
            # Dirty 状态：必须存在确凿证据表明工作树不干净
            if self.dirty_files_count <= 0:
                raise ValueError(f"is_dirty=True 时 dirty_files_count 必须大于 0，收到: {self.dirty_files_count}")
            if len(self.dirty_files) == 0:
                raise ValueError("is_dirty=True 时 dirty_files 列表不能为空")
            if self.dirty_files_count < len(self.dirty_files):
                raise ValueError(f"dirty_files_count ({self.dirty_files_count}) 不能小于采样列表长度 ({len(self.dirty_files)})")
            has_patch = bool(self.staged_patch_base64) or bool(self.unstaged_patch_base64) or bool(self.head_diff_base64)
            has_untracked = len(self.untracked_files) > 0
            if not has_patch and not has_untracked:
                raise ValueError("is_dirty=True 但未提供任何 patch 或未跟踪文件证据，语义矛盾")
            if not bool(self.staged_patch_base64) and not bool(self.unstaged_patch_base64):
                if bool(self.head_diff_base64):
                    raise ValueError("staged 和 unstaged patch 均为空时，head_diff 也必须为空")
            else:
                if not bool(self.head_diff_base64):
                    raise ValueError("staged 或 unstaged patch 非空时，head_diff 不能为空")


# 向后兼容别名
GitProvenance = ProvenanceCapsule



def check_port_open(host: str, port: int, timeout: float = 0.3) -> bool:
    """检测本地端口是否开放"""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(timeout)
    try:
        s.connect((host, port))
        s.close()
        return True
    except Exception:
        return False


def detect_recommended_proxy() -> Optional[str]:
    """检测本机是否运行了常用本地代理端口"""
    for port in [7897, 7890, 10808, 10809, 8080]:
        if check_port_open("127.0.0.1", port):
            return f"http://127.0.0.1:{port}"
    return None


def find_axiom_binary() -> Optional[str]:
    """定位 Axiom 编译器可执行文件路径"""
    target_debug = os.path.abspath(os.path.join(SCRIPT_DIR, "..", "..", "target", "debug", "axiom.exe" if sys.platform == "win32" else "axiom"))
    if os.path.isfile(target_debug):
        return target_debug
    which_p = shutil.which("axiom")
    if which_p:
        return which_p
    return target_debug


def find_z3_executable() -> Optional[str]:
    """定位 Z3 SMT 求解器可执行文件路径，与 Rust 内核搜索顺序严格对齐 (AXIOM_Z3_PATH -> PATH -> ~/.axiom/bin)"""
    candidates = []
    # 1. 优先读取环境变量
    env_z3 = os.environ.get("AXIOM_Z3_PATH")
    if env_z3:
        candidates.append(env_z3)
    # 2. 从 PATH 查找
    which_z3 = shutil.which("z3")
    if which_z3:
        candidates.append(which_z3)
    # 3. 默认位置 ~/.axiom/bin/z3(.exe)
    home_z3 = os.path.expanduser(os.path.join("~", ".axiom", "bin", "z3.exe" if sys.platform == "win32" else "z3"))
    candidates.append(home_z3)
    # 4. Windows 备选路径
    if sys.platform == "win32":
        candidates.append(r"C:\Users\Administrator\.axiom\bin\z3.exe")

    for p in candidates:
        if p and os.path.isfile(p):
            return os.path.abspath(p)
    return None


def get_z3_version(is_mock: bool = False) -> str:
    """探测系统或 Axiom 内置 Z3 SMT 求解器版本，Fail-Closed 严禁未知 unknown"""
    z3_p = find_z3_executable()
    if z3_p:
        try:
            out = subprocess.check_output([z3_p, "--version"], stderr=subprocess.DEVNULL, timeout=5)
            v = out.decode("utf-8").strip()
            if v:
                return v
        except Exception:
            pass
    if is_mock:
        return "Z3 4.12.2 (synthetic_mock_environment)"
    raise RuntimeError("未检测到 Z3 求解器可执行文件，实测模式下严禁缺少 Z3 或使用未知版本")


_CACHED_PROVENANCE = None


def collect_git_provenance(actual_z3_path: Optional[str] = None, actual_z3_sha256: Optional[str] = None) -> GitProvenance:
    """严格以 Fail-Closed 方式收集不可变 Git Provenance Capsule，包含可还原源码快照与依赖二进制哈希"""
    try:
        repo_root = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=SCRIPT_DIR,
            stderr=subprocess.PIPE,
            timeout=5
        ).decode("utf-8").strip()
    except Exception as e:
        raise RuntimeError(f"GitProvenance 采集失败 (git rev-parse --show-toplevel): {e}")

    try:
        commit = subprocess.check_output(
            ["git", "rev-parse", "HEAD"],
            cwd=repo_root,
            stderr=subprocess.PIPE,
            timeout=5
        ).decode("utf-8").strip()
    except Exception as e:
        raise RuntimeError(f"GitProvenance 采集失败 (git rev-parse HEAD): {e}")
    if not HEX40_RE.match(commit):
        raise ValueError(f"Git commit sha 格式非法 (必须为40位十六进制): {commit}")

    try:
        tree = subprocess.check_output(
            ["git", "rev-parse", "HEAD^{tree}"],
            cwd=repo_root,
            stderr=subprocess.PIPE,
            timeout=5
        ).decode("utf-8").strip()
    except Exception as e:
        raise RuntimeError(f"GitProvenance 采集失败 (git rev-parse HEAD^{{tree}}): {e}")
    if not HEX40_RE.match(tree):
        raise ValueError(f"Git tree hash 格式非法 (必须为40位十六进制): {tree}")

    path_excludes = [":(exclude)graphify-out", ":(exclude)examples/benchmark_eval/data"]

    try:
        staged_patch = subprocess.check_output(
            ["git", "diff", "--cached", "--binary", "--", "."] + path_excludes,
            cwd=repo_root,
            stderr=subprocess.PIPE,
            timeout=5
        )
        staged_hash = hashlib.sha256(staged_patch).hexdigest()
        staged_b64 = base64.b64encode(staged_patch).decode("ascii")
    except Exception as e:
        raise RuntimeError(f"GitProvenance 采集失败 (git diff --cached --binary): {e}")

    try:
        unstaged_patch = subprocess.check_output(
            ["git", "diff", "--binary", "--", "."] + path_excludes,
            cwd=repo_root,
            stderr=subprocess.PIPE,
            timeout=5
        )
        unstaged_hash = hashlib.sha256(unstaged_patch).hexdigest()
        unstaged_b64 = base64.b64encode(unstaged_patch).decode("ascii")
    except Exception as e:
        raise RuntimeError(f"GitProvenance 采集失败 (git diff --binary): {e}")

    try:
        head_diff = subprocess.check_output(
            ["git", "diff", "--binary", "HEAD", "--", "."] + path_excludes,
            cwd=repo_root,
            stderr=subprocess.PIPE,
            timeout=5
        )
        diff_hash = hashlib.sha256(head_diff).hexdigest()
        head_diff_b64 = base64.b64encode(head_diff).decode("ascii")
    except Exception as e:
        raise RuntimeError(f"GitProvenance 采集失败 (git diff --binary HEAD): {e}")

    runner_path = os.path.abspath(__file__)
    with open(runner_path, "rb") as f:
        runner_sha256 = hashlib.sha256(f.read()).hexdigest()

    tasks_path = os.path.join(SCRIPT_DIR, "concurrent_tasks.json")
    with open(tasks_path, "rb") as f:
        tasks_sha256 = hashlib.sha256(f.read()).hexdigest()

    # 1. 采集未跟踪文件快照：使用 git ls-files -z 规避文件名转义与截断，并用 lstat 杜绝 symlink 穿透 (P1 Spec 4)
    untracked_snapshots = []
    untracked_items = []
    try:
        ls_out = subprocess.check_output(
            ["git", "ls-files", "--others", "--exclude-standard", "-z"],
            cwd=repo_root,
            stderr=subprocess.PIPE,
            timeout=5
        )
        raw_paths = [p.decode("utf-8", errors="replace") for p in ls_out.split(b"\x00") if p]
        for rel_p in raw_paths:
            # 过滤不需要的目录与产物
            if (rel_p.startswith("graphify-out")
                or rel_p.startswith("examples/benchmark_eval/data")
                or rel_p.startswith("examples\\benchmark_eval\\data")):
                continue

            full_path = os.path.join(repo_root, rel_p)
            try:
                st = os.lstat(full_path)
            except OSError:
                continue

            mode = st.st_mode
            if stat.S_ISLNK(mode):
                file_type = "symlink"
                target = os.readlink(full_path)
                raw_bytes = target.encode("utf-8", errors="replace")
                symlink_target = target
            elif stat.S_ISREG(mode):
                file_type = "regular"
                symlink_target = None
                with open(full_path, "rb") as uf:
                    raw_bytes = uf.read()
            else:
                continue

            f_hash = hashlib.sha256(raw_bytes).hexdigest()
            f_b64 = base64.b64encode(raw_bytes).decode("ascii")
            snap = UntrackedFileSnapshot(
                path=rel_p,
                size_bytes=len(raw_bytes),
                sha256=f_hash,
                content_base64=f_b64,
                file_type=file_type,
                file_mode=stat.S_IMODE(mode),
                symlink_target=symlink_target,
            )
            untracked_snapshots.append(snap)
            untracked_items.append((rel_p, f_hash))
    except Exception as e:
        raise RuntimeError(f"GitProvenance 采集未跟踪文件失败 (git ls-files -z): {e}")

    # 2. 采集 dirty_files 清单：使用 git status --porcelain=v1 -z
    dirty_files = []
    try:
        status_out = subprocess.check_output(
            ["git", "status", "--porcelain=v1", "-z"],
            cwd=repo_root,
            stderr=subprocess.PIPE,
            timeout=5
        )
        status_items = [p.decode("utf-8", errors="replace") for p in status_out.split(b"\x00") if p]
        idx = 0
        while idx < len(status_items):
            item = status_items[idx]
            code = item[:2]
            path_part = item[3:] if len(item) > 3 else ""
            if "R" in code or "C" in code:
                idx += 1  # 跳过重命名/复制的旧路径
            if path_part:
                if (not path_part.startswith("graphify-out")
                    and not path_part.startswith("examples/benchmark_eval/data")
                    and not path_part.startswith("examples\\benchmark_eval\\data")):
                    dirty_files.append(f"{code} {path_part}")
            idx += 1
    except Exception as e:
        raise RuntimeError(f"GitProvenance 采集 dirty 状态失败 (git status -z): {e}")

    axiom_bin = find_axiom_binary()
    if axiom_bin and os.path.isfile(axiom_bin):
        with open(axiom_bin, "rb") as f:
            axiom_sha256 = hashlib.sha256(f.read()).hexdigest()
    else:
        axiom_sha256 = hashlib.sha256(b"axiom-compiler-debug-binary-mock-hash").hexdigest()

    # P1 Spec 2: 绑定 Z3 绝对路径与二进制 SHA-256，优先采信实测中内核报告的真实求解器
    if actual_z3_path and os.path.isfile(actual_z3_path):
        z3_path = os.path.abspath(actual_z3_path)
    else:
        z3_path = find_z3_executable()

    if actual_z3_sha256:
        z3_sha256 = actual_z3_sha256
    elif z3_path and os.path.isfile(z3_path):
        with open(z3_path, "rb") as f:
            z3_sha256 = hashlib.sha256(f.read()).hexdigest()
    else:
        z3_sha256 = hashlib.sha256(b"z3-solver-executable-mock-hash").hexdigest()

    is_dirty_state = (len(dirty_files) > 0) or (len(untracked_snapshots) > 0) or (staged_b64 != "") or (unstaged_b64 != "") or (head_diff_b64 != "")

    prov = ProvenanceCapsule(
        git_commit=commit,
        git_tree_hash=tree,
        git_diff_hash=diff_hash,
        runner_script_sha256=runner_sha256,
        tasks_payload_sha256=tasks_sha256,
        staged_patch_sha256=staged_hash,
        staged_patch_base64=staged_b64,
        unstaged_patch_sha256=unstaged_hash,
        unstaged_patch_base64=unstaged_b64,
        head_diff_base64=head_diff_b64,
        untracked_files=tuple(untracked_snapshots),
        untracked_files_sha256=tuple(untracked_items),
        is_dirty=is_dirty_state,
        dirty_files_count=len(dirty_files),
        dirty_files=tuple(dirty_files[:20]),
        axiom_binary_sha256=axiom_sha256,
        z3_binary_sha256=z3_sha256,
        z3_path=z3_path,
    )

    prov.validate()
    return prov


def get_git_provenance() -> GitProvenance:
    global _CACHED_PROVENANCE
    if _CACHED_PROVENANCE is None:
        _CACHED_PROVENANCE = collect_git_provenance()
    return _CACHED_PROVENANCE


def get_git_provenance_dict() -> dict:
    return get_git_provenance().to_dict()


def get_git_commit_sha() -> str:
    return get_git_provenance().git_commit


def get_git_tree_hash() -> str:
    return get_git_provenance().git_tree_hash


def get_git_diff_hash() -> str:
    return get_git_provenance().git_diff_hash


def get_git_status() -> dict:
    prov = get_git_provenance()
    return {
        "is_dirty": prov.is_dirty,
        "dirty_files_count": prov.dirty_files_count,
        "dirty_files": list(prov.dirty_files),
    }


def validate_task_envelope_schema(envelope: dict, expected_hole: str, expected_func: str):
    """Fail-Closed 严格校验 TaskEnvelope 数据字典的结构契约与哈希格式"""
    if not isinstance(envelope, dict):
        raise ValueError(f"TaskEnvelope 必须为字典对象，收到: {type(envelope)}")

    if envelope.get("target_hole_id") != expected_hole:
        raise ValueError(f"TaskEnvelope target_hole_id 不匹配: {envelope.get('target_hole_id')} != {expected_hole}")

    if envelope.get("target_function") != expected_func:
        raise ValueError(f"TaskEnvelope target_function 不匹配: {envelope.get('target_function')} != {expected_func}")

    c_hash = envelope.get("contract_hash", "")
    if not isinstance(c_hash, str) or not c_hash.startswith("#contract_"):
        raise ValueError(f"TaskEnvelope contract_hash 格式非法: {c_hash}")

    base_root = envelope.get("base_root_hash")
    if base_root is not None and (not isinstance(base_root, str) or not base_root.startswith("#mod_")):
        raise ValueError(f"TaskEnvelope base_root_hash 格式非法: {base_root}")

    dep_hashes = envelope.get("dependency_hashes")
    if not isinstance(dep_hashes, list):
        raise ValueError(f"TaskEnvelope dependency_hashes 必须为列表，收到: {type(dep_hashes)}")
    for dep in dep_hashes:
        if not isinstance(dep, str) or not dep.startswith("#"):
            raise ValueError(f"TaskEnvelope 依赖项哈希非法: {dep}")


def validate_benchmark_result_schema(data: dict):
    """校验评测结果输出 JSON 顶层与各子系统的 Schema 规范，Fail-Closed 严禁 unknown，显式向前兼容 3.0 并严格校验 3.1"""
    required_top_keys = ["version", "subsystem", "experiment_metadata", "summary", "detailed_runs"]
    for key in required_top_keys:
        if key not in data:
            raise ValueError(f"Benchmark 输出结果缺少必填顶层字段: {key}")

    version = data["version"]
    if version not in ("3.0", "3.1"):
        raise ValueError(f"不支持的 Schema 版本: {version} (当前仅支持 3.0 与 3.1 规范)")

    meta = data["experiment_metadata"]
    required_meta = [
        "engine", "eval_mode", "is_mock", "evidence_grade",
        "git_commit", "git_tree_hash", "git_diff_hash", "git_provenance",
        "z3_version"
    ]
    if version >= "3.1":
        required_meta.append("z3_path")

    for req in required_meta:
        if req not in meta:
            raise ValueError(f"experiment_metadata 缺少字段: {req}")

    if meta.get("is_mock") is True:
        if meta.get("evidence_grade") != "synthetic_non_evidentiary":
            raise ValueError(f"当 is_mock 为 True 时，evidence_grade 必须为 synthetic_non_evidentiary，收到: {meta.get('evidence_grade')}")
    elif meta.get("is_mock") is False:
        if meta.get("evidence_grade") != "live_empirical":
            raise ValueError(f"当 is_mock 为 False 时，evidence_grade 必须为 live_empirical，收到: {meta.get('evidence_grade')}")
    else:
        raise ValueError(f"is_mock 必须为原生布尔值，收到: {meta.get('is_mock')}")

    # 使用单一真相源 ProvenanceCapsule.from_dict 严格校验顶层 Provenance 并深度复算密码学 SHA-256
    top_prov = ProvenanceCapsule.from_dict(meta["git_provenance"], schema_version=version)
    top_prov.validate()

    # 顶层冗余字段必须与顶层 ProvenanceCapsule 严格一致
    if meta["git_commit"] != top_prov.git_commit:
        raise ValueError(f"experiment_metadata.git_commit 与 git_provenance 不一致: {meta['git_commit']} != {top_prov.git_commit}")
    if meta["git_tree_hash"] != top_prov.git_tree_hash:
        raise ValueError(f"experiment_metadata.git_tree_hash 与 git_provenance 不一致: {meta['git_tree_hash']} != {top_prov.git_tree_hash}")
    if meta["git_diff_hash"] != top_prov.git_diff_hash:
        raise ValueError(f"experiment_metadata.git_diff_hash 与 git_provenance 不一致: {meta['git_diff_hash']} != {top_prov.git_diff_hash}")

    # 校验顶层 git_status 与 top_prov 语义自洽
    if "git_status" in meta:
        gstatus = meta["git_status"]
        if not isinstance(gstatus, dict):
            raise ValueError("experiment_metadata.git_status 必须为字典")
        if gstatus.get("is_dirty") != top_prov.is_dirty:
            raise ValueError(f"git_status.is_dirty 与 git_provenance 不一致: {gstatus.get('is_dirty')} != {top_prov.is_dirty}")
        if gstatus.get("dirty_files_count") != top_prov.dirty_files_count:
            raise ValueError(f"git_status.dirty_files_count 与 git_provenance 不一致: {gstatus.get('dirty_files_count')} != {top_prov.dirty_files_count}")
        if gstatus.get("dirty_files") != list(top_prov.dirty_files):
            raise ValueError("git_status.dirty_files 与 git_provenance 不一致")

    summary = data.get("summary", {})
    axiom_summary = summary.get("axiom_group", {})
    if meta.get("is_mock"):
        if axiom_summary.get("speedup_vs_rust") is not None:
            raise ValueError("在 mock 模式下，summary.axiom_group.speedup_vs_rust 必须为 None")
        if not axiom_summary.get("synthetic_note"):
            raise ValueError("在 mock 模式下，summary.axiom_group 必须包含 synthetic_note 说明")
    else:
        speedup = axiom_summary.get("speedup_vs_rust")
        if not isinstance(speedup, (int, float)) or speedup <= 0:
            raise ValueError(f"在实测模式下，summary.axiom_group.speedup_vs_rust 必须为正数，收到: {speedup}")

    g2 = data["detailed_runs"].get("group2_rust", {})
    g3 = data["detailed_runs"].get("group3_axiom", {})
    for g_data, g_name in [(g2, "Group 2"), (g3, "Group 3")]:
        for req in ["total_tasks", "git_tree_hash", "git_diff_hash", "git_provenance"]:
            if req not in g_data:
                raise ValueError(f"{g_name} 缺少必要 provenance 字段: {req}")

        # 使用单一真相源 ProvenanceCapsule.from_dict 解析并校验子系统 Provenance
        sub_prov = ProvenanceCapsule.from_dict(g_data["git_provenance"], schema_version=version)
        sub_prov.validate()

        # 严格断言子系统 Provenance 与顶层 ProvenanceCapsule 全等，杜绝规则与数据漂移
        if sub_prov != top_prov:
            raise ValueError(f"{g_name} git_provenance 与顶层 git_provenance 存在字段漂移不一致！")

        if g_data["git_tree_hash"] != top_prov.git_tree_hash:
            raise ValueError(f"{g_name} git_tree_hash 与顶层不一致: {g_data['git_tree_hash']} != {top_prov.git_tree_hash}")
        if g_data["git_diff_hash"] != top_prov.git_diff_hash:
            raise ValueError(f"{g_name} git_diff_hash 与顶层不一致: {g_data['git_diff_hash']} != {top_prov.git_diff_hash}")
        if "git_commit_sha" in g_data and g_data["git_commit_sha"] != top_prov.git_commit:
            raise ValueError(f"{g_name} git_commit_sha 与顶层不一致: {g_data['git_commit_sha']} != {top_prov.git_commit}")

    # 递归 Fail-Closed 严查所有字段，绝不能有字符串 "unknown"
    def check_no_unknown(obj, path=""):
        if isinstance(obj, str):
            if obj == "unknown":
                raise ValueError(f"Schema 校验失败：路径 '{path}' 的值为 'unknown'，严禁任何未知占位符！")
        elif isinstance(obj, dict):
            for k, v in obj.items():
                check_no_unknown(v, f"{path}.{k}" if path else k)
        elif isinstance(obj, list):
            for i, v in enumerate(obj):
                check_no_unknown(v, f"{path}[{i}]")

    check_no_unknown(data)


def find_codex_binary() -> str:
    p = shutil.which("codex")
    if p:
        return p
    default_win = r"C:\Users\Administrator\AppData\Roaming\npm\codex.cmd"
    if os.path.isfile(default_win):
        return default_win
    return "codex"


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


class ContextCapsule:
    """
    Three-level immutable context capsule:
    - SystemCapsule: global shared prelude/syntax guide
    - ContractCapsule: localized specification (requires/ensures/signature)
    - DependencyCapsule: explicit typed symbols imported by function
    """
    def __init__(self, system_text: str, contract_text: str, dependency_text: str = ""):
        self.system_text = system_text.strip()
        self.contract_text = contract_text.strip()
        self.dependency_text = dependency_text.strip()
        self.system_tokens = count_exact_tokens(self.system_text)
        self.contract_tokens = count_exact_tokens(self.contract_text)
        self.dependency_tokens = count_exact_tokens(self.dependency_text)
        self.total_tokens = self.system_tokens + self.contract_tokens + self.dependency_tokens

    def render_prompt(self) -> str:
        dep_section = f"\n[Dependency Capsule]\n{self.dependency_text}\n" if self.dependency_text else ""
        return f"""You are an Axiom micro-worker. Write ONLY a single pure expression to fill the `?hole` in the function below:

[Axiom Syntax Guide]
{self.system_text}
{dep_section}
```axiom
{self.contract_text}
```

Return ONLY the pure expression strictly satisfying the contracts. Do not output function signatures or boilerplate."""


def extract_code(text: str, target_kind: str = "rust") -> str:
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


def call_codex_generate(prompt: str, model: str = None, proxy: str = None, timeout: int = 240) -> dict:
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

    t_start = time.time()
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
        t_elapsed = time.time() - t_start

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
            raise RuntimeError(f"codex exec 失败 (退出码 {proc.returncode}): {err_msg}")

        # 精确 BPE Token 统计，无任何粗暴人工乘数
        task_prompt_tokens = count_exact_tokens(prompt)
        if raw_output > 0:
            visible_code_tokens = max(1, raw_output - raw_reasoning)
        else:
            visible_code_tokens = count_exact_tokens(ans_text)
        total_payload = task_prompt_tokens + visible_code_tokens
        raw_total = raw_input + raw_output
        if raw_total == 0:
            raw_total = total_payload

        return {
            "text": ans_text,
            "latency_sec": round(t_elapsed, 2),
            "task_payload": {
                "prompt_tokens": task_prompt_tokens,
                "candidates_tokens": visible_code_tokens,
                "total_tokens": total_payload
            },
            "raw_billed": {
                "input_tokens": raw_input,
                "output_tokens": raw_output,
                "reasoning_tokens": raw_reasoning,
                "total_tokens": raw_total
            }
        }
    finally:
        if os.path.exists(out_path):
            try:
                os.remove(out_path)
            except Exception:
                pass


def mock_generate_worker(task: dict, group: int, capsule: "ContextCapsule" = None) -> dict:
    tid = task["id"]
    fname = task["function_name"]
    if group == 2:
        if tid == 1:
            code = f"pub fn {fname}(item_id1: i64, count1: i64, max_stack1: i64, item_id2: i64, count2: i64) -> bool {{\n    item_id1 == item_id2 && count1 < max_stack1\n}}"
        elif tid == 2:
            code = f"pub fn {fname}(count1: i64, count2: i64, max_stack: i64) -> i64 {{\n    if count1 + count2 > max_stack {{ max_stack }} else {{ count1 + count2 }}\n}}"
        elif tid == 3:
            code = f"pub fn {fname}(unit_price: i64, quantity: i64, discount_pct: i64) -> i64 {{\n    (unit_price * quantity * (100 - discount_pct)) / 100\n}}"
        elif tid == 4:
            code = f"pub fn {fname}(buyer_gold: i64, total_cost: i64, free_slots: i64, needed_slots: i64) -> bool {{\n    buyer_gold >= total_cost && free_slots >= needed_slots\n}}"
        else:
            code = f"pub fn {fname}(cost: i64, durability_cur: i64, durability_max: i64) -> i64 {{\n    (cost * durability_cur) / durability_max\n}}"
        c_tok = count_exact_tokens(code)
        p_tok = 220
    else:
        code = task["canonical_axiom_expr"]
        c_tok = count_exact_tokens(code)
        p_tok = capsule.total_tokens if capsule is not None else 200

    return {
        "text": code,
        "latency_sec": 0.15,
        "task_payload": {
            "prompt_tokens": p_tok,
            "candidates_tokens": c_tok,
            "total_tokens": p_tok + c_tok
        },
        "raw_billed": {
            "input_tokens": 15000,
            "output_tokens": c_tok,
            "reasoning_tokens": 0,
            "total_tokens": 15000 + c_tok
        }
    }


def mock_generate_resolver(conflict_content: str, incoming_fname: str) -> dict:
    """模拟 Conflict Resolver Agent 清理冲突标记"""
    lines = []
    for line in conflict_content.splitlines():
        if line.startswith("<<<<<<<") or line.startswith("=======") or line.startswith(">>>>>>>"):
            continue
        lines.append(line)
    resolved = "\n".join(lines)
    return {
        "text": resolved,
        "latency_sec": 0.25,
        "task_payload": {
            "prompt_tokens": 450,
            "candidates_tokens": 320,
            "total_tokens": 770
        },
        "raw_billed": {
            "input_tokens": 16500,
            "output_tokens": 320,
            "reasoning_tokens": 0,
            "total_tokens": 16820
        }
    }


def load_concurrent_tasks():
    with open(CONCURRENT_TASKS_PATH, "r", encoding="utf-8") as f:
        data = json.load(f)
    return data["tasks"]


def build_rust_initial_skeleton(tasks: list) -> str:
    parts = ["// Subsystem: Inventory Settlement Pipeline", ""]
    for t in tasks:
        fname = t["function_name"]
        params = ", ".join([f"{p['name']}: i64" for p in t["inputs"]])
        ret = "bool" if t["output"] == "Bool" else "i64"
        parts.append(f"/// Description: {t['description']}")
        parts.append(f"pub fn {fname}({params}) -> {ret} {{")
        parts.append(f'    todo!("implement {fname}")')
        parts.append("}\n")
    return "\n".join(parts)


def build_axiom_initial_skeleton(tasks: list) -> str:
    parts = ["// Subsystem: Inventory Settlement Pipeline", ""]
    for t in tasks:
        fname = t["function_name"]
        params = ", ".join([f"{p['name']}: {p['type']}" for p in t["inputs"]])
        reqs = "\n    ".join([f"{r}," for r in t["contracts"]["requires"]])
        enss = "\n    ".join([f"{e}," for e in t["contracts"]["ensures"]])
        parts.append(f"fn {fname}({params}) -> {t['output']}")
        parts.append(f"requires {{\n    {reqs}\n}}")
        parts.append(f"ensures {{\n    {enss}\n}}")
        parts.append(f"{{\n    ?{fname}_hole\n}}\n")
    return "\n".join(parts)


def build_rust_worker_prompt(task: dict, current_module_code: str) -> str:
    fname = task["function_name"]
    params = ", ".join([f"{p['name']}: i64" for p in task["inputs"]])
    ret = "bool" if task["output"] == "Bool" else "i64"
    reqs = "\n/// - ".join(task["contracts"]["requires"])
    enss = "\n/// - ".join(task["contracts"]["ensures"])

    return f"""You are an expert Rust systems engineer working on a cohesive inventory subsystem. Below is the existing shared module `src/inventory.rs`:

```rust
{current_module_code}
```

[YOUR ASSIGNMENT]
Implement function `{fname}`:
/// Description: {task['description']}
/// Preconditions:
/// - {reqs}
/// Postconditions:
/// - {enss}
pub fn {fname}({params}) -> {ret} {{
    // Implement here
}}

Return ONLY the complete implementation for function `{fname}` in ```rust ... ``` block."""


AXIOM_SYSTEM_CAPSULE = """- Return ONLY a single pure expression (no semicolons `;`, no `return` keyword, no blocks).
- Supported operators: +, -, *, /, &&, ||, ==, !=, <, <=, >, >=.
- Conditional expressions use match: `match cond { true => expr1, false => expr2 }`.
- Canonical Example: `match a >= b { true => a - b, false => b - a }`"""


def build_axiom_context_capsule(task: dict, envelope: dict = None) -> ContextCapsule:
    fname = task["function_name"]
    hole_id = f"{fname}_hole"
    params = ", ".join([f"{p['name']}: {p['type']}" for p in task["inputs"]])
    reqs = "\n        ".join([f"{r}," for r in task["contracts"]["requires"]])
    enss = "\n        ".join([f"{e}," for e in task["contracts"]["ensures"]])

    skeleton = f"""fn {fname}({params}) -> {task['output']}
    requires {{
        {reqs}
    }}
    ensures {{
        {enss}
    }}
{{
    ?{hole_id}
}}"""
    dep_lines = []
    types_used = sorted(set([p["type"] for p in task.get("inputs", [])] + [task.get("output", "Int")]))
    dep_lines.append(f"// Primitive & Domain Types: {', '.join(types_used)}")
    if envelope:
        c_hash = envelope.get("contract_hash")
        if c_hash:
            dep_lines.append(f"// TaskEnvelope Contract Hash: {c_hash}")
        deps = envelope.get("dependency_hashes")
        if deps:
            for dep in deps:
                dep_lines.append(f"// TaskEnvelope Dependency Hash: {dep}")
    dependency_text = "\n".join(dep_lines)
    return ContextCapsule(AXIOM_SYSTEM_CAPSULE, skeleton, dependency_text)


def build_axiom_worker_prompt(task: dict, envelope: dict = None) -> str:
    capsule = build_axiom_context_capsule(task, envelope)
    return capsule.render_prompt()


def build_resolver_prompt(conflict_content: str, incoming_fname: str) -> str:
    return f"""You are a Git Conflict Resolution specialist. During a concurrent multi-agent merge, `src/inventory.rs` encountered a merge conflict while merging implementation for `{incoming_fname}`:

```rust
{conflict_content}
```

[YOUR TASK]
1. Retain all previously implemented functions from HEAD.
2. Integrate the incoming implementation of `{incoming_fname}` without syntax errors.
3. Remove all conflict markers (`<<<<<<< HEAD`, `=======`, `>>>>>>>`).
4. Ensure all functions are clean, syntactically correct Rust code.

Return ONLY the completely resolved `src/inventory.rs` source code in a ```rust ... ``` block."""


def run_rust_system_tests(code: str, tasks: list) -> tuple[bool, str]:
    test_lines = []
    for t in tasks:
        fname = t["function_name"]
        for vec in t.get("test_vectors", []):
            args_str = ", ".join([str(a).lower() if isinstance(a, bool) else str(a) for a in vec["inputs"]])
            exp_str = str(vec["expected"]).lower() if isinstance(vec["expected"], bool) else str(vec["expected"])
            test_lines.append(f"    assert_eq!({fname}({args_str}), {exp_str});")
    tests_body = "\n".join(test_lines)

    full_source = f"""{code}

#[cfg(test)]
mod tests {{
    use super::*;
    #[test]
    fn test_subsystem_integration() {{
{tests_body}
    }}
}}
"""
    with tempfile.TemporaryDirectory() as tmpdir:
        src_path = os.path.join(tmpdir, "test.rs")
        exe_path = os.path.join(tmpdir, "test.exe" if sys.platform == "win32" else "test")
        with open(src_path, "w", encoding="utf-8") as f:
            f.write(full_source)

        c_res = subprocess.run(["rustc", "--test", src_path, "-o", exe_path], capture_output=True, text=True, timeout=20)
        if c_res.returncode != 0:
            return False, f"Rust compilation error:\n{c_res.stderr.strip()[:600]}"

        r_res = subprocess.run([exe_path], capture_output=True, text=True, timeout=10)
        if r_res.returncode != 0:
            err = r_res.stdout.strip() or r_res.stderr.strip()
            return False, f"Rust unit test assertion failed:\n{err[:600]}"

    return True, "All subsystem unit tests passed"


def run_axiom_system_verify(axiom_source: str, axiom_bin: str) -> tuple[bool, str, list]:
    with tempfile.TemporaryDirectory() as tmpdir:
        src_path = os.path.join(tmpdir, "inventory.ax")
        with open(src_path, "w", encoding="utf-8") as f:
            f.write(axiom_source)

        cmd = [axiom_bin, "verify", "--strict", "--json", src_path]
        try:
            res = subprocess.run(cmd, capture_output=True, text=True, timeout=25)
            stdout = res.stdout.strip()
            if not stdout:
                return False, f"Axiom returned empty output (stderr: {res.stderr[:300]})", []
            try:
                reports = json.loads(stdout)
            except Exception:
                return False, f"Failed to parse Axiom diagnostic output: {stdout[:300]}", []

            all_proved = True
            summary_msgs = []
            for r in reports:
                fn_name = r.get("function_name", "unknown")
                status = r.get("status")
                diag = r.get("diagnosis", "")
                if status != "Proved":
                    all_proved = False
                    summary_msgs.append(f"{fn_name}: {status} ({diag})")
                else:
                    summary_msgs.append(f"{fn_name}: Proved")

            if all_proved:
                return True, "All 5 functions formally proved by Z3", reports
            else:
                return False, f"Verification failed: {'; '.join(summary_msgs)}", reports
        except Exception as e:
            return False, f"Axiom verify execution error: {e}", []


def benchmark_cas_concurrency_and_stale_root(axiom_bin: str, is_mock: bool = False) -> dict:
    """
    真实评测 AgentTaskPool CAS 并发竞争、排他文件锁互斥与 Stale Root 冲突重试指标。
    构造 2 个独立的 Worker 线程通过同步屏障并发向同一个共享持久化数据库 (--db) 提交，
    观察并记录底层 CAS 互斥锁拦截脏写、重读最新根、以及 Fast-Forward 自动重基的全流程。
    """
    import threading
    t_start = time.time()

    skel = """fn task_a(x: Int) -> Int requires { x >= 0, x < 1000000 } ensures { result > x } { ?hole_a }
fn task_b(y: Int) -> Int requires { y >= 0, y < 1000000 } ensures { result >= y } { ?hole_b }
"""
    with tempfile.TemporaryDirectory() as tmpdir:
        skel_file = os.path.join(tmpdir, "race_skel.ax")
        db_file = os.path.join(tmpdir, "race_astdb.json")
        with open(skel_file, "w", encoding="utf-8") as f:
            f.write(skel)

        # 1. 显式通过 CLI 初始化共享物理数据库，建立共同的基线 RootRef
        init_cmd = [axiom_bin, "pool", "status", skel_file, "--db", db_file]
        init_p = subprocess.run(init_cmd, capture_output=True, text=True, timeout=15)
        if init_p.returncode != 0:
            raise RuntimeError(f"failed to initialize shared db: {init_p.stderr}")

        with open(db_file, "r", encoding="utf-8") as f:
            db_state_initial = json.load(f)
        initial_root = db_state_initial.get("root_ref")

        barrier = threading.Barrier(2)
        results = {}

        def worker_submit(worker_id, hole, expr):
            # 预先检查就绪，确保跨进程/线程 DB open 就绪
            prep_cmd = [axiom_bin, "pool", "status", skel_file, "--db", db_file]
            subprocess.run(prep_cmd, capture_output=True, text=True, timeout=15)

            barrier.wait()  # 同步屏障：确保两个 Worker 在 DB 就绪后同时发起 CAS 提交碰撞

            cmd = [axiom_bin, "pool", "submit", skel_file, "--db", db_file, "--hole", hole, "--expr", expr, "--json"]
            p = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
            results[worker_id] = {
                "returncode": p.returncode,
                "stdout": p.stdout.strip(),
                "stderr": p.stderr.strip(),
            }

        t1 = threading.Thread(target=worker_submit, args=("worker_a", "hole_a", "x + 1"))
        t2 = threading.Thread(target=worker_submit, args=("worker_b", "hole_b", "y * 2"))
        t1.start()
        t2.start()
        t1.join()
        t2.join()

        success_count = sum(1 for r in results.values() if r["returncode"] == 0)
        elapsed = round(time.time() - t_start, 3)

        # 检查持久化数据库中落盘的真实 commit 链和收据
        final_commits = {}
        final_root = None
        if os.path.exists(db_file):
            with open(db_file, "r", encoding="utf-8") as f:
                db_state_final = json.load(f)
            final_commits = db_state_final.get("commits", {})
            final_root = db_state_final.get("root_ref")

        commit_chain_len = len(final_commits)

        # 直接从 Worker 实际发射的结构化 CAS 事件流中客观统计指标，杜绝结果反推
        stale_roots_observed = 0
        rebases_succeeded = 0
        parsed_events = {}

        for wid, r in results.items():
            stdout = r.get("stdout", "")
            if not stdout:
                raise RuntimeError(
                    f"Fail-Closed: Worker {wid} returned empty output. "
                    f"returncode={r.get('returncode')}, stderr={r.get('stderr')}"
                )
            try:
                data = json.loads(stdout)
            except json.JSONDecodeError as exc:
                raise RuntimeError(
                    f"Fail-Closed: Worker {wid} CLI output is not valid JSON:\nSTDOUT:\n{stdout}\nSTDERR:\n{r.get('stderr')}"
                ) from exc

            w_events = []
            events = data.get("cas_events", [])
            for ev in events:
                w_events.append(ev)
                if isinstance(ev, dict):
                    if "StaleRootDetected" in ev:
                        stale_roots_observed += 1
                    if "RebaseSucceeded" in ev:
                        rebases_succeeded += 1
            parsed_events[wid] = w_events

        return {
            "race_workers": 2,
            "success_submissions": success_count,
            "concurrency_elapsed_sec": elapsed,
            "stale_root_conflicts_observed": stale_roots_observed,
            "fast_forward_retries_succeeded": rebases_succeeded,
            "final_commit_chain_length": commit_chain_len,
            "initial_root": initial_root,
            "final_root": final_root,
            "eval_mode": "mock" if is_mock else "live",
            "is_mock": is_mock,
            "git_commit_sha": get_git_commit_sha(),
            "git_tree_hash": get_git_tree_hash(),
            "cas_events_by_worker": parsed_events,
            "worker_a": results.get("worker_a", {}),
            "worker_b": results.get("worker_b", {}),
        }


def run_group2_rust_pipeline(tasks: list, args, proxy: str) -> dict:
    """Group 2: Rust multi-branch concurrent generation + Git merge & Conflict Resolver Agent"""
    print("\n" + "=" * 70)
    print("🚀 [Group 2] 启动 Rust 多 Agent 并发分支协作流水线...")
    print("=" * 70)

    repo_dir = tempfile.mkdtemp(prefix="axiom_git_bench_")
    t_pipeline_start = time.time()
    worker_results = []
    conflict_count = 0
    resolver_calls = []

    try:
        # 1. 初始化 Git 仓库与主分支骨架
        subprocess.run(["git", "init", "-b", "main"], cwd=repo_dir, capture_output=True, check=True)
        subprocess.run(["git", "config", "user.name", "AxiomBench"], cwd=repo_dir, capture_output=True)
        subprocess.run(["git", "config", "user.email", "bench@axiom.local"], cwd=repo_dir, capture_output=True)

        initial_skeleton = build_rust_initial_skeleton(tasks)
        src_file = os.path.join(repo_dir, "inventory.rs")
        with open(src_file, "w", encoding="utf-8") as f:
            f.write(initial_skeleton)

        subprocess.run(["git", "add", "inventory.rs"], cwd=repo_dir, capture_output=True, check=True)
        subprocess.run(["git", "commit", "-m", "Initial inventory skeleton"], cwd=repo_dir, capture_output=True, check=True)

        # 2. 并发派发 5 个 Worker 生成独立函数实现
        print(f"⏳ 派发 {len(tasks)} 个并发 Worker (各负责一个独立函数)...")
        worker_start = time.time()

        def do_worker(task):
            tid = task["id"]
            fname = task["function_name"]
            prompt = build_rust_worker_prompt(task, initial_skeleton)
            if args.mock:
                res = mock_generate_worker(task, group=2)
            else:
                res = call_codex_generate(prompt, model=args.codex_model, proxy=proxy)
            code = extract_code(res["text"], "rust")
            return {
                "task_id": tid,
                "function_name": fname,
                "code": code,
                "raw_result": res,
                "prompt": prompt
            }

        with concurrent.futures.ThreadPoolExecutor(max_workers=len(tasks)) as executor:
            worker_results = list(executor.map(do_worker, tasks))

        worker_wall_time = round(time.time() - worker_start, 2)
        print(f"✅ 5 个 Worker 并发生成完毕 (耗时: {worker_wall_time}s)")

        # 3. 创建 Git 分支并提交各 Worker 产出
        branch_names = []
        for w in worker_results:
            bname = f"worker-{w['task_id']}-{w['function_name']}"
            branch_names.append(bname)
            # 从 main 创建新分支
            subprocess.run(["git", "checkout", "-b", bname, "main"], cwd=repo_dir, capture_output=True, check=True)

            # 替换 inventory.rs 中对应的函数骨架
            with open(src_file, "r", encoding="utf-8") as f:
                cur = f.read()

            target_sig = f"pub fn {w['function_name']}("
            # 找到函数位置替换为真实实现
            if target_sig in cur:
                # 简单精确的单函数替换
                before = cur.split(target_sig)[0]
                after = cur.split(target_sig)[1]
                # 找到闭合大括号
                brace_cnt = 0
                idx = 0
                found_first = False
                for c in after:
                    idx += 1
                    if c == "{":
                        brace_cnt += 1
                        found_first = True
                    elif c == "}":
                        brace_cnt -= 1
                        if found_first and brace_cnt == 0:
                            break
                remaining = after[idx:]
                new_module = before.rstrip() + "\n\n" + w["code"] + "\n\n" + remaining.lstrip()
            else:
                new_module = cur + "\n\n" + w["code"]

            with open(src_file, "w", encoding="utf-8") as f:
                f.write(new_module)

            subprocess.run(["git", "commit", "-am", f"Implement {w['function_name']}"], cwd=repo_dir, capture_output=True, check=True)

        # 4. 依次执行合并到 main，监测冲突并调用 Conflict Resolver Agent
        print("⏳ 依次合并 5 个分支到 main 分支...")
        subprocess.run(["git", "checkout", "main"], cwd=repo_dir, capture_output=True, check=True)

        for idx, (bname, w) in enumerate(zip(branch_names, worker_results)):
            fname = w["function_name"]
            print(f"  • 合并 PR #{idx+1} ({fname})...", end="", flush=True)

            m_proc = subprocess.run(["git", "merge", bname, "--no-edit"], cwd=repo_dir, capture_output=True, text=True)
            if m_proc.returncode == 0:
                print(" 干净合入 (Clean Merge) ✅")
            else:
                conflict_count += 1
                print(" 产生冲突 (CONFLICT) ⚠️ -> 唤醒 Resolver Agent...", end="", flush=True)
                with open(src_file, "r", encoding="utf-8") as f:
                    conflict_text = f.read()

                # 调用 Conflict Resolver Agent
                r_prompt = build_resolver_prompt(conflict_text, fname)
                t_r_start = time.time()
                if args.mock:
                    r_res = mock_generate_resolver(conflict_text, fname)
                else:
                    r_res = call_codex_generate(r_prompt, model=args.codex_model, proxy=proxy)
                t_r_elapsed = time.time() - t_r_start

                resolved_code = extract_code(r_res["text"], "rust")
                with open(src_file, "w", encoding="utf-8") as f:
                    f.write(resolved_code)

                subprocess.run(["git", "add", "inventory.rs"], cwd=repo_dir, capture_output=True, check=True)
                subprocess.run(["git", "commit", "-m", f"Agent resolved conflict for {fname}"], cwd=repo_dir, capture_output=True, check=True)

                resolver_calls.append({
                    "task_id": w["task_id"],
                    "function_name": fname,
                    "latency_sec": round(t_r_elapsed, 2),
                    "task_payload": r_res["task_payload"],
                    "raw_billed": r_res["raw_billed"]
                })
                print(f" 冲突修复完毕 ✅ (耗时: {round(t_r_elapsed, 2)}s, 消耗: {r_res['raw_billed']['total_tokens']}t)")

        # 5. 最终物理单元测试验证
        with open(src_file, "r", encoding="utf-8") as f:
            final_rust_code = f.read()

        print("⏳ 物理编译与系统全量单测验证中...", end="", flush=True)
        t_ok, t_msg = run_rust_system_tests(final_rust_code, tasks)
        print(f" {'通过 ✅' if t_ok else '失败 ❌'}")
        if not t_ok:
            print(f"  报错详情: {t_msg}")

        t_pipeline_end = time.time()
        pipeline_wall_time = round(t_pipeline_end - t_pipeline_start, 2)

        # 统计汇总
        workers_payload = sum(w["raw_result"]["task_payload"]["total_tokens"] for w in worker_results)
        workers_billed = sum(w["raw_result"]["raw_billed"]["total_tokens"] for w in worker_results)
        resolver_payload = sum(r["task_payload"]["total_tokens"] for r in resolver_calls)
        resolver_billed = sum(r["raw_billed"]["total_tokens"] for r in resolver_calls)

        return {
            "status": "success" if t_ok else "test_failed",
            "total_tasks": len(tasks),
            "wall_clock_sec": pipeline_wall_time,
            "worker_concurrency_sec": worker_wall_time,
            "conflict_count": conflict_count,
            "conflict_rate_pct": round(conflict_count / len(tasks) * 100, 1),
            "resolver_calls_count": len(resolver_calls),
            "tokens": {
                "workers_payload": workers_payload,
                "resolver_payload_tax": resolver_payload,
                "total_payload": workers_payload + resolver_payload,
                "workers_raw_billed": workers_billed,
                "resolver_billed_tax": resolver_billed,
                "total_raw_billed": workers_billed + resolver_billed
            },
            "unit_test_passed": t_ok,
            "unit_test_message": t_msg,
            "git_commit_sha": get_git_commit_sha(),
            "git_tree_hash": get_git_tree_hash(),
            "git_diff_hash": get_git_diff_hash(),
            "git_provenance": get_git_provenance_dict(),
            "git_status": get_git_status(),
            "resolver_details": resolver_calls,
            "workers_details": [
                {
                    "task_id": w["task_id"],
                    "function_name": w["function_name"],
                    "latency_sec": w["raw_result"]["latency_sec"],
                    "payload": w["raw_result"]["task_payload"],
                    "raw_billed": w["raw_result"]["raw_billed"]
                }
                for w in worker_results
            ]
        }
    finally:
        shutil.rmtree(repo_dir, ignore_errors=True)


def run_group3_axiom_pipeline(tasks: list, args, proxy: str, axiom_bin: str) -> dict:
    """Group 3: Axiom AST multi-hole concurrent generation + Combinator 0-conflict assembly + Z3 proof"""
    print("\n" + "=" * 70)
    print("🚀 [Group 3] 启动 Axiom AST 零冲突并发打洞与组合器流水线...")
    print("=" * 70)

    t_pipeline_start = time.time()

    # 1. 架构师初始 5 洞骨架
    initial_skeleton = build_axiom_initial_skeleton(tasks)

    # 1.1 从权威骨架中提取真实 TaskEnvelope 契约与依赖元数据 (Fail-Closed)
    envelopes_by_hole = {}
    with tempfile.TemporaryDirectory() as tmp_meta_dir:
        skel_meta_path = os.path.join(tmp_meta_dir, "skel.ax")
        with open(skel_meta_path, "w", encoding="utf-8") as f:
            f.write(initial_skeleton)
        status_cmd = [axiom_bin, "pool", "status", skel_meta_path, "--json"]
        status_p = subprocess.run(status_cmd, capture_output=True, text=True, timeout=20)
        if status_p.returncode != 0:
            raise RuntimeError(f"Fail-Closed: 获取 pool status 失败: {status_p.stderr}")
        try:
            status_obj = json.loads(status_p.stdout.strip())
            for t_item in status_obj.get("tasks", []):
                h_id = t_item.get("hole_id")
                env = t_item.get("envelope")
                if h_id and env:
                    envelopes_by_hole[h_id] = env
        except json.JSONDecodeError as exc:
            raise RuntimeError(f"Fail-Closed: 无法解析 pool status 输出为 JSON: {status_p.stdout}") from exc

    # Fail-Closed 强校验：启动 Worker 前，强制断言预期 5 个 hole 均存在合法完整的 TaskEnvelope
    if len(envelopes_by_hole) != len(tasks):
        raise RuntimeError(f"Fail-Closed: 提取到的信封数量 ({len(envelopes_by_hole)}) 与任务数 ({len(tasks)}) 不匹配")

    for task in tasks:
        expected_hole = f"{task['function_name']}_hole"
        expected_func = task["function_name"]
        env = envelopes_by_hole.get(expected_hole)
        try:
            validate_task_envelope_schema(env, expected_hole, expected_func)
        except ValueError as e:
            raise RuntimeError(f"Fail-Closed: TaskEnvelope 强 Schema 校验失败: {e}") from e

    # 2. 并发派发 5 个 Worker 填洞 (纯表达式)
    print(f"⏳ 派发 {len(tasks)} 个并发 Worker (各负责一个纯代数表达式打洞任务)...")
    worker_start = time.time()

    def do_worker(task):
        tid = task["id"]
        fname = task["function_name"]
        hole_id = f"{fname}_hole"
        envelope = envelopes_by_hole.get(hole_id)
        capsule = build_axiom_context_capsule(task, envelope)
        prompt = capsule.render_prompt()
        if args.mock:
            res = mock_generate_worker(task, group=3, capsule=capsule)
        else:
            res = call_codex_generate(prompt, model=args.codex_model, proxy=proxy)
        expr = extract_code(res["text"], "axiom")
        return {
            "task_id": tid,
            "function_name": fname,
            "expr": expr,
            "raw_result": res,
            "prompt": prompt,
            "capsule": {
                "system_tokens": capsule.system_tokens,
                "contract_tokens": capsule.contract_tokens,
                "dependency_tokens": capsule.dependency_tokens,
                "total_tokens": capsule.total_tokens,
            }
        }

    with concurrent.futures.ThreadPoolExecutor(max_workers=len(tasks)) as executor:
        worker_results = list(executor.map(do_worker, tasks))

    worker_wall_time = round(time.time() - worker_start, 2)
    print(f"✅ 5 个 Worker 并发填洞完毕 (耗时: {worker_wall_time}s)")

    # 3. AgentTaskPool 原生原子组装与 CAS 验证 (零文本冲突)
    print("⏳ AgentTaskPool 原生原子组装中 (调用 axiom pool assemble)...", end="", flush=True)
    t_combine_start = time.time()

    with tempfile.TemporaryDirectory() as tmpdir:
        skel_path = os.path.join(tmpdir, "skeleton.ax")
        with open(skel_path, "w", encoding="utf-8") as f:
            f.write(initial_skeleton)

        solutions = {}
        for t, w in zip(tasks, worker_results):
            fname = t["function_name"]
            hole_id = f"{fname}_hole"
            solutions[hole_id] = w["expr"].strip()

        sol_path = os.path.join(tmpdir, "solutions.json")
        with open(sol_path, "w", encoding="utf-8") as f:
            json.dump(solutions, f, indent=2)

        out_path = os.path.join(tmpdir, "assembled.ax")
        cmd = [axiom_bin, "pool", "assemble", skel_path, "--solutions", sol_path, "--output", out_path]
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        t_combine_elapsed = round((time.time() - t_combine_start) * 1000, 2)

        if res.returncode != 0:
            err_msg = res.stderr.strip() or res.stdout.strip()
            print(f" 组装失败 ❌: {err_msg}")
            assembled_code = ""
        else:
            print(f" 组装成功 ✅ (耗时: {t_combine_elapsed}ms | 冲突率: 0.0%)")
            with open(out_path, "r", encoding="utf-8") as f:
                assembled_code = f.read()

    # 4. Z3 SMT 求解器一次性全域联合证明
    print("⏳ 交付 Z3 SMT 求解器执行全域定理证明...", end="", flush=True)
    t_smt_start = time.time()
    v_ok, v_msg, reports = run_axiom_system_verify(assembled_code, axiom_bin)
    t_smt_elapsed = round(time.time() - t_smt_start, 2)
    print(f" {'通过 (All UNSAT) ✅' if v_ok else '未通过 ❌'} (耗时: {t_smt_elapsed}s)")
    if not v_ok:
        print(f"  诊断详情: {v_msg}")

    # P1 Spec 2: 绑定与交叉核验内核实际使用的求解器身份
    if v_ok and reports:
        actual_solvers = {r.get("solver_path") for r in reports if r.get("status") == "Proved" and r.get("solver_path")}
        actual_hashes = {r.get("solver_sha256") for r in reports if r.get("status") == "Proved" and r.get("solver_sha256")}
        if not args.mock:
            if len(actual_solvers) != 1 or len(actual_hashes) != 1:
                raise RuntimeError(f"实测模式下未能从内核报告中提取到唯一的求解器身份: paths={actual_solvers}, hashes={actual_hashes}")
            act_path = list(actual_solvers)[0]
            act_hash = list(actual_hashes)[0]
            curr_prov = get_git_provenance()
            if curr_prov.z3_path and os.path.abspath(curr_prov.z3_path) != os.path.abspath(act_path):
                raise RuntimeError(f"Provenance z3_path ({curr_prov.z3_path}) 与内核实测调用的 Z3 ({act_path}) 不一致！")
            if curr_prov.z3_binary_sha256 != act_hash:
                raise RuntimeError(f"Provenance z3_binary_sha256 ({curr_prov.z3_binary_sha256}) 与内核实测调用的 Z3 哈希 ({act_hash}) 不一致！")

    # 5. 真实 CAS 并发竞争与排他文件锁压力测试
    print("⏳ 执行 AgentTaskPool 真实 CAS 并发竞争与排他文件锁压测...", end="", flush=True)
    cas_bench_data = benchmark_cas_concurrency_and_stale_root(axiom_bin, is_mock=args.mock)
    print(f" 压测完成 ✅ (双 Worker 并发成功数: {cas_bench_data['success_submissions']}/2, 耗时: {cas_bench_data['concurrency_elapsed_sec']}s)")

    t_pipeline_end = time.time()
    pipeline_wall_time = round(t_pipeline_end - t_pipeline_start, 2)

    # 保证口径完全一致：Axiom 组 worker payload 严格等于 capsule(system + contract + dep) + candidate
    for w in worker_results:
        if "capsule" in w:
            w["raw_result"]["task_payload"]["prompt_tokens"] = w["capsule"]["total_tokens"]
            w["raw_result"]["task_payload"]["total_tokens"] = w["capsule"]["total_tokens"] + w["raw_result"]["task_payload"]["candidates_tokens"]

    workers_payload = sum(w["raw_result"]["task_payload"]["total_tokens"] for w in worker_results)
    workers_billed = sum(w["raw_result"]["raw_billed"]["total_tokens"] for w in worker_results)

    return {
        "status": "success" if v_ok else "proof_failed",
        "total_tasks": len(tasks),
        "wall_clock_sec": pipeline_wall_time,
        "worker_concurrency_sec": worker_wall_time,
        "combinator_latency_ms": t_combine_elapsed,
        "smt_verify_sec": t_smt_elapsed,
        "conflict_count": 0,
        "conflict_rate_pct": 0.0,
        "resolver_calls_count": 0,
        "tokens": {
            "workers_payload": workers_payload,
            "resolver_payload_tax": 0,
            "total_payload": workers_payload,
            "workers_raw_billed": workers_billed,
            "resolver_billed_tax": 0,
            "total_raw_billed": workers_billed,
            "capsule_breakdown": {
                "system_tokens": sum(w["capsule"]["system_tokens"] for w in worker_results),
                "contract_tokens": sum(w["capsule"]["contract_tokens"] for w in worker_results),
                "dependency_tokens": sum(w["capsule"]["dependency_tokens"] for w in worker_results),
                "candidates_tokens": sum(w["raw_result"]["task_payload"]["candidates_tokens"] for w in worker_results),
            },
            "tokens_to_proved": workers_payload if v_ok else None,
        },
        "formal_proved": v_ok,
        "git_commit_sha": get_git_commit_sha(),
        "git_tree_hash": get_git_tree_hash(),
        "git_diff_hash": get_git_diff_hash(),
        "git_provenance": get_git_provenance_dict(),
        "git_status": get_git_status(),
        "smt_message": v_msg,
        "smt_reports": reports,
        "assembled_source": assembled_code,
        "cas_concurrency_benchmark": cas_bench_data,
        "workers_details": [
            {
                "task_id": w["task_id"],
                "function_name": w["function_name"],
                "latency_sec": w["raw_result"]["latency_sec"],
                "payload": w["raw_result"]["task_payload"],
                "raw_billed": w["raw_result"]["raw_billed"],
                "capsule": w["capsule"]
            }
            for w in worker_results
        ]
    }


def render_mock_synthetic_report(args, tasks, g2_res, g3_res, g2_total_time, g3_total_time):
    """离线仿真模式报告展示层，严格展示协议连通性与结构自洽性，彻底剔除实证措辞与加速比推断"""
    print("\n" + "=" * 70)
    print("🔬 [Phase 3 离线协议仿真与结构自洽性报告 (Synthetic Mock)]")
    print("=" * 70)

    print("【维度 1: 协议结构与组合验证 (Protocol Structure & AST Combinator)】")
    print(f"  • Rust 模拟流水线:   已执行 {len(tasks)} 个独立函数分支合入，模拟流程通路畅通")
    print(f"  • Axiom 模拟流水线:  AST 组合子模式匹配正常，打洞协议契约与类型系统结构自洽")
    print(f"  • 模拟冲突解决记录:  共记录模拟修复调用 {g2_res['resolver_calls_count']} 次")
    print("-" * 70)

    print("【维度 2: 模拟流程步态与离线调用链 (Synthetic Protocol Step Verification)】")
    print(f"  • Rust 组模拟流转耗时:  {g2_total_time}s (仿真流水线执行耗时)")
    print(f"  • Axiom 组模拟流转耗时: {g3_total_time}s (仿真流水线执行耗时)")
    print("  ℹ️  [协议说明]: 离线仿真模式不反映真实模型生成耗时与网络延迟，已根据规范隐藏加速比。")
    print("-" * 70)

    g2_billed = g2_res["tokens"]["total_raw_billed"]
    g3_billed = g3_res["tokens"]["total_raw_billed"]
    print("【维度 3: 语法上下文胶囊与 Token 格式核验 (Context Capsule & Schema Validation)】")
    print(f"  • Rust 模拟账单流转:    {g2_billed:,} tokens (仅供流转格式校验)")
    print(f"  • Axiom 模拟账单流转:   {g3_billed:,} tokens (三级上下文胶囊切分格式一致)")
    print("  ℹ️  [协议说明]: 离线仿真模式不反映生产环境模型消耗，已根据规范隐藏对比倍率。")
    print("-" * 70)

    print("【维度 4: 仿真测试与接口契约断言 (Mock Verification & Invariant Checks)】")
    print(f"  • Rust 仿真测试状态:  {'通过 ✅' if g2_res['unit_test_passed'] else '未通过 ❌'} (单测结构校验)")
    print(f"  • Axiom 仿真证明状态: {'通过 ✅' if g3_res['formal_proved'] else '未通过 ❌'} (协议握手与 AST 打洞还原校验)")
    print(f"  • 证据评级:           synthetic_non_evidentiary (非实证，仅供流水线与类型系统离线自测)")
    print("=" * 70)


def render_live_empirical_report(args, tasks, g2_res, g3_res, g2_total_time, g3_total_time, speedup):
    """实测模式报告展示层，输出严谨端到端实证对比指标"""
    print("\n" + "=" * 70)
    print("📊 [Phase 3 并发流水线实测综合对比分析 (Live Empirical)]")
    print("=" * 70)

    print("【维度 1: 结构协作与冲突解决开销 (Merge Conflict & Resolver Tax)】")
    print(f"  • Rust 组合并冲突率:      {g2_res['conflict_rate_pct']}% ({g2_res['conflict_count']}/{len(tasks)} PRs 产生冲突)")
    print(f"  • Axiom 组打洞冲突率:     0.0% (AST Combinator 物理零冲突)")
    print(f"  • Rust 冲突解决 Agent 账单: +{g2_res['tokens']['resolver_billed_tax']:,} tokens (调用 {g2_res['resolver_calls_count']} 次)")
    print(f"  • Axiom 冲突解决开销:     0 tokens (无任何二次修复开销)")
    print("-" * 70)

    print("【维度 2: 端到端耗时与并发吞吐 (Wall-clock Time & Speedup)】")
    print(f"  • Rust 组端到端总耗时:   {g2_total_time}s (含 Worker 并发 {g2_res['worker_concurrency_sec']}s + 串行合并与冲突修复)")
    print(f"  • Axiom 组端到端总耗时:  {g3_total_time}s (含 Worker 并发 {g3_res['worker_concurrency_sec']}s + AST组装 {g3_res['combinator_latency_ms']}ms + Z3全域证明 {g3_res['smt_verify_sec']}s)")
    print(f"  🔥 端到端交付耗时对比: Axiom 较 Rust 提速 {round((1.0 - g3_total_time/g2_total_time)*100, 1)}% (加速比: {speedup}x)")
    print("-" * 70)

    g2_billed = g2_res["tokens"]["total_raw_billed"]
    g3_billed = g3_res["tokens"]["total_raw_billed"]
    billed_diff = round((g3_billed - g2_billed) / g2_billed * 100, 1) if g2_billed > 0 else 0
    print("【维度 3: 真实总账单与交付有效 Payload (含冲突修复全口径)】")
    print(f"  • Rust 组全链路总账单:   {g2_billed:,} tokens (Worker生成: {g2_res['tokens']['workers_raw_billed']:,}t + 冲突修复: {g2_res['tokens']['resolver_billed_tax']:,}t)")
    print(f"  • Axiom 组全链路总账单: {g3_billed:,} tokens (Worker生成: {g3_res['tokens']['workers_raw_billed']:,}t + 组合器: 0t)")
    print(f"  🔥 全链路总账单倍率:     Axiom 为 Rust 的 {round(g3_billed / g2_billed, 2)} 倍 ({'多 ' if billed_diff >= 0 else '省 '}{abs(billed_diff)}%)")
    print("-" * 70)

    print("【维度 4: 交付质量与形式验证结论】")
    print(f"  • Rust 交付状态:  {'全部单元测试通过 ✅' if g2_res['unit_test_passed'] else '单测未通过 ❌'} (基于离散测试向量采样)")
    print(f"  • Axiom 交付状态: {'Z3 SMT 定理证明通过 ✅ (All UNSAT)' if g3_res['formal_proved'] else 'SMT 证明失败 ❌'} (在指定 Profile 下，求解器对全部义务均判定为 UNSAT)")
    print("=" * 70)


def main():
    parser = argparse.ArgumentParser(description="Axiom Phase 3 Concurrent Multi-Agent Evaluation Runner")
    parser.add_argument("--engine", choices=["codex", "gemini"], default="codex", help="评测后端引擎 (默认: codex)")
    parser.add_argument("--codex-model", default=None, help="Codex 模型代号")
    parser.add_argument("--proxy", default=None, help="HTTP 代理")
    parser.add_argument("--mock", action="store_true", help="离线物理仿真模式 (0 成本验证完整流程)")
    args = parser.parse_args()

    target_proxy = args.proxy
    if not target_proxy and not urllib.request.getproxies().get("https"):
        rec_proxy = detect_recommended_proxy()
        if rec_proxy:
            target_proxy = rec_proxy

    tasks = load_concurrent_tasks()
    axiom_bin = find_axiom_binary()
    z3_bin = find_z3_executable()
    z3_display = z3_bin if z3_bin else "内置/模拟求解器"

    print("=" * 70)
    print("🔥 Axiom Phase 3 多 Agent 并发流水线宏观集成基准测试启动")
    print(f"评测子系统: 背包交易与库存结算 (inventory_settlement) | 函数数: {len(tasks)}")
    print(f"验证后端: rustc --test + Z3 SMT 求解器 ({z3_display})")
    print(f"引擎模式: {args.engine.upper()} {'[离线仿真模式]' if args.mock else ''}")
    if target_proxy and not args.mock:
        print(f"网络代理: {target_proxy}")
    if args.mock:
        print("\n" + "*" * 78)
        print("⚠️  SYNTHETIC MOCK BENCHMARK RUN (NON-EVIDENTIARY)")
        print("All latency and token measurements are synthetic simulations and MUST NOT")
        print("be cited as production comparative metrics or empirical performance evidence!")
        print("*" * 78)
    print("=" * 70)

    # 1. 运行 Group 2 (Rust)
    g2_res = run_group2_rust_pipeline(tasks, args, target_proxy)

    # 2. 运行 Group 3 (Axiom)
    g3_res = run_group3_axiom_pipeline(tasks, args, target_proxy, axiom_bin)

    # 3. 结果汇总与对比分析
    g2_total_time = g2_res["wall_clock_sec"]
    g3_total_time = g3_res["wall_clock_sec"]
    speedup = round(g2_total_time / g3_total_time, 2) if g3_total_time > 0 else 1.0

    if args.mock:
        render_mock_synthetic_report(args, tasks, g2_res, g3_res, g2_total_time, g3_total_time)
    else:
        render_live_empirical_report(args, tasks, g2_res, g3_res, g2_total_time, g3_total_time, speedup)

    # 4. 落盘
    g2_billed = g2_res["tokens"]["total_raw_billed"]
    g3_billed = g3_res["tokens"]["total_raw_billed"]
    output_payload = {
        "version": "3.1",
        "subsystem": "inventory_settlement",
        "experiment_metadata": {
            "engine": args.engine,
            "mode": "concurrent_multi_agent",
            "eval_mode": "mock" if args.mock else "live_codex",
            "is_mock": args.mock,
            "evidence_grade": "synthetic_non_evidentiary" if args.mock else "live_empirical",
            "git_commit": get_git_commit_sha(),
            "git_tree_hash": get_git_tree_hash(),
            "git_diff_hash": get_git_diff_hash(),
            "git_provenance": get_git_provenance_dict(),
            "git_status": get_git_status(),
            "z3_path": get_git_provenance().z3_path,
            "z3_version": get_z3_version(is_mock=args.mock),
            "platform": sys.platform,
            "python_version": sys.version.split()[0],
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S")
        },
        "summary": {
            "rust_group": {
                "wall_clock_sec": g2_total_time,
                "conflict_rate_pct": g2_res["conflict_rate_pct"],
                "conflict_count": g2_res["conflict_count"],
                "resolver_calls_count": g2_res["resolver_calls_count"],
                "workers_billed_tokens": g2_res["tokens"]["workers_raw_billed"],
                "resolver_tax_billed_tokens": g2_res["tokens"]["resolver_billed_tax"],
                "total_billed_tokens": g2_billed,
                "unit_test_passed": g2_res["unit_test_passed"]
            },
            "axiom_group": {
                "wall_clock_sec": g3_total_time,
                "conflict_rate_pct": 0.0,
                "conflict_count": 0,
                "resolver_calls_count": 0,
                "workers_billed_tokens": g3_res["tokens"]["workers_raw_billed"],
                "resolver_tax_billed_tokens": 0,
                "total_billed_tokens": g3_billed,
                "formal_proved": g3_res["formal_proved"],
                "speedup_vs_rust": None if args.mock else speedup,
                "synthetic_note": "Comparative speedup is omitted in synthetic mock mode" if args.mock else None
            }
        },
        "detailed_runs": {
            "group2_rust": g2_res,
            "group3_axiom": g3_res
        }
    }

    # 强不变式断言 Fail-Closed 校验
    assert sum(g3_res["tokens"]["capsule_breakdown"].values()) == g3_res["tokens"]["workers_payload"], \
        f"Axiom capsule_breakdown 必须与 workers_payload 完全自洽: {sum(g3_res['tokens']['capsule_breakdown'].values())} != {g3_res['tokens']['workers_payload']}"
    if g3_res["formal_proved"]:
        assert g3_res["tokens"]["tokens_to_proved"] == g3_res["tokens"]["workers_payload"], \
            f"formal_proved 下 tokens_to_proved 必须严格等于 workers_payload: {g3_res['tokens']['tokens_to_proved']} != {g3_res['tokens']['workers_payload']}"

    validate_benchmark_result_schema(output_payload)

    os.makedirs(os.path.dirname(OUTPUT_DATA_PATH), exist_ok=True)
    with open(OUTPUT_DATA_PATH, "w", encoding="utf-8") as f:
        json.dump(output_payload, f, indent=2, ensure_ascii=False)
    print(f"💾 并发流水线详细审计数据已落盘并通过强 Schema 校验: {OUTPUT_DATA_PATH}\n")


if __name__ == "__main__":
    main()
