#!/usr/bin/env python3
"""窗口群框架 v0.1 骨架 — 命令面实现（对齐 execution-bridge-v22 v1.1 补丁1/5）。

独立项目，不依赖 codex-rust 主仓库源码。实现 v0.1 必备 9 条命令：
  project create/open/status
  window list/create/start/stop/delete/restore
  framework check（窗口 wiring 4 断言，补丁7）
其余命令（snapshot/rollback/conflict/budget/export/import/template）在 --help 占位。
"""
import argparse
import json
import os
import shutil
import sys
import tomllib
from pathlib import Path
from datetime import datetime, timezone

VERSION = "1.0.3"
PROJECTS_ROOT = Path(os.environ.get("CODEX_PROJECTS_ROOT", str(Path.home() / ".codex-projects")))

# ── 项目骨架模板 ─────────────────────────────────────────────

PROJECT_TOML_TEMPLATE = """# 项目配置（v2.1 schema）
[project]
id = "{project_id}"
name = "{name}"
type = "{project_type}"
created_at = "{created_at}"

[workflow]
stages = []
"""

WINDOW_TOML_TEMPLATE = """# 窗口元数据（v2.1 schema）
[window]
id = "{win_id}"
name = "{name}"
role = "{role}"
prompt = {prompt}   # WT17: JSON 字面量（转义换行/引号/反斜杠，防注入破坏 TOML）
state = "pending"          # pending | working | blocked | done | archived
created_by = "{created_by}"  # auto | human
created_at = "{created_at}"

[context]
max_tokens = 32000
current_tokens = 0
current_cost = 0.0
compression_count = 0

[budget]                   # §8.1 必需段（缺则 framework check 红）
max_steps = {max_steps}
max_cost_cny = {max_cost_cny}
provider = "{provider}"
timeout_min = 30

[dependencies]
upstream = []

[outputs]
files = []
gate = ""
"""


# ── 框架断言（补丁7：窗口群 wiring）──────────────────────────

class FrameworkCheck:
    """codex framework check — 4 条断言，每条可失败（对齐 project-xray 哲学）。"""

    def __init__(self, project_root: Path):
        self.root = project_root
        self.results = []

    def _record(self, name: str, ok: bool, detail: str = ""):
        self.results.append({"name": name, "ok": ok, "detail": detail})
        print(f"  [{'PASS' if ok else 'FAIL'}] {name} — {detail}")

    def run(self) -> bool:
        print(f"framework check: {self.root}")
        windows_dir = self.root / "windows"

        # 断言1: window.toml-exists
        missing = []
        for d in sorted(windows_dir.iterdir()) if windows_dir.exists() else []:
            if d.is_dir() and not (d / "window.toml").exists():
                missing.append(d.name)
        self._record("window.toml-exists", not missing,
                     "全部窗口含 window.toml" if not missing else f"缺: {missing}")

        # 断言2: budget-declared（§8.1 必需）
        no_budget = []
        for d in sorted(windows_dir.iterdir()) if windows_dir.exists() else []:
            wt = d / "window.toml"
            if wt.exists():
                data = tomllib.loads(wt.read_text(encoding="utf-8"))
                if "budget" not in data:
                    no_budget.append(d.name)
        self._record("budget-declared", not no_budget,
                     "全部窗口声明 [budget]" if not no_budget else f"缺: {no_budget}")

        # 断言3: gate-script-exists（§8.3 脚本化）
        no_gate = []
        for d in sorted(windows_dir.iterdir()) if windows_dir.exists() else []:
            wt = d / "window.toml"
            if wt.exists():
                data = tomllib.loads(wt.read_text(encoding="utf-8"))
                gate = data.get("outputs", {}).get("gate", "")
                # v1.0.3: human: 开头的 gate 是描述不是脚本——豁免（多窗口 dogfooding 暴露）
                if gate and not gate.startswith("human:") and not (self.root / gate).exists():
                    no_gate.append(f"{d.name}->{gate}")
        self._record("gate-script-exists", not no_gate,
                     "gate 脚本全部存在" if not no_gate else f"缺: {no_gate}")

        # 断言4: conflict-marked（§8.4：同路径产出必须标记冲突）
        # v0.1 简化：检查是否有两个窗口声明了同一 outputs 路径
        path_owners = {}
        conflicts = []
        for d in sorted(windows_dir.iterdir()) if windows_dir.exists() else []:
            wt = d / "window.toml"
            if wt.exists():
                data = tomllib.loads(wt.read_text(encoding="utf-8"))
                for out in data.get("outputs", {}).get("files", []):
                    if out in path_owners:
                        conflicts.append(f"{out} (by {path_owners[out]} & {d.name})")
                    else:
                        path_owners[out] = d.name
        self._record("conflict-marked", not conflicts,
                     "无路径冲突" if not conflicts else f"冲突: {conflicts}")

        # 断言5: stage-role-unique（WT19 audit-fix：同 stage 窗口 role 查重）
        # 多窗口协作允许全局同 role（多个 dev），但同一 stage 内重复 role
        # 会让 analyze/deploy 的 duplicate roles 校验失败——check 阶段就要暴露。
        dup_roles = []
        pt = self.root / "project.toml"
        if pt.exists():
            data = tomllib.loads(pt.read_text(encoding="utf-8"))
            for s in data.get("workflow", {}).get("stages", []):
                seen = {}
                for wid in s.get("windows", []):
                    wt = self.root / "windows" / wid / "window.toml"
                    if not wt.exists():
                        continue
                    wd = tomllib.loads(wt.read_text(encoding="utf-8"))
                    role = str(wd.get("window", {}).get("role", "")).replace(" ", "").lower()
                    if role in seen:
                        dup_roles.append(f"stage {s.get('id')}: {seen[role]} & {wid} 同 role={role}")
                    else:
                        seen[role] = wid
        self._record("stage-role-unique", not dup_roles,
                     "同 stage 窗口 role 无重复" if not dup_roles else f"重复: {dup_roles}")

        all_ok = all(r["ok"] for r in self.results)
        print(f"framework check: {sum(1 for r in self.results if r['ok'])}/{len(self.results)} "
              f"{'PASS' if all_ok else 'BROKEN'}")
        return all_ok


# ── 命令实现 ─────────────────────────────────────────────────

def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def cmd_project_create(args):
    pid = args.name.replace(" ", "-").lower()
    root = PROJECTS_ROOT / pid
    if root.exists():
        print(f"project '{pid}' already exists at {root}")
        return 1
    root.mkdir(parents=True)
    for sub in ["windows", "shared", "shared/outputs", "shared/gates", "shared/specs",
                ".trash", ".archive", ".snapshots", "exports"]:
        (root / sub).mkdir(parents=True)
    (root / "project.toml").write_text(
        PROJECT_TOML_TEMPLATE.format(project_id=pid, name=args.name,
                                     project_type=args.type, created_at=now_iso()),
        encoding="utf-8")
    (root / "shared/progress.md").write_text(f"# {args.name} 进度板\n\n", encoding="utf-8")
    (root / "shared/decisions.md").write_text(f"# {args.name} 决策记录\n\n", encoding="utf-8")
    print(f"created project '{pid}' ({args.type}) at {root}")
    return 0


def cmd_project_open(args):
    # 自动发现：当前目录或参数
    root = Path(args.name) if args.name else Path.cwd()
    if not (root / "project.toml").exists():
        # 尝试从 projects root 找
        cand = PROJECTS_ROOT / args.name
        if (cand / "project.toml").exists():
            root = cand
        else:
            print(f"no project.toml found (looked at {root} and {PROJECTS_ROOT})")
            return 1
    print(f"opened project at {root}")
    return 0


def cmd_project_status(args):
    root = PROJECTS_ROOT / args.name if args.name else Path.cwd()
    if not (root / "project.toml").exists():
        print(f"no project.toml at {root}")
        return 1
    data = tomllib.loads((root / "project.toml").read_text(encoding="utf-8"))
    print(f"project: {data['project']['name']} ({data['project']['type']})")
    print(f"  id: {data['project']['id']}  created: {data['project']['created_at']}")
    print("  windows:")
    wins = root / "windows"
    for d in sorted(wins.iterdir()) if wins.exists() else []:
        wt = d / "window.toml"
        if wt.exists():
            w = tomllib.loads(wt.read_text(encoding="utf-8"))["window"]
            print(f"    {w['id']:<20} [{w['state']:<8}] {w['role']}")
    return 0


def _win_id(root: Path, name: str) -> str:
    """窗口 ID 生成：win-{role}-{n} 或 win-{name}，冲突自动 -2/-3。"""
    if name.startswith("win-"):
        base = name
    else:
        base = f"win-{name}"
    wins = root / "windows"
    existing = {d.name for d in wins.iterdir()} if wins.exists() else set()
    if base not in existing:
        return base
    n = 2
    while f"{base}-{n}" in existing:
        n += 1
    return f"{base}-{n}"


def cmd_window_create(args):
    root = PROJECTS_ROOT / args.project
    if not (root / "project.toml").exists():
        print(f"project '{args.project}' not found")
        return 1
    # WT19 注记：不在 create 层做全局 role 查重——同一项目多窗口可同 role
    # （多 dev 协作场景）。重复 role 的检查落在有 stage 上下文的
    # FrameworkCheck（同 stage 窗口 role 查重）+ validate_deploy_yaml（analyze/deploy）。
    win_id = _win_id(root, args.name)
    wdir = root / "windows" / win_id
    wdir.mkdir(parents=True)
    (wdir / "window.toml").write_text(
        WINDOW_TOML_TEMPLATE.format(win_id=win_id, name=args.name, role=args.role,
                                    prompt=json.dumps(args.prompt, ensure_ascii=False),
                                    created_by="human", created_at=now_iso(),
                                    max_steps=args.max_steps,
                                    max_cost_cny=args.max_cost,
                                    provider=args.provider),
        encoding="utf-8")
    print(f"created window '{win_id}' (role={args.role}, budget={args.max_steps} steps)")
    return 0


def _load_windows(root: Path):
    wins = {}
    for d in sorted((root / "windows").iterdir()) if (root / "windows").exists() else []:
        wt = d / "window.toml"
        if wt.exists():
            data = tomllib.loads(wt.read_text(encoding="utf-8"))
            wins[d.name] = data
    return wins


def _set_state(root: Path, win_id: str, state: str):
    wt = root / "windows" / win_id / "window.toml"
    if not wt.exists():
        print(f"window '{win_id}' not found")
        return False
    text = wt.read_text(encoding="utf-8")
    import re
    text = re.sub(r'(state = ")[^"]*(")', f'\\g<1>{state}\\g<2>', text)
    wt.write_text(text, encoding="utf-8")
    print(f"window '{win_id}' -> {state}")
    return True


def cmd_window_list(args):
    root = PROJECTS_ROOT / args.project
    wins = _load_windows(root)
    for wid, data in sorted(wins.items()):
        w = data["window"]
        tag = " [adhoc]" if w["created_by"] == "human" else ""
        print(f"  {wid:<20} [{w['state']:<8}]{tag} {w['role']}")
    return 0


def cmd_window_start(args):
    return 0 if _set_state(PROJECTS_ROOT / args.project, args.id, "working") else 1


def cmd_window_stop(args):
    rc = 0 if _set_state(PROJECTS_ROOT / args.project, args.id, "blocked") else 1
    # v0.3 S3: stop 自动快照
    if rc == 0:
        _auto_snapshot(PROJECTS_ROOT / args.project, args.id)
    return rc


def cmd_window_delete(args):
    root = PROJECTS_ROOT / args.project
    src = root / "windows" / args.id
    if not src.exists():
        print(f"window '{args.id}' not found")
        return 1
    if args.hard:
        shutil.rmtree(src)
        print(f"window '{args.id}' hard-deleted")
    else:
        trash = root / ".trash" / args.id
        if trash.exists():
            shutil.rmtree(trash)
        shutil.move(str(src), str(trash))
        print(f"window '{args.id}' moved to .trash/ (restore within 30 days)")
    return 0


def cmd_window_restore(args):
    root = PROJECTS_ROOT / args.project
    src = root / ".trash" / args.id
    if not src.exists():
        print(f"no trashed window '{args.id}'")
        return 1
    shutil.move(str(src), str(root / "windows" / args.id))
    print(f"window '{args.id}' restored")
    return 0


# ── v0.2: 对话存储 / agent 引擎 / 快照 / 导出 ─────────────────

CONVERSION_FIELDS = ("t", "role", "content", "tool_calls", "tool_results")


def _conv_path(root: Path, win_id: str) -> Path:
    return root / "windows" / win_id / "conversation.jsonl"


def _append_conv(root: Path, win_id: str, entry: dict):
    """追加一条对话记录（JSONL，增量写）。"""
    p = _conv_path(root, win_id)
    line = json.dumps(entry, ensure_ascii=False)
    with open(p, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def _read_conv(root: Path, win_id: str) -> list:
    p = _conv_path(root, win_id)
    if not p.exists():
        return []
    entries = []
    for l in p.read_text(encoding="utf-8").splitlines():
        if not l.strip():
            continue
        try:
            entries.append(json.loads(l))
        except json.JSONDecodeError:
            continue  # 脏行跳过（容错）
    return entries


def _update_budget(root: Path, win_id: str, tokens: int, cost: float):
    """v0.2 补充4：budget 双上限运行时追踪。"""
    wt = root / "windows" / win_id / "window.toml"
    text = wt.read_text(encoding="utf-8")
    import re
    m = re.search(r"current_tokens = (\d+)", text)
    cur_tok = int(m.group(1)) if m else 0
    m = re.search(r"current_cost = ([\d.]+)", text)
    cur_cost = float(m.group(1)) if m else 0.0
    text = re.sub(r"current_tokens = [\d]+", f"current_tokens = {cur_tok + tokens}", text)
    text = re.sub(r"current_cost = [\d.]+", f"current_cost = {cur_cost + cost:.4f}", text)
    wt.write_text(text, encoding="utf-8")


def _budget_exceeded(root: Path, win_id: str) -> str:
    """返回超限原因；未超限返回空串。"""
    data = tomllib.loads((root / "windows" / win_id / "window.toml").read_text(encoding="utf-8"))
    ctx, bud = data["context"], data["budget"]
    if ctx["current_tokens"] > ctx["max_tokens"]:
        return f"tokens exceeded (>{ctx['max_tokens']})"
    if ctx["current_cost"] > bud["max_cost_cny"]:
        return f"cost exceeded (>{bud['max_cost_cny']} CNY)"
    return ""


class Agent:
    """v0.2 S1: 最小 agent 引擎——单窗口单轮工具循环（独立于 codex-rust agent-core）。

    写路径沙箱（补充2）：只允许 {window}/outputs/ 与 {project}/shared/outputs/。
    """

    TOOLS = [
        {"type": "function", "function": {
            "name": "write_file", "description": "写入/创建文件（仅限窗口 outputs 与 shared/outputs）",
            "parameters": {"type": "object", "properties": {
                "path": {"type": "string"}, "content": {"type": "string"}},
                "required": ["path", "content"]}}},
        {"type": "function", "function": {
            "name": "read", "description": "读取文件（shared 只读 + 本窗口 outputs）",
            "parameters": {"type": "object", "properties": {
                "path": {"type": "string"}}, "required": ["path"]}}},
        {"type": "function", "function": {
            "name": "bash", "description": "执行 shell 命令（工作目录=项目根）",
            "parameters": {"type": "object", "properties": {
                "cmd": {"type": "string"}}, "required": ["cmd"]}}},
    ]

    # v1.0.2: provider 路由表（原 hardcode deepseek——LLM 产出 provider 字段是摆设）
    PROVIDERS = {
        "deepseek": {"base_url": "https://api.deepseek.com/v1",
                     "env_key": "DEEPSEEK_API_KEY",
                     "model": "deepseek-v4-flash"},
        "zhipu": {"base_url": "https://open.bigmodel.cn/api/paas/v4",
                  "env_key": "ZHIPU_API_KEY",
                  "model": "glm-4.5"},
        "agnes": {"base_url": "https://api.agnes-ai.cn/v1",
                  "env_key": "AGNES_API_KEY",
                  "model": "agnes-2.5-flash"},
        "openai": {"base_url": "https://api.openai.com/v1",
                   "env_key": "OPENAI_API_KEY",
                   "model": "gpt-4o-mini"},
    }

    def __init__(self, root: Path, win_id: str, provider: str = "auto"):
        self.root = root
        self.win_id = win_id
        self.win_dir = root / "windows" / win_id
        self.data = tomllib.loads((self.win_dir / "window.toml").read_text(encoding="utf-8"))
        # v1.0.2: provider="auto" → 从 window.toml budget.provider 路由
        if provider == "auto":
            provider = self.data["budget"].get("provider", "deepseek")
        self.provider = provider
        # P2-5 (audit-fix): 未知 provider 显式报错，不再静默回退 deepseek
        # （拼写错误必须立刻暴露，否则 key 发错端点/凭据错配）
        if provider not in self.PROVIDERS:
            raise ValueError(
                f"unknown provider {provider!r}; available: {sorted(self.PROVIDERS)}"
            )
        spec = self.PROVIDERS[provider]
        self.base_url = spec["base_url"]
        self.api_key = os.environ.get(spec["env_key"], "")
        self._model = spec["model"]

    def _allowed_write(self, path: str) -> bool:
        # P1-5 (audit-fix): is_relative_to 取代 startswith——杜绝前缀旁路
        # （outputs_evil/x、outputsX/x 不再被误判为 outputs 内）。
        p = Path(path).resolve()
        out_dirs = [
            (self.win_dir / "outputs").resolve(),
            (self.root / "shared" / "outputs").resolve(),
        ]
        return any(p.is_relative_to(d) for d in out_dirs)

    def _allowed_read(self, path: str) -> bool:
        p = Path(path).resolve()
        allow = [
            (self.root / "shared").resolve(),
            (self.win_dir / "outputs").resolve(),
            self.win_dir.resolve(),
            self.root.resolve(),
        ]
        return any(p.is_relative_to(d) for d in allow)

    def _chat(self, messages: list) -> dict:
        """调用 OpenAI 兼容 chat API（非流式）。返回 {content, tool_calls, usage}。"""
        import urllib.request
        body = json.dumps({
            "model": self.data["budget"].get("model", self._model),
            "messages": messages, "tools": self.TOOLS,
            "tool_choice": "auto", "temperature": 0.3,
        }).encode()
        req = urllib.request.Request(f"{self.base_url}/chat/completions", data=body,
                                     headers={"Content-Type": "application/json",
                                              "Authorization": f"Bearer {self.api_key}"})
        try:
            with urllib.request.urlopen(req, timeout=120) as r:
                resp = json.loads(r.read().decode())
        except urllib.error.HTTPError as e:
            # v1.0.1 dogfooding: 打印响应体（400 诊断必须看 body）
            detail = e.read().decode(errors="replace")[:500]
            raise RuntimeError(f"LLM HTTP {e.code}: {detail}") from e
        msg = resp["choices"][0]["message"]
        usage = resp.get("usage", {})
        return {"content": msg.get("content"), "tool_calls": msg.get("tool_calls"),
                "usage": usage, "reasoning_content": msg.get("reasoning_content")}

    def _has_outputs(self) -> bool:
        """v1.0.1 dogfooding: 窗口是否产出了文件（outputs/ 任一**非空文件**）。

        P1-5 (audit-fix): 0 字节文件不算产出——否则写个空文件就骗过 done 闸门。
        """
        for d in (self.win_dir / "outputs", self.root / "shared" / "outputs"):
            if not d.exists():
                continue
            for f in d.rglob("*"):
                if f.is_file() and f.stat().st_size > 0:
                    return True
        return False

    def _exec_tool(self, name: str, args: dict) -> str:
        """执行工具 + 沙箱校验。"""
        if name == "write_file":
            path, content = args["path"], args["content"]
            if not self._allowed_write(path):
                return "DENIED: path outside window outputs"
            try:
                fp = Path(path)
                fp.parent.mkdir(parents=True, exist_ok=True)
                fp.write_text(content, encoding="utf-8")
                return f"wrote {len(content)} bytes to {path}"
            except Exception as e:
                # v1.0.1: 写失败不崩——返回错误让 LLM 修正路径
                return f"WRITE_ERROR: {e}"
        if name == "read":
            if not self._allowed_read(args["path"]):
                return "DENIED: path outside readable scope"
            fp = Path(args["path"])
            if not fp.exists():
                return "NO_SUCH_FILE"
            # v1.0.1: 截断大文件 + 容错非 utf-8（二进制/压缩文件不崩）
            try:
                text = fp.read_text(encoding="utf-8", errors="replace")
            except Exception as e:
                return f"READ_ERROR: {e}"
            return text[:8000] + (f"\n...[TRUNCATED {len(text) - 8000} chars]"
                                  if len(text) > 8000 else "")
        if name == "bash":
            import shlex
            import subprocess
            # P0-5 (audit-fix): bash 零沙箱 → 命令白名单 + shlex.split（去 shell=True）
            # + 路径参数沙箱校验 + 禁网 env。WINDOW_ALLOW_RAW_BASH=1 显式逃生。
            if os.environ.get("WINDOW_ALLOW_RAW_BASH") not in ("1", "true"):
                BASH_ALLOWED = {
                    "ls", "cat", "grep", "find", "pwd", "wc", "head", "tail",
                    "python3", "pytest", "git", "mkdir", "cp", "mv",
                }
                try:
                    parts = shlex.split(args["cmd"])
                except ValueError as e:
                    return f"BASH_PARSE_ERROR: {e}"
                if not parts or parts[0] not in BASH_ALLOWED:
                    return (f"DENIED: command '{parts[0] if parts else ''}' not in bash "
                            f"whitelist {sorted(BASH_ALLOWED)} — set WINDOW_ALLOW_RAW_BASH=1 to override")
                # 路径参数沙箱校验：绝对路径 / .. 必须在读/写允许范围内
                for a in parts[1:]:
                    if a.startswith("~"):
                        return f"DENIED: ~ expansion not allowed: {a}"
                    if a.startswith("/") or a.startswith(".."):
                        cand = Path(a).resolve()
                        if not (self._allowed_read(str(cand)) or self._allowed_write(str(cand))):
                            return f"DENIED: path {a} outside sandbox scope"
                cmd_parts = parts
            else:
                try:
                    cmd_parts = shlex.split(args["cmd"])
                except ValueError as e:
                    return f"BASH_PARSE_ERROR: {e}"
            # 禁网：清空代理相关 env（防经 bash 外泄）
            clean_env = {k: v for k, v in os.environ.items()
                         if "proxy" not in k.lower()}
            try:
                cp = subprocess.run(cmd_parts, shell=False, capture_output=True,
                                    text=True, cwd=str(self.root).replace("\\", "/"),
                                    timeout=30, env=clean_env)
                return (cp.stdout + cp.stderr)[:2000]
            except subprocess.TimeoutExpired:
                # v1.0.1 dogfooding: bash 超时不崩溃——返回提示让 LLM 换命令
                return "BASH_TIMEOUT: command took >30s — use narrower command"
            except Exception as e:
                return f"BASH_ERROR: {e}"
        return f"UNKNOWN_TOOL:{name}"

    def run(self, goal: str, max_turns: int = 12, start_turn: int = 0) -> int:
        """启动 agent：system prompt + 工具循环，直到完成或预算超限。

        AGENT_MODE=replay（v0.3 补充3）：不调 LLM，立即标 done + 写假对话——测试 0 成本。
        v0.9: start_turn 用于 resume（从断点续编号）。
        """
        import time
        t0 = time.time()  # v0.9: step_log wall_ms 计时
        if not self.api_key and os.environ.get("AGENT_MODE", "real") != "replay":
            # v1.0.3: 明确是哪个 provider 缺 key（不再静默 deepseek 语义混淆）
            spec = self.PROVIDERS.get(self.provider, self.PROVIDERS["deepseek"])
            print(f"ERROR: {spec['env_key']} not set (provider={self.provider}, "
                  f"set env or .env)")
            return 1
        w = self.data["window"]
        system = (f"你是窗口 {w['id']}（{w['role']}）。{w.get('prompt','')}\n"
                  f"目标: {goal}\n"
                  f"可写: {self.win_dir}/outputs/ 与 {self.root}/shared/outputs/\n"
                  f"只读: {self.root}/shared/（specs/decisions/progress）\n"
                  f"行动纪律（v1.0.1）：探索最多 6 轮，之后必须开始产出；"
                  f"最终必须调用 write_file 至少一次写入 outputs/，否则视为失败被 blocked。")
        messages = [{"role": "system", "content": system},
                    {"role": "user", "content": goal}]
        _append_conv(self.root, self.win_id, {"t": now_iso(), "role": "system", "content": system})
        _append_conv(self.root, self.win_id, {"t": now_iso(), "role": "user", "content": goal})
        _set_state(self.root, self.win_id, "working")

        # v0.3 补充3: replay 模式（测试用，不调 LLM）——但仍走循环以验证快照/预算逻辑
        if os.environ.get("AGENT_MODE", "real") == "replay":
            # v0.9: replay 也写 step_log（供 run --verbose demo 渲染）
            replay_phases = ["Plan", "Read", "Write", "Bash"]
            for i in range(max_turns):
                turn = start_turn + i + 1
                # WT4 (audit-fix): replay 也查预算——极端 max_steps（如 99999）
                # 不得产生 ~2000 次快照（每 50 轮一次）
                over = _budget_exceeded(self.root, self.win_id)
                if over:
                    print(f"budget exceeded (replay): {over} — stopping")
                    _set_state(self.root, self.win_id, "blocked")
                    return 2
                if turn % CompressionEngine.AUTO_SNAPSHOT_EVERY == 0:
                    _auto_snapshot(self.root, self.win_id)
                    print(f"auto snapshot at turn {turn}")
                phase = replay_phases[i % len(replay_phases)]
                _append_conv(self.root, self.win_id, {
                    "t": now_iso(), "role": "assistant",
                    "content": f"[replay] turn {turn}",
                    "meta": {"step_log": {"turn": turn, "phase": "Act",
                                          "tool": phase, "summary": f"replay {phase}",
                                          "tokens": 100, "wall_ms": 1000}}})
            print(f"agent done (replay mode, {max_turns} turns)")
            _set_state(self.root, self.win_id, "done")
            return 0

        no_produce = 0  # v1.0.1: 连续无 write_file 的轮数（防探索死循环）
        for i in range(max_turns):
            turn = start_turn + i + 1
            # v0.4 补充4: 50 轮自动快照
            if turn % CompressionEngine.AUTO_SNAPSHOT_EVERY == 0:
                _auto_snapshot(self.root, self.win_id)
                print(f"auto snapshot at turn {turn}")
            over = _budget_exceeded(self.root, self.win_id)
            if over:
                print(f"budget exceeded: {over} — stopping")
                # v1.0.1: 任何超限都是异常终止 → blocked（tokens 超限≠成功完成）
                _set_state(self.root, self.win_id, "blocked")
                return 2
            # v1.0.1: 连续探索 ≥6 轮无产出 → 强制引导（否则 LLM 会一直 read/sed）
            if no_produce >= 6:
                nudge = (f"[系统] 你已连续 {no_produce} 轮未产出任何文件。"
                         f"你的任务要求最终必须 write_file 到 outputs/。"
                         f"请立即停止探索，直接写出你已掌握的内容产物。")
                messages.append({"role": "user", "content": nudge})
                no_produce = 0
            try:
                resp = self._chat(messages)
            except Exception as e:
                print(f"LLM error: {e}")
                _append_conv(self.root, self.win_id, {"t": now_iso(), "role": "tool",
                                                      "content": f"LLM_ERROR:{e}"})
                _set_state(self.root, self.win_id, "blocked")
                return 3
            # budget 追踪（补充4）——v1.0.1: 用 completion_tokens 增量（每轮新增消耗），
            # 而非 total_tokens（含 prompt 全文，多轮累加必然爆炸）
            u = resp["usage"]
            tok = u.get("completion_tokens", 0) or 0
            cost = tok * 0.000001  # 粗估单价（deepseek 约 $1/M tokens）
            _update_budget(self.root, self.win_id, tok, cost)

            # v0.9: StepLog 持久化（meta.step_log）——resume/--verbose 依赖
            tool_names = [tc["function"]["name"] for tc in (resp["tool_calls"] or [])]
            tool_summary = ""
            if tool_names:
                first = resp["tool_calls"][0]["function"]["name"]
                try:
                    fargs = json.loads(resp["tool_calls"][0]["function"]["arguments"] or "{}")
                    tool_summary = fargs.get("path") or fargs.get("cmd", "")[:40] or first
                except json.JSONDecodeError:
                    tool_summary = first
            step_log = {"turn": turn, "phase": "Act",
                        "tool": tool_names[0] if tool_names else "",
                        "summary": tool_summary, "tokens": tok,
                        "wall_ms": round((time.time() - t0) * 1000)}
            _append_conv(self.root, self.win_id, {"t": now_iso(), "role": "assistant",
                                                  "content": resp["content"],
                                                  "tool_calls": resp["tool_calls"],
                                                  "meta": {"step_log": step_log}})
            if resp["tool_calls"]:
                produced = False
                for tc in resp["tool_calls"]:
                    name = tc["function"]["name"]
                    args = json.loads(tc["function"]["arguments"] or "{}")
                    result = self._exec_tool(name, args)
                    if name == "write_file":
                        produced = True
                    _append_conv(self.root, self.win_id, {"t": now_iso(), "role": "tool",
                                                          "name": name, "content": result[:1000]})
                    asst = {"role": "assistant", "content": None,
                            "tool_calls": [{"id": tc["id"], "type": "function",
                                            "function": {"name": name,
                                                         "arguments": tc["function"]["arguments"]}}]}
                    # v1.0.1 dogfooding: deepseek 推理模式必须回传 reasoning_content
                    if resp.get("reasoning_content"):
                        asst["reasoning_content"] = resp["reasoning_content"]
                    messages.append(asst)
                    # v1.0.1: messages 里工具结果也截断（read 全文会导致 tokens 爆炸超 32k）
                    messages.append({"role": "tool", "tool_call_id": tc["id"],
                                     "content": result[:2000]})
                # v1.0.1: 追踪连续无产出轮数（write_file 才算产出）
                if not produced:
                    no_produce += 1
                else:
                    no_produce = 0
            else:
                # 无工具调用 = 声称完成 → v1.0.1 必须验证产出（否则虚假 done）
                if not self._has_outputs():
                    print(f"agent claims done but NO outputs — blocking (turn {turn})")
                    _set_state(self.root, self.win_id, "blocked")
                    return 4
                print(f"agent done (turn {turn + 1})")
                _set_state(self.root, self.win_id, "done")
                return 0
        print(f"max_turns reached ({max_turns}) — done")
        # v1.0.1: max_turns 到但无产出 = 未完成 → blocked 而非 done
        if not self._has_outputs():
            print("max_turns reached but NO outputs — blocking")
            _set_state(self.root, self.win_id, "blocked")
            return 4
        _set_state(self.root, self.win_id, "done")
        return 0


def cmd_window_start(args):
    root = PROJECTS_ROOT / args.project
    win_dir = root / "windows" / args.id
    if not (win_dir / "window.toml").exists():
        print(f"window '{args.id}' not found")
        return 1
    data = tomllib.loads((win_dir / "window.toml").read_text(encoding="utf-8"))
    if not data["window"].get("prompt"):
        print(f"window '{args.id}' has no prompt — refusing to start (no-brain window)")
        return 1
    goal = args.goal or "完成 window.toml 中声明的角色任务，产出文件到 outputs/。"
    agent = Agent(root, args.id, provider=data["budget"].get("provider", "deepseek"))
    return agent.run(goal, max_turns=args.max_turns)


def cmd_window_export(args):
    root = PROJECTS_ROOT / args.project
    entries = _read_conv(root, args.id)
    if not entries:
        print(f"no conversation for '{args.id}'")
        return 1
    # v0.5 补充: --compress 只输出摘要+热层；--full 完整（默认 full）
    if args.compress:
        engine = CompressionEngine(root, args.id)
        entries = [e for e in entries if e.get("role") == "summary"] + entries[-20:]
    if args.format == "json":
        out = json.dumps(entries, ensure_ascii=False, indent=2)
    else:  # markdown
        lines = [f"# 窗口 {args.id} 对话导出\n"]
        for e in entries:
            role = e.get("role", "?")
            t = e.get("t", "")[11:19]
            # WT15 (audit-fix): 导出不截断内容——往返一致性（export → import
            # 不得丢字）。原 [:200]/[:150]/[:300] 截断导致有损。
            if role == "system":
                lines.append(f"**system** ({t}): {str(e.get('content',''))}\n")
            elif role == "user":
                lines.append(f"**user** ({t}): {str(e.get('content',''))}\n")
            elif role == "assistant":
                tcs = e.get("tool_calls") or []
                tc_summary = ", ".join(tc["function"]["name"] for tc in tcs) if tcs else ""
                lines.append(f"**assistant** ({t}): {str(e.get('content') or '')}"
                             + (f"  ⟪tools: {tc_summary}⟫" if tc_summary else "") + "\n")
            elif role == "tool":
                lines.append(f"  ↳ tool {e.get('name','')}: {str(e.get('content',''))}\n")
            elif role == "summary":
                meta = e.get("meta", {})
                lines.append(f"**📦 摘要 (第 {meta.get('compressed_rounds','?')} 轮, "
                             f"compression #{meta.get('compression_id','?')})**: "
                             f"{str(e.get('content',''))}\n")
        out = "".join(lines)
    dest = args.output or str(root / "exports" / f"{args.id}-conversation.md")
    if args.format == "json" and not args.output:
        dest = str(root / "exports" / f"{args.id}-conversation.json")
    Path(dest).parent.mkdir(parents=True, exist_ok=True)
    Path(dest).write_text(out, encoding="utf-8")
    print(f"exported {len(entries)} messages -> {dest}")
    return 0


def cmd_window_snapshot(args):
    root = PROJECTS_ROOT / args.project
    win_dir = root / "windows" / args.id
    if not win_dir.exists():
        print(f"window '{args.id}' not found")
        return 1
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")
    snap = root / ".snapshots" / f"{args.id}--{ts}"
    n = 2
    while snap.exists():  # 同秒冲突：追加 -2/-3（对齐窗口 ID 命名规则）
        snap = root / ".snapshots" / f"{args.id}--{ts}-{n}"
        n += 1
    snap.mkdir(parents=True)
    shutil.copy(win_dir / "window.toml", snap / "window.toml")
    conv = win_dir / "conversation.jsonl"
    if conv.exists():
        shutil.copy(conv, snap / "conversation.jsonl")
    # P2-issue-2 (audit-fix): 手动快照纳入 outputs/——与 _auto_snapshot 保持一致，
    # 否则手动 snapshot 后 rollback 还原不了产出（cmd_window_rollback 依赖 snap/outputs）。
    out_dir = win_dir / "outputs"
    if out_dir.exists():
        shutil.copytree(out_dir, snap / "outputs")
    print(f"snapshot saved: {snap}")
    return 0


def cmd_window_rollback(args):
    root = PROJECTS_ROOT / args.project
    win_dir = root / "windows" / args.id
    # P2-6 (audit-fix): --to 白名单——不得含路径分隔符/..（防逃出 .snapshots/）
    if not args.to or any(ch in args.to for ch in ("/", "\\", "..", "~")):
        print(f"invalid snapshot id: {args.to!r} (path traversal rejected)")
        return 1
    snap = root / ".snapshots" / f"{args.id}--{args.to}"
    if not snap.exists():
        print(f"no snapshot {args.id}--{args.to}")
        return 1
    # P2-6 (audit-fix): 先校验目标有效——缺 conversation.jsonl 的快照视为坏快照，
    # 不得在「先 move 当前再判断」的旧顺序里把当前对话弄丢。
    if not (snap / "conversation.jsonl").exists():
        print(f"snapshot {args.id}--{args.to} missing conversation.jsonl — refuse (data loss guard)")
        return 1
    safe_ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")  # Windows 安全（无冒号）
    # 备份当前历史（只在目标有效后）
    conv = win_dir / "conversation.jsonl"
    if conv.exists():
        rb = root / ".snapshots" / "rolled-back"
        rb.mkdir(parents=True, exist_ok=True)
        shutil.move(str(conv), str(rb / f"{args.id}--{safe_ts}.jsonl"))
    shutil.copy(snap / "window.toml", win_dir / "window.toml")
    shutil.copy(snap / "conversation.jsonl", conv)
    # P2-6: outputs/ 一并还原（快照里有才还原；当前 outputs 先整体移走防混合）
    out_dir = win_dir / "outputs"
    snap_out = snap / "outputs"
    if out_dir.exists() or snap_out.exists():
        old_out = None
        if out_dir.exists():
            old_out = root / ".snapshots" / "rolled-back" / f"{args.id}--{safe_ts}-outputs"
            shutil.move(str(out_dir), str(old_out))
        if snap_out.exists():
            shutil.copytree(snap_out, out_dir)
    print(f"window '{args.id}' rolled back to {args.to}")
    return 0


def cmd_window_snapshot_list(args):
    root = PROJECTS_ROOT / args.project
    snaps = sorted((root / ".snapshots").glob(f"{args.id}--*")) if (root / ".snapshots").exists() else []
    for s in snaps:
        print(f"  {s.name}")
    print(f"{len(snaps)} snapshots")
    return 0


def cmd_window_status(args):
    root = PROJECTS_ROOT / args.project
    wt = root / "windows" / args.id / "window.toml"
    if not wt.exists():
        print(f"window '{args.id}' not found")
        return 1
    data = tomllib.loads(wt.read_text(encoding="utf-8"))
    w = data["window"]
    ctx = data["context"]
    conv = _read_conv(root, args.id)
    print(f"window {w['id']}: state={w['state']} role={w['role']}")
    print(f"  tokens: {ctx['current_tokens']}/{ctx['max_tokens']}  "
          f"cost: {ctx['current_cost']}/{data['budget']['max_cost_cny']} CNY")
    print(f"  conversation: {len(conv)} messages")
    return 0


# ── v0.3: 自动快照 + 工作流引擎 + workflow deploy ──────────

def _auto_snapshot(root: Path, win_id: str):
    """v0.3 S3: 自动快照（stop 时触发；50 轮在 agent 循环中调）。

    P2-6 (audit-fix): 快照纳入 outputs/——回滚必须能还原产出，
    否则「快照回滚」是半还原（对话回退但产出文件不回退）。
    """
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")
    snap = root / ".snapshots" / f"{win_id}--{ts}"
    n = 2
    while snap.exists():
        snap = root / ".snapshots" / f"{win_id}--{ts}-{n}"
        n += 1
    snap.mkdir(parents=True)
    win_dir = root / "windows" / win_id
    shutil.copy(win_dir / "window.toml", snap / "window.toml")
    conv = win_dir / "conversation.jsonl"
    if conv.exists():
        shutil.copy(conv, snap / "conversation.jsonl")
    # P2-6: outputs/ 一并快照（存在才拷）
    out_dir = win_dir / "outputs"
    if out_dir.exists():
        shutil.copytree(out_dir, snap / "outputs")


class WorkflowEngine:
    """v0.3 S1: 工作流引擎——读 project.toml [workflow]，按 trigger/gate 推进。"""

    def __init__(self, root: Path):
        self.root = root
        data = tomllib.loads((root / "project.toml").read_text(encoding="utf-8"))
        self.stages = data.get("workflow", {}).get("stages", [])
        self.status_file = root / "shared" / "progress.md"

    def _stage_done_mark(self, stage_id: str) -> str:
        return f"{stage_id}:done"

    def _window_state(self, win_id: str) -> str:
        wt = self.root / "windows" / win_id / "window.toml"
        if not wt.exists():
            return "missing"
        return tomllib.loads(wt.read_text(encoding="utf-8"))["window"]["state"]

    @staticmethod
    def _gate_path_ok(root: Path, path: str) -> bool:
        """WT11 (audit-fix): gate 路径白名单——拒绝绝对路径 / ~ / .. 逃逸 / 超出项目根。

        防止 `auto:../../etc/passwd` 之类经 gate 脚本路径越权执行/写入。
        """
        if not path or path.startswith(("/", "\\", "~")):
            return False
        if ".." in path.split("/"):
            return False
        try:
            return (root / path).resolve().is_relative_to(root.resolve())
        except OSError:
            return False

    def _run_gate(self, gate_spec: str) -> bool:
        """v0.3.1 补充2：gate 双语义——human: 人工 / auto:path 脚本 / 无前缀默认脚本。"""
        if gate_spec.startswith("human:"):
            return False  # 停在 stage 等人审
        path = gate_spec[5:] if gate_spec.startswith("auto:") else gate_spec
        # WT11 (audit-fix): 路径白名单——越权路径当 blocked 处理，不执行
        if not self._gate_path_ok(self.root, path):
            print(f"gate path denied (WT11): {path!r} — treating as blocked")
            return False
        gp = self.root / path
        if not gp.exists():
            print(f"gate script missing: {gp} — treating as blocked")
            return False
        import subprocess
        # 相对路径 + cwd=项目根：兼容 Git Bash（MSYS 不转义 /c/ 前缀）和 Linux
        cp = subprocess.run(["bash", path], capture_output=True, text=True,
                            cwd=str(self.root), timeout=60)
        ok = cp.returncode == 0
        print(f"gate {path}: {'PASS' if ok else 'FAIL'} (rc={cp.returncode})")
        return ok

    def run(self, max_rounds: int = 20) -> int:
        """推进工作流直到全部完成或阻塞。返回 0=全完成 / 1=阻塞待人类。"""
        done_stages = set()
        # P2-3 (audit-fix): 引擎必须读 blocked: ——reject 的 stage 不得推进
        blocked_stages = set()
        progress = []

        # 解析 progress.md 已有 done/blocked 标记
        if self.status_file.exists():
            for line in self.status_file.read_text(encoding="utf-8").splitlines():
                ls = line.strip()
                if ls.startswith("done:"):
                    done_stages.add(ls[5:].strip())
                elif ls.startswith("blocked:"):
                    blocked_stages.add(ls[8:].strip())

        for _ in range(max_rounds):
            progressed = False
            for stage in self.stages:
                sid = stage["id"]
                if sid in done_stages:
                    continue
                # P2-3: 被 reject 的 stage 卡住不推进——等人类 approve/retry
                if sid in blocked_stages:
                    continue
                # trigger 检查
                trig = stage.get("trigger", "")
                if trig == "project_start":
                    trig_ok = True
                elif trig.endswith(":done"):
                    trig_ok = trig[:-5] in done_stages
                else:
                    trig_ok = False
                if not trig_ok:
                    continue
                # 启动本 stage 窗口（若未 working/done）
                wins = stage.get("windows", [])
                all_done = True
                for wid in wins:
                    st = self._window_state(wid)
                    if st == "missing":
                        print(f"window {wid} missing — stage {sid} blocked")
                        return 1
                    if st == "pending":
                        # v0.3: 启动 = 真跑 agent（replay 模式直接 done，real 调 LLM）
                        _set_state(self.root, wid, "working")
                        progress.append(f"started {wid} (stage {sid})")
                        print(f"workflow: started {wid}")
                        # v1.0.2: provider=auto → 从窗口 budget.provider 路由
                        agent = Agent(self.root, wid, provider="auto")
                        # v1.0.1 dogfooding: 用窗口 budget.max_steps（默认 40）替代硬编码 8
                        max_t = agent.data["budget"].get("max_steps", 40)
                        agent.run("(workflow auto-start)", max_turns=max_t)
                        st = self._window_state(wid)  # 重读（agent 可能 done/blocked）
                    elif st in ("blocked", "error"):
                        print(f"workflow: {wid} {st} — stage {sid} blocked")
                        return 1
                    elif st == "working":
                        # v1.0.1 dogfooding: working 残留（上次进程已死）→ 重置重跑，
                        # 否则引擎永久卡死（pending 不启动 / blocked 不拦截 / done 永假）
                        _set_state(self.root, wid, "pending")
                        print(f"workflow: {wid} stale working → reset to pending")
                        st = "pending"
                        # WT6 修复（红队 audit-findings-v22）：重置后该窗口未 done——
                        # 必须置 all_done=False，否则 continue 跳过下方检查，
                        # 单窗口 stage 被自动 gate 判 done 而窗口从未运行（假完成）
                        all_done = False
                        continue
                    if st != "done":
                        all_done = False
                if not all_done:
                    progressed = True
                    continue
                # 全部 done → gate
                gate_ok = self._run_gate(stage.get("gate", ""))
                if not gate_ok:
                    gate_spec = stage.get("gate", "")
                    if gate_spec.startswith("human:"):
                        print(f"workflow: stage {sid} waiting human gate — approve with "
                              f"'codex workflow gate {sid} --approve'")
                    else:
                        print(f"workflow: stage {sid} gate FAILED — blocked")
                        with open(self.status_file, "a", encoding="utf-8") as f:
                            f.write(f"blocked:{sid}\n")
                    return 1
                # gate 通过 → 标记 done + 触发下一 stage
                done_stages.add(sid)
                with open(self.status_file, "a", encoding="utf-8") as f:
                    f.write(f"done:{sid}\n")
                print(f"workflow: stage {sid} done → trigger {sid}:done")
                progressed = True
            if not progressed:
                break

        remaining = [s["id"] for s in self.stages if s["id"] not in done_stages]
        if remaining:
            print(f"workflow: remaining stages {remaining} — waiting (human gates or deps)")
            return 1
        print("workflow: ALL STAGES DONE")
        return 0


def cmd_workflow_start(args):
    root = PROJECTS_ROOT / args.project
    if not (root / "project.toml").exists():
        print(f"project '{args.project}' not found")
        return 1
    return WorkflowEngine(root).run(max_rounds=args.max_rounds)


def cmd_workflow_watch(args):
    """v0.7 S3: 持续轮询（不退出）直到全部 stage done 或阻塞/等人类。"""
    import time
    root = PROJECTS_ROOT / args.project
    if not (root / "project.toml").exists():
        print(f"project '{args.project}' not found")
        return 1
    deadline = time.time() + args.timeout * 60
    engine = WorkflowEngine(root)
    prev_done_count = -1
    while time.time() < deadline:
        rc = engine.run(max_rounds=args.max_rounds)
        if rc == 0:
            print("workflow watch: ALL STAGES DONE")
            return 0
        # 本轮进展判定：若 done stage 数没增加且无窗口在工作 → 停滞（等人类或 deadlock）
        done_count = 0
        if engine.status_file.exists():
            done_count = sum(1 for l in
                             engine.status_file.read_text(encoding="utf-8").splitlines()
                             if l.strip().startswith("done:"))
        any_working = any(
            engine._window_state(wid) == "working"
            for stage in engine.stages for wid in stage.get("windows", []))
        if done_count == prev_done_count and not any_working:
            # 看是否卡在 human gate（progress.md 有 waiting 提示）
            status = engine.status_file.read_text(encoding="utf-8") if engine.status_file.exists() else ""
            if "waiting human" in status or any(
                    "human" in s.get("gate", "") for s in engine.stages
                    if f"done:{s['id']}" not in status):
                print("workflow watch: waiting human gates — no more auto progress")
            else:
                print("workflow watch: no progress (possible deadlock)")
            return 1
        prev_done_count = done_count
        time.sleep(args.interval)
    print(f"workflow watch: timeout after {args.timeout}min")
    return 1


def cmd_workflow_gate(args):
    """人类审核 gate：--approve 标记 stage done，--reject 写 blocked。

    v1.0.3: approve 前验证该 stage 窗口全部 done——防止"窗口 pending 却 approve"
    （多窗口 dogfooding 暴露：progress.md 残留 done 标记 → 引擎跳过窗口 → 全链路假 done）。
    """
    import tomllib as TL
    root = PROJECTS_ROOT / args.project
    status = root / "shared" / "progress.md"
    if args.approve:
        # 验证 stage 窗口全部 done
        data = TL.loads((root / "project.toml").read_text(encoding="utf-8"))
        for s in data.get("workflow", {}).get("stages", []):
            if s.get("id") == args.stage:
                pending = []
                for wid in s.get("windows", []):
                    wt = root / "windows" / wid / "window.toml"
                    st = TL.loads(wt.read_text(encoding="utf-8"))["window"]["state"] if wt.exists() else "missing"
                    if st != "done":
                        pending.append(f"{wid}={st}")
                if pending:
                    print(f"ERROR: stage {args.stage} 窗口未全部 done: {pending} — 拒绝 approve")
                    return 1
                break
    # P2-3 (audit-fix): 幂等 + 对侧标记清理——
    # approve 清 blocked: 行；reject 清 done: 行；重复操作不产生重复标记。
    mark = f"done:{args.stage}" if args.approve else f"blocked:{args.stage}"
    other = f"blocked:{args.stage}" if args.approve else f"done:{args.stage}"
    lines = []
    if status.exists():
        lines = status.read_text(encoding="utf-8").splitlines()
    has_self = any(l.strip() == mark for l in lines)
    if has_self:
        # 幂等：已处于目标状态 → 直接成功，不重复写
        print(f"stage {args.stage} already {'approved → done' if args.approve else 'rejected → blocked'}")
        return 0
    remaining = [l for l in lines if l.strip() != other]
    remaining.append(mark)
    status.write_text("\n".join(remaining) + "\n", encoding="utf-8")
    if args.approve:
        print(f"stage {args.stage} approved → done")
    else:
        print(f"stage {args.stage} rejected → blocked")
    return 0


def cmd_workflow_retry(args):
    """v0.9 S2: retry 指定 stage——只重置该 stage 的窗口为 pending，其余不动。"""
    root = PROJECTS_ROOT / args.project
    engine = WorkflowEngine(root)
    target = next((s for s in engine.stages if s["id"] == args.stage), None)
    if not target:
        print(f"stage '{args.stage}' not found")
        return 1
    for wid in target.get("windows", []):
        wt = root / "windows" / wid / "window.toml"
        if wt.exists():
            _set_state(root, wid, "pending")
            print(f"retry: {wid} → pending")
    print(f"retry: stage '{args.stage}' reset — run 'workflow start' or 'watch'")
    return 0


# ── v0.4: 上下文压缩引擎（三层分层 + summary 存储 + replay 摘要）──

class CompressionEngine:
    """v0.4 S1-S4: 三层压缩（热/温/冷）+ 摘要替换 + 压缩前快照 + 回滚保护。

    v0.4.1 补充1: AGENT_MODE=replay 时生成假摘要（不调 LLM，测试 0 成本）。
    补充2: 压缩后 conversation.jsonl 中替换为 role=summary 行。
    补充4: 顺带实现 50 轮自动快照（v0.3 遗留）。
    """

    HOT_ROUNDS = 20       # 热层：最近 20 轮完整保留
    WARM_CHUNK = 5        # 温层：每 5 轮压一条摘要
    SUMMARY_MAX = 2000    # 摘要上限（§8.6 对齐 MAX_CONSTITUTION_CHARS 教训）
    AUTO_SNAPSHOT_EVERY = 50  # 补充4：50 轮自动快照

    def __init__(self, root: Path, win_id: str):
        self.root = root
        self.win_id = win_id
        self.win_dir = root / "windows" / win_id
        self.wt = self.win_dir / "window.toml"
        self.data = tomllib.loads(self.wt.read_text(encoding="utf-8"))
        # P2-5 (audit-fix): 压缩引擎也走 provider 路由（budget.provider →
        # endpoint/key/model），与 Agent 构造一致；未知 provider 显式报错。
        provider = self.data["budget"].get("provider", "deepseek")
        if provider not in Agent.PROVIDERS:
            raise ValueError(
                f"unknown provider {provider!r}; available: {sorted(Agent.PROVIDERS)}"
            )
        _spec = Agent.PROVIDERS[provider]
        self.provider = provider
        self.base_url = _spec["base_url"]
        self.api_key = os.environ.get(_spec["env_key"], "")
        self._model = _spec["model"]

    def _conv(self) -> list:
        return _read_conv(self.root, self.win_id)

    def _write_conv(self, entries: list):
        p = self.win_dir / "conversation.jsonl"
        with open(p, "w", encoding="utf-8") as f:
            for e in entries:
                f.write(json.dumps(e, ensure_ascii=False) + "\n")

    def _tokens(self) -> int:
        # 粗估：每字符 ~0.35 token（英文），中文更高，取 0.5 保守
        return sum(len(str(e.get("content") or "")) + sum(
            len(str(tc)) for tc in e.get("tool_calls") or []) for e in self._conv()) // 2

    def should_compress(self) -> bool:
        # 重新读 window.toml（外部可能改过 current_tokens）
        data = tomllib.loads(self.wt.read_text(encoding="utf-8"))
        ctx = data["context"]
        return ctx["current_tokens"] > ctx["max_tokens"] * 0.7

    def _fake_summary(self, chunk: list) -> str:
        """v0.4.1 补充1: replay 模式假摘要——内容 = chunk 前 3 行拼接。"""
        head = " | ".join(str(e.get("content") or e.get("name") or "")[:60]
                          for e in chunk[:3])
        return (f"做了什么: [replay] {head}\n关键决策: replay\n"
                f"产出文件: (none)\n未解决: (none)")

    def _llm_summary(self, chunk: list) -> str:
        """real 模式：调 LLM 产真摘要（对齐压缩内容契约）。

        P2-5 (audit-fix): 走 provider 路由（self.base_url/api_key/_model）——
        不再硬编码 DEEPSEEK_API_KEY + api.deepseek.com（provider=zhipu 时
        compress 会错发 deepseek 端点 → 400 / 凭据错配）。model 优先
        budget.model（window 指定），缺省用路由表 model。
        """
        api_key = self.api_key
        if not api_key:
            raise RuntimeError(
                f"API key not set for provider {self.provider!r} "
                f"(env {Agent.PROVIDERS[self.provider]['env_key']}) (real compression)"
            )
        import urllib.request
        text = "\n".join(json.dumps(e, ensure_ascii=False) for e in chunk)
        body = json.dumps({
            "model": self.data["budget"].get("model", self._model),
            "messages": [
                {"role": "system", "content":
                 f"压缩这段窗口对话为结构化摘要（≤{self.SUMMARY_MAX} 字符），必须含："
                 "做了什么/关键决策/产出文件/未解决。"},
                {"role": "user", "content": text[:8000]}],
            "temperature": 0.1,
        }).encode()
        req = urllib.request.Request(
            f"{self.base_url}/chat/completions", data=body,
            headers={"Content-Type": "application/json",
                     "Authorization": f"Bearer {api_key}"})
        with urllib.request.urlopen(req, timeout=60) as r:
            resp = json.loads(r.read().decode())
        return resp["choices"][0]["message"]["content"][:self.SUMMARY_MAX]

    def _make_summary(self, chunk: list, rounds: str, comp_id: int) -> dict:
        if os.environ.get("AGENT_MODE", "real") == "replay":
            content = self._fake_summary(chunk)
        else:
            content = self._llm_summary(chunk)
        return {"t": now_iso(), "role": "summary", "content": content,
                "meta": {"compressed_rounds": rounds, "compression_id": comp_id}}

    def compress(self) -> int:
        """执行压缩。返回 0=成功 / 1=无内容可压 / 2=摘要缺字段回滚。
        返回 3= real 模式无 key（不压，留给调用方）。
        """
        entries = self._conv()
        if len(entries) <= self.HOT_ROUNDS:
            return 1  # 全在热层，无可压
        # 压缩前必快照（S3 / §5f）
        _auto_snapshot(self.root, self.win_id)

        hot = entries[-self.HOT_ROUNDS:]
        old = entries[:-self.HOT_ROUNDS]
        # 温层 = old 中非 summary 的消息（按轮分组每 WARM_CHUNK 条压一条）
        comp_id = self.data["context"].get("compression_count", 0) + 1
        summaries = []
        i = 0
        while i < len(old):
            chunk = old[i:i + self.WARM_CHUNK]
            rounds = f"{i + 1}-{i + len(chunk)}"
            try:
                s = self._make_summary(chunk, rounds, comp_id)
            except RuntimeError:
                return 3
            # 质量自检（S4）：摘要必须含 4 个关键字段
            for key in ("做了什么", "关键决策", "产出文件", "未解决"):
                if key not in s["content"]:
                    # 回滚保护（§5g）
                    _rollback_latest(self.root, self.win_id)
                    return 2
            summaries.append(s)
            i += self.WARM_CHUNK

        # 重建对话：冷层摘要(≤10) + 温层摘要 + 热层完整
        cold = summaries[:-10] if len(summaries) > 10 else []
        warm = summaries[-10:] if len(summaries) > 10 else summaries
        # 冷层再压一条总摘要（replay 直接拼接）
        if cold:
            total = self._fake_summary(cold) if os.environ.get("AGENT_MODE", "real") == "replay" \
                else self._llm_summary(cold)
            cold_block = [{"t": now_iso(), "role": "summary", "content": total,
                           "meta": {"compressed_rounds": "all-old", "compression_id": comp_id}}]
        else:
            cold_block = []

        new_conv = cold_block + warm + hot
        self._write_conv(new_conv)

        # 更新 window.toml：compression_count +1 + current_tokens 重算
        text = self.wt.read_text(encoding="utf-8")
        import re
        text = re.sub(r"compression_count = \d+", f"compression_count = {comp_id}", text)
        text = re.sub(r"current_tokens = \d+", f"current_tokens = {self._tokens()}", text)
        self.wt.write_text(text, encoding="utf-8")
        print(f"compress: {len(old)} msgs → {len(cold_block) + len(warm)} summaries "
              f"(compression #{comp_id})")
        return 0


def _rollback_latest(root: Path, win_id: str):
    """回滚到最新快照（§5g 压缩坏保护）。"""
    snaps = sorted((root / ".snapshots").glob(f"{win_id}--*")) if (root / ".snapshots").exists() else []
    if not snaps:
        return
    snap = snaps[-1]
    conv = root / "windows" / win_id / "conversation.jsonl"
    if (snap / "conversation.jsonl").exists():
        shutil.copy(snap / "conversation.jsonl", conv)
    print(f"rollback: restored conversation from {snap.name}")


def cmd_window_compress(args):
    root = PROJECTS_ROOT / args.project
    engine = CompressionEngine(root, args.id)
    rc = engine.compress()
    msg = {0: "compressed", 1: "nothing to compress (≤20 rounds)",
           2: "bad summary — rolled back", 3: "no API key (real mode)"}[rc]
    print(f"compress: {msg}")
    return 0 if rc in (0, 1) else 1


# ── v0.9: UX 呈现面（window run --verbose / resume）+ polish ──

STEP_ICONS = {"Plan": "💭", "Read": "🔍", "Write": "✏️", "Edit": "✏️",
              "Bash": "🔧", "Fail": "❌", "Done": "✅"}


def _render_step_log(log: dict, budget_total: int) -> str:
    """渲染单行 StepLog（v0.9 呈现面）。"""
    turn = log.get("turn", "?")
    phase = log.get("phase", "")
    tool = log.get("tool", "")
    summary = log.get("summary", "")
    icon = STEP_ICONS.get(tool) or STEP_ICONS.get(phase, "•")
    tokens = log.get("tokens", 0)
    wall = log.get("wall_ms", 0)
    line = f"[{turn}/{budget_total}]  {icon} {phase}"
    if tool:
        line += f" → {summary[:50]}" if summary else f" → {tool}"
    line += f"  (~{wall // 1000}s, {tokens} tok)"
    return line


def cmd_window_run(args):
    """v0.9: window run = start（跑 agent）+ --verbose 实时 StepLog。"""
    root = PROJECTS_ROOT / args.project
    win_dir = root / "windows" / args.id
    if not (win_dir / "window.toml").exists():
        print(f"window '{args.id}' not found")
        return 1
    data = tomllib.loads((win_dir / "window.toml").read_text(encoding="utf-8"))
    if not data["window"].get("prompt"):
        print(f"window '{args.id}' has no prompt — refusing to start")
        return 1
    max_steps = data["budget"].get("max_steps", 40)
    if args.verbose:
        print(f"━━ {args.id} {data['window']['role']} ━━")
    goal = args.goal or "完成 window.toml 中声明的角色任务，产出文件到 outputs/。"
    agent = Agent(root, args.id, provider=data["budget"].get("provider", "deepseek"))
    # WT4 (audit-fix): CLI 未显式指定 max-turns 时尊重 budget.max_steps
    rc = agent.run(goal, max_turns=args.max_turns or max_steps)
    if args.verbose:
        # 渲染 step_log（读 conversation 最后 N 条）
        entries = _read_conv(root, args.id)
        logs = [e["meta"]["step_log"] for e in entries
                if e.get("meta", {}).get("step_log")]
        for log in logs:
            print(_render_step_log(log, max_steps))
        print(f"━━ done (rc={rc}, {len(logs)} steps) ━━")
    return rc


def cmd_window_resume(args):
    """v0.9 补充4: 断点续传——从最后完整 step_log 的 turn+1 继续。"""
    root = PROJECTS_ROOT / args.project
    win_dir = root / "windows" / args.id
    if not (win_dir / "window.toml").exists():
        print(f"window '{args.id}' not found")
        return 1
    entries = _read_conv(root, args.id)
    # 找最后完整 step_log 的 turn
    last_turn = 0
    for e in entries:
        sl = e.get("meta", {}).get("step_log")
        if sl and sl.get("turn"):
            last_turn = max(last_turn, sl["turn"])
    # 丢弃可能不完整的最后一条 assistant（无 tool_calls 且无 content）
    if entries and entries[-1].get("role") == "assistant":
        last = entries[-1]
        if not last.get("tool_calls") and not last.get("content"):
            entries = entries[:-1]
            _write_conv_raw(root, args.id, entries)
    print(f"resume: continuing window '{args.id}' from turn {last_turn + 1}")
    data = tomllib.loads((win_dir / "window.toml").read_text(encoding="utf-8"))
    goal = args.goal or "继续完成角色任务"
    agent = Agent(root, args.id, provider=data["budget"].get("provider", "deepseek"))
    return agent.run(goal, max_turns=args.max_turns, start_turn=last_turn)


def _write_conv_raw(root: Path, win_id: str, entries: list):
    p = root / "windows" / win_id / "conversation.jsonl"
    with open(p, "w", encoding="utf-8") as f:
        for e in entries:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")


def cmd_status(args):
    """v0.9 S3: 全局聚合状态（一条命令回答"项目现在怎么样了"）。"""
    import glob
    projects = sorted(glob.glob(str(PROJECTS_ROOT / "*")))
    for proot in projects:
        pt = Path(proot) / "project.toml"
        if not pt.exists():
            continue
        try:
            pdata = tomllib.loads(pt.read_text(encoding="utf-8"))
        except Exception:
            continue
        name = pdata["project"]["name"]
        ptype = pdata["project"]["type"]
        print(f"━━ {name} ({ptype}) ━━")
        wins_dir = Path(proot) / "windows"
        total = done = working = 0
        for d in sorted(wins_dir.iterdir()) if wins_dir.exists() else []:
            wt = d / "window.toml"
            if wt.exists():
                try:
                    wdata = tomllib.loads(wt.read_text(encoding="utf-8"))
                    w = wdata["window"]
                    total += 1
                    icon = {"done": "✅", "working": "🔄", "blocked": "⏸️",
                            "pending": "⏳", "error": "❌"}.get(w["state"], "•")
                    conv = _read_conv(Path(proot), d.name)
                    print(f"  {icon}  {w['id']:<18} {w['role']:<10} {w['state']:<8}"
                          f"({len(conv)}轮)")
                    if w["state"] == "done":
                        done += 1
                    elif w["state"] == "working":
                        working += 1
                except Exception:
                    pass
        print(f"  项目窗口: {total}（{done} done / {working} working）\n")
    return 0


# ── v0.5: window analyze（需求自主分析）+ import + template ──


def _yaml_val(stripped: str) -> str:
    """提取 YAML 值（容错：带引号/无引号/单引号）。"""
    val = stripped.split(":", 1)[1].strip() if ":" in stripped else ""
    val = val.strip().strip('"').strip("'")
    return val


REPLAY_YAML = """project_type: software
goal: "测试项目"
windows:
  - id: "win-dev-01"
    role: "开发"
    prompt: "你是开发者"
    depends_on: []
    budget:
      max_steps: 30
      max_cost_cny: 0.3
      provider: "deepseek"
    outputs: ["src/"]
    gate: "shared/gates/dev-gate.sh"
workflow_stages:
  - id: "impl"
    trigger: "project_start"
    windows: ["win-dev-01"]
    gate: "human:人类审核"
"""

ANALYZE_PROMPT = """你是一个项目需求分析师。现在有一个需求窗口和人类聊了几十轮，对话历史如下。
你的任务是分析这段对话，产出一份结构化的窗口配置 YAML。

要求：
1. 项目类型从对话中推断（software/writing/data/research/ops）
2. 项目目标用一句话描述
3. 根据对话中的需求复杂度，确定需要的窗口角色 + 数量（1-10 个）
4. 每个窗口必须有：id、role、prompt、depends_on、budget(max_steps,max_cost_cny,provider)、outputs、gate(.sh)
5. 窗口之间不要创建冗余角色——**role 值必须全局唯一**（不能有两个窗口都是"开发"）
6. workflow_stages 定义流转次序（id、trigger、windows、gate——human: 或 auto:）
7. 每个窗口的 gate：人类审核写 "human:..."; 自动校验写 "auto:脚本名.sh"（必须以 .sh 结尾）
7. depends 不得成环

输出格式（只输出 YAML，不要废话）："""


def validate_deploy_yaml(windows, stages):
    """v0.5 补充2: analyze 产出契约校验（复用 deploy 规则）。
    返回 (ok, error_msg)。"""
    if not windows or len(windows) > 10:
        return False, f"window count {len(windows)} not in 1..10"
    # WT7 (audit-fix): 缺 id 的 window 必须前置拦截——否则下方 w["id"] 下标 KeyError
    for i, w in enumerate(windows):
        if not isinstance(w, dict) or "id" not in w:
            return False, f"window #{i} missing id (got: {w!r})"
    roles = [str(w.get("role", "")).replace(" ", "").lower() for w in windows]
    if len(roles) != len(set(roles)):
        return False, "duplicate roles"
    # depends 无环（拓扑）
    deps = {w["id"]: w.get("depends_on", []) for w in windows}
    visiting, visited = set(), set()

    def has_cycle(node):
        if node in visiting:
            return True
        if node in visited:
            return False
        visiting.add(node)
        for d in deps.get(node, []):
            if has_cycle(d):
                return True
        visiting.discard(node)
        visited.add(node)
        return False

    for w in windows:
        if has_cycle(w["id"]):
            return False, f"cycle in depends: {w['id']}"
    # 契约字段（v1.1 补丁6）
    for w in windows:
        missing = [k for k in ("id", "role", "prompt", "budget", "outputs", "gate") if k not in w]
        if missing:
            return False, f"window {w.get('id','?')} missing {missing}"
        gate = w.get("gate", "")
        # v0.6.1: gate 宽容归一——LLM 常产出 "human review" / "auto:test" / "test.sh"
        # 无 .sh 且非 human → 自动补 .sh（"auto:test" → gate="auto:test.sh"）
        if not gate.startswith("human:") and not gate.endswith(".sh"):
            if gate.startswith("auto:"):
                w["gate"] = gate + ".sh"
            elif gate:  # 裸名 "test" → "test.sh"
                w["gate"] = gate + ".sh"
    for s in stages:
        for k in ("id", "trigger", "windows", "gate"):
            if k not in s:
                return False, f"stage missing {k}"
    return True, ""


# ── v0.7: function calling 结构化输出（替代 YAML 自由文本解析）──

FC_TOOL = {
    "type": "function",
    "function": {
        "name": "output_project_config",
        "description": "产出项目窗口配置（结构化 JSON）",
        "parameters": {
            "type": "object",
            "required": ["project_type", "goal", "windows", "workflow_stages"],
            "properties": {
                "project_type": {"type": "string",
                                 "enum": ["software", "writing", "data", "research", "ops"]},
                "goal": {"type": "string"},
                "windows": {"type": "array", "items": {
                    "type": "object",
                    "required": ["id", "role", "prompt", "budget", "outputs", "gate"],
                    "properties": {
                        "id": {"type": "string"},
                        "role": {"type": "string"},
                        "prompt": {"type": "string"},
                        "depends_on": {"type": "array", "items": {"type": "string"}},
                        "budget": {"type": "object",
                                   "properties": {"max_steps": {"type": "integer"},
                                                  "max_cost_cny": {"type": "number"},
                                                  "provider": {"type": "string"}},
                                   "required": ["max_steps", "max_cost_cny", "provider"]},
                        "outputs": {"type": "array", "items": {"type": "string"}},
                        "gate": {"type": "string"},
                    }}},
                "workflow_stages": {"type": "array", "items": {
                    "type": "object",
                    "required": ["id", "trigger", "windows", "gate"],
                    "properties": {
                        "id": {"type": "string"},
                        "trigger": {"type": "string"},
                        "windows": {"type": "array", "items": {"type": "string"}},
                        "parallel": {"type": "boolean"},
                        "gate": {"type": "string"},
                    }}},
            }
        }
    }
}

REPLAY_CONFIG_JSON = {
    "project_type": "software",
    "goal": "测试项目",
    "windows": [{"id": "win-dev-01", "role": "开发", "prompt": "你是开发者",
                 "depends_on": [], "budget": {"max_steps": 30, "max_cost_cny": 0.3,
                 "provider": "deepseek"}, "outputs": ["src/"],
                 "gate": "auto:shared/gates/dev-gate.sh"}],
    "workflow_stages": [{"id": "impl", "trigger": "project_start",
                         "windows": ["win-dev-01"], "gate": "human:人类审核"}],
}


def _fc_analyze(conv: list, provider: str = "deepseek") -> dict:
    """v0.7 S1: function calling 产出配置 JSON（real 模式）。返回 dict。

    P2-5 (audit-fix): 走 Agent.PROVIDERS 路由——不再硬编码
    DEEPSEEK_API_KEY + api.deepseek.com（provider=zhipu 时错发 deepseek 端点）。
    """
    import urllib.request
    if provider not in Agent.PROVIDERS:
        raise ValueError(f"unknown provider {provider!r} (analyze)")
    spec = Agent.PROVIDERS[provider]
    api_key = os.environ.get(spec["env_key"], "")
    if not api_key:
        raise RuntimeError(
            f"{spec['env_key']} not set (analyze real mode, provider={provider})"
        )
    history = "\n".join(json.dumps(e, ensure_ascii=False)[:500] for e in conv[-40:])
    body = json.dumps({
        "model": spec["model"],
        "messages": [{"role": "system", "content": ANALYZE_PROMPT},
                     {"role": "user", "content": history[:8000]}],
        "tools": [FC_TOOL],
        "tool_choice": {"type": "function", "function": {"name": "output_project_config"}},
        "temperature": 0.2,
    }).encode()
    req = urllib.request.Request(f"{spec['base_url']}/chat/completions",
                                 data=body, headers={"Content-Type": "application/json",
                                                     "Authorization": f"Bearer {api_key}"})
    with urllib.request.urlopen(req, timeout=120) as r:
        resp = json.loads(r.read().decode())
    msg = resp["choices"][0]["message"]
    tcs = msg.get("tool_calls") or []
    if not tcs:
        # 降级（补充5）：provider 没走 function calling → 回退内容解析
        return {"_fallback_text": msg.get("content", "")}
    args = json.loads(tcs[0]["function"]["arguments"])
    return args


def _yaml_analyze_fallback(conv: list, provider: str = "deepseek") -> dict:
    """v0.7 补充5: 降级路径——YAML 自由文本（provider 不支持 function calling 时）。

    P2-5 (audit-fix): 走 provider 路由（同 _fc_analyze）。
    """
    import urllib.request
    if provider not in Agent.PROVIDERS:
        raise ValueError(f"unknown provider {provider!r} (analyze)")
    spec = Agent.PROVIDERS[provider]
    api_key = os.environ.get(spec["env_key"], "")
    if not api_key:
        raise RuntimeError(
            f"{spec['env_key']} not set (analyze yaml fallback, provider={provider})"
        )
    history = "\n".join(json.dumps(e, ensure_ascii=False)[:500] for e in conv[-40:])
    body = json.dumps({
        "model": spec["model"],
        "messages": [{"role": "system", "content": ANALYZE_PROMPT},
                     {"role": "user", "content": history[:8000]}],
        "temperature": 0.2,
    }).encode()
    req = urllib.request.Request(f"{spec['base_url']}/chat/completions",
                                 data=body, headers={"Content-Type": "application/json",
                                                     "Authorization": f"Bearer {api_key}"})
    with urllib.request.urlopen(req, timeout=120) as r:
        resp = json.loads(r.read().decode())
    yaml_text = resp["choices"][0]["message"]["content"]
    import re
    m = re.search(r"```yaml\n(.*?)```", yaml_text, re.DOTALL)
    text = m.group(1) if m else yaml_text
    return {"_yaml_text": text}


def cmd_window_analyze(args):
    """v0.7: analyze 用 function calling 产出结构化 JSON → 直接建窗。
    降级：provider 不支持 → 回退 YAML（补充5）。"""
    root = PROJECTS_ROOT / args.project
    conv = _read_conv(root, args.id)
    if not conv:
        print(f"no conversation for '{args.id}'")
        return 1
    # P2-5 (audit-fix): analyze 也走 provider 路由——从 window.toml budget.provider 读
    try:
        _wt = root / "windows" / args.id / "window.toml"
        _cfg = tomllib.loads(_wt.read_text(encoding="utf-8"))
        analyze_provider = _cfg["budget"].get("provider", "deepseek")
    except Exception:
        analyze_provider = "deepseek"
    # replay 模式（补充2）
    if os.environ.get("AGENT_MODE", "real") == "replay":
        config = dict(REPLAY_CONFIG_JSON)
        print("[replay] using built-in config JSON")
    elif os.environ.get("FC_DISABLED") == "1":
        # 强制降级（补充5 测试用）
        print("WARN: function calling disabled — falling back to YAML mode")
        result = _yaml_analyze_fallback(conv, analyze_provider)
        if "_yaml_text" in result:
            wins = _parse_windows_shared(result["_yaml_text"])
            stages = _parse_stages_shared(result["_yaml_text"])
        else:
            print("ERROR: fallback YAML failed")
            return 1
        ok, err = validate_deploy_yaml(wins, stages)
        if not ok:
            print(f"analyze: INVALID YAML — {err}")
            return 1
        print(f"analyze: {len(wins)} windows, {len(stages)} stages (yaml fallback, valid)")
        _finish_analyze(root, args, wins, stages)
        return 0
    else:
        try:
            config = _fc_analyze(conv, analyze_provider)
        except RuntimeError as e:
            print(f"ERROR: {e}")
            return 1
        except Exception as e:
            print(f"ERROR: function calling failed ({e}) — falling back to YAML")
            result = _yaml_analyze_fallback(conv, analyze_provider)
            if "_yaml_text" in result:
                wins = _parse_windows_shared(result["_yaml_text"])
                stages = _parse_stages_shared(result["_yaml_text"])
            else:
                return 1
            ok, err = validate_deploy_yaml(wins, stages)
            if not ok:
                print(f"analyze: INVALID YAML — {err}")
                return 1
            _finish_analyze(root, args, wins, stages)
            return 0
        if "_fallback_text" in config:
            print("WARN: no tool_calls — falling back to YAML mode")
            result = _yaml_analyze_fallback(conv, analyze_provider)
            if "_yaml_text" in result:
                wins = _parse_windows_shared(result["_yaml_text"])
                stages = _parse_stages_shared(result["_yaml_text"])
            else:
                return 1
            ok, err = validate_deploy_yaml(wins, stages)
            if not ok:
                print(f"analyze: INVALID YAML — {err}")
                return 1
            _finish_analyze(root, args, wins, stages)
            return 0

    # function calling JSON 直接转 windows/stages
    wins = []
    for w in config.get("windows", []):
        budget = w.get("budget", {})
        wins.append({
            "id": w.get("id", ""), "role": w.get("role", ""),
            "prompt": w.get("prompt", ""), "depends_on": w.get("depends_on", []),
            "budget": budget, "outputs": w.get("outputs", []),
            "gate": w.get("gate", ""),
            "max_steps": budget.get("max_steps", 40),
            "max_cost": budget.get("max_cost_cny", 0.5),
            "provider": budget.get("provider", "deepseek"),
        })
    stages = config.get("workflow_stages", [])
    ok, err = validate_deploy_yaml(wins, stages)
    if not ok:
        print(f"analyze: INVALID config — {err}")
        return 1
    print(f"analyze: {len(wins)} windows, {len(stages)} stages (function calling, valid)")
    for w in wins:
        print(f"  - {w['id']} ({w['role']})")
    _finish_analyze(root, args, wins, stages)
    return 0


def _finish_analyze(root, args, wins, stages):
    """analyze 收尾：--confirm 写入对话；无论 confirm 与否都写 last_analyze.json（v0.9 补充3）。"""
    import json as J
    # v0.9: 快照供 deploy --last（valid=true）
    snap_dir = root / ".snapshots"
    snap_dir.mkdir(parents=True, exist_ok=True)
    (snap_dir / "last_analyze.json").write_text(
        J.dumps({"ts": now_iso(), "valid": True, "windows": wins,
                 "workflow_stages": stages}, ensure_ascii=False),
        encoding="utf-8")
    if args.confirm:
        config = {"windows": wins, "workflow_stages": stages}
        # v0.7.1: 用 ```json 标记（function calling 产出是 JSON 不是 YAML）
        content = f"```json\n{J.dumps(config, ensure_ascii=False, indent=2)}\n```"
        _append_conv(root, args.id, {"t": now_iso(), "role": "assistant",
                                     "content": content})
        print("analyze: confirmed — run 'workflow deploy' to create windows")


def _parse_windows_shared(text):
    """从 YAML 提取 windows（与 deploy 解析一致的极简解析器）。"""
    import re
    wins, current, in_budget = [], None, False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("workflow_stages:"):
            break
        if stripped.startswith("- id:"):
            if current:
                wins.append(current)
            current = {"id": _yaml_val(stripped)}
            in_budget = False
        elif current is not None:
            if re.match(r"^\s*role:", line):
                current["role"] = _yaml_val(stripped)
            elif re.match(r"^\s*prompt:", line):
                current["prompt"] = _yaml_val(stripped)
            elif re.match(r"^\s*depends_on:", line):
                current["depends_on"] = re.findall(r'"(win-[^"]+)"', stripped)
            elif re.match(r"^\s*outputs:", line):
                current["outputs"] = re.findall(r'"(shared/[^"]+|src/|web/|docs/)"', stripped)
            elif "budget:" in stripped:
                in_budget = True
            elif in_budget and "max_steps:" in stripped:
                current["budget"] = {"max_steps": int(stripped.split(":")[1].strip())}
            elif in_budget and "max_cost_cny:" in stripped:
                current["budget"]["max_cost_cny"] = float(stripped.split(":")[1].strip())
            elif in_budget and "provider:" in stripped:
                current["budget"]["provider"] = stripped.split(":")[1].strip().strip('"')
            elif re.match(r"^\s*gate:", line):
                current["gate"] = stripped.split(":", 1)[1].strip().strip('"')
    if current:
        wins.append(current)
    return wins


def _parse_stages_shared(text):
    import re
    # 只解析 workflow_stages: 之后的部分（v0.5: 防 windows 段的 - id 混入）
    if "workflow_stages:" in text:
        text = text.split("workflow_stages:", 1)[1]
    stages, current = [], None
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("- id:"):
            if current:
                stages.append(current)
            current = {"id": _yaml_val(stripped)}
        elif current is not None:
            if re.match(r"^\s*trigger:", line):
                current["trigger"] = stripped.split(":")[1].strip().strip('"')
            elif re.match(r"^\s*windows:", line):
                current["windows"] = re.findall(r'"(win-[^"]+)"', stripped)
            elif re.match(r"^\s*gate:", line):
                current["gate"] = stripped.split(":", 1)[1].strip().strip('"')
    if current:
        stages.append(current)
    return stages


def cmd_window_import(args):
    """v0.5 S3: 导入外部对话（JSONL / OpenAI messages 数组 / chat completion）。"""
    import json as J
    root = PROJECTS_ROOT / args.project
    src = Path(args.source)
    if not src.exists():
        print(f"source not found: {src}")
        return 1
    raw = src.read_text(encoding="utf-8")
    # 支持 JSONL（多行 JSON）与 JSON 数组
    try:
        data = J.loads(raw)
    except J.JSONDecodeError:
        data = [J.loads(line) for line in raw.splitlines() if line.strip()]
    entries = []
    # 判断格式：list = OpenAI messages 数组；dict with messages = chat completion；
    # 单个 dict（无 messages）= 单条消息（WT15 往返场景）
    if isinstance(data, dict):
        data = data["messages"] if "messages" in data else [data]
    for m in data:
        # WT15 (audit-fix): 接受 summary role（导出→导入往返不丢摘要）；
        # 保留原时间戳（t/ts/timestamp），不再强制 now_iso() 改写历史。
        if isinstance(m, dict) and "role" in m and m["role"] in (
                "system", "user", "assistant", "tool", "summary"):
            entry = {"t": m.get("t") or m.get("ts") or m.get("timestamp") or now_iso(),
                     "role": m["role"], "content": m.get("content") or ""}
            if m.get("meta"):
                entry["meta"] = m["meta"]
            if m.get("tool_calls"):
                tcs = []
                for tc in m["tool_calls"]:
                    fn = tc.get("function", {})
                    try:
                        a = J.loads(fn.get("arguments", "{}"))
                    except J.JSONDecodeError:
                        a = {}
                    tcs.append({"name": fn.get("name", ""), "args": a})
                entry["tool_calls"] = tcs
            entries.append(entry)
    if not entries:
        print("import: no valid messages found")
        return 1
    win_id = _win_id(root, args.name)
    wdir = root / "windows" / win_id
    wdir.mkdir(parents=True)
    conv_path = wdir / "conversation.jsonl"
    with open(conv_path, "w", encoding="utf-8") as f:
        for e in entries:
            f.write(J.dumps(e, ensure_ascii=False) + "\n")
    # 创建 window.toml
    (wdir / "window.toml").write_text(
        WINDOW_TOML_TEMPLATE.format(win_id=win_id, name=args.name, role=args.role,
                                    prompt=json.dumps(args.prompt or "", ensure_ascii=False), created_by="import",
                                    created_at=now_iso(), max_steps=40, max_cost_cny=0.5,
                                    provider="deepseek"), encoding="utf-8")
    # 跑一次压缩（长对话）
    if len(entries) > 20 and os.environ.get("AGENT_MODE", "real") == "replay":
        CompressionEngine(root, win_id).compress()
        print(f"import: compressed {len(entries)} → summaries")
    # 提取文件引用警告（补充4）
    import re
    refs = set(re.findall(r'"(?:shared/|src/|web/|docs/)?[a-zA-Z0-9_\-/]+\.(?:rs|md|toml|py|js)"',
                          src.read_text(encoding="utf-8")))
    missing = [r for r in refs if not (root / r).exists()]
    if missing:
        print(f"import: WARN {len(missing)} referenced files missing: {missing[:5]}")
    print(f"import: {len(entries)} messages → window '{win_id}'")
    return 0


TEMPLATES_DIR = PROJECTS_ROOT / "templates"


def cmd_template_save(args):
    root = PROJECTS_ROOT / args.project
    wt = root / "windows" / args.from_window / "window.toml"
    if not wt.exists():
        print(f"window '{args.from_window}' not found")
        return 1
    data = tomllib.loads(wt.read_text(encoding="utf-8"))
    tpl_path = TEMPLATES_DIR / f"{args.name}.toml"
    if tpl_path.exists() and not args.force:
        print(f"template '{args.name}' exists — use --force to overwrite")
        return 1
    TEMPLATES_DIR.mkdir(parents=True, exist_ok=True)
    tpl = {"window": {"role": data["window"]["role"], "prompt": data["window"].get("prompt", "")},
           "budget": data["budget"]}
    tpl_path.write_text(tomllib_dumps(tpl), encoding="utf-8")
    print(f"template saved: {tpl_path.name}")
    return 0


def tomllib_dumps(data):
    """极简 toml 序列化（模板用）。"""
    lines = ["[window]"]
    lines.append(f'role = "{data["window"]["role"]}"')
    lines.append(f'prompt = "{data["window"]["prompt"]}"')
    lines.append("\n[budget]")
    lines.append(f'max_steps = {data["budget"]["max_steps"]}')
    lines.append(f'max_cost_cny = {data["budget"]["max_cost_cny"]}')
    lines.append(f'provider = "{data["budget"]["provider"]}"')
    return "\n".join(lines) + "\n"


def cmd_template_list(args):
    if not TEMPLATES_DIR.exists():
        print("(no templates)")
        return 0
    for t in sorted(TEMPLATES_DIR.glob("*.toml")):
        print(f"  {t.stem}")
    return 0


def cmd_template_create(args):
    """window create --template <name>。"""
    tpl_path = TEMPLATES_DIR / f"{args.name}.toml"
    if not tpl_path.exists():
        print(f"template '{args.name}' not found")
        return 1
    data = tomllib.loads(tpl_path.read_text(encoding="utf-8"))
    win_id = _win_id(PROJECTS_ROOT / args.project, args.win_name or args.name)
    wdir = PROJECTS_ROOT / args.project / "windows" / win_id
    wdir.mkdir(parents=True)
    (wdir / "window.toml").write_text(
        WINDOW_TOML_TEMPLATE.format(
            win_id=win_id, name=args.win_name or args.name, role=data["window"]["role"],
            prompt=json.dumps(data["window"]["prompt"], ensure_ascii=False), created_by="template",
            created_at=now_iso(), max_steps=data["budget"]["max_steps"],
            max_cost_cny=data["budget"]["max_cost_cny"], provider=data["budget"]["provider"]),
        encoding="utf-8")
    print(f"created window '{win_id}' from template '{args.name}'")
    return 0


# ── v0.6: 共享层冲突仲裁（§8.4 第二层）──────────────────────

def line_diff_sets(new_text: str, base_text: str):
    """v0.6 补充2: 行级 diff——返回被修改的行号集合。
    简化实现：逐行比较 new vs base，不同的行号加入集合。"""
    new_lines = new_text.split("\n")
    base_lines = base_text.split("\n")
    changed = set()
    for i in range(max(len(new_lines), len(base_lines))):
        nl = new_lines[i] if i < len(new_lines) else ""
        bl = base_lines[i] if i < len(base_lines) else ""
        if nl != bl:
            changed.add(i)
    return changed


def merge_two(path: str, version_a: str, version_b: str, base: str):
    """v0.6 补充2: 行级自动合并。返回 (merged_text|None, conflict: bool, detail)。"""
    a_changed = line_diff_sets(version_a, base)
    b_changed = line_diff_sets(version_b, base)
    if a_changed & b_changed:
        return None, True, f"conflict at lines {sorted(a_changed & b_changed)}"
    # 无交集 → 合并：以 base 为底，应用 A 和 B 的改动（行号不重叠，顺序取 A 后 B）
    a_lines = version_a.split("\n")
    b_lines = version_b.split("\n")
    # 简单策略：A 改动行取 A，B 改动行取 B，其余取 base
    base_lines = base.split("\n")
    merged = []
    max_len = max(len(a_lines), len(b_lines), len(base_lines))
    for i in range(max_len):
        bl = base_lines[i] if i < len(base_lines) else ""
        al = a_lines[i] if i < len(a_lines) else bl
        bl2 = b_lines[i] if i < len(b_lines) else bl
        if i in a_changed:
            merged.append(al)
        elif i in b_changed:
            merged.append(bl2)
        else:
            merged.append(bl)
    return "\n".join(merged), False, f"merged (A@{len(a_changed)} lines, B@{len(b_changed)} lines)"


def cmd_conflict_list(args):
    """列出项目所有共享层冲突（两窗口声明同路径）。"""
    root = PROJECTS_ROOT / args.project
    path_owners = {}
    for d in sorted((root / "windows").iterdir()) if (root / "windows").exists() else []:
        wt = d / "window.toml"
        if wt.exists():
            data = tomllib.loads(wt.read_text(encoding="utf-8"))
            for out in data.get("outputs", {}).get("files", []):
                path_owners.setdefault(out, []).append(d.name)
    conflicts = {p: owners for p, owners in path_owners.items() if len(owners) > 1}
    if not conflicts:
        print("(no conflicts)")
        return 0
    for p, owners in conflicts.items():
        print(f"  {p} ← {owners}")
    return 0


def cmd_conflict_resolve(args):
    """v0.6 补充5: 人类仲裁——保留 win_id 版本，清冲突标记，被否窗口 blocked。"""
    root = PROJECTS_ROOT / args.project
    # 冲突路径是声明路径（文件可能未产出）——只处理声明，不要求文件存在
    target = root / args.path
    if not target.exists() and not any(
            args.path in (d / "window.toml").read_text(encoding="utf-8")
            for d in (root / "windows").iterdir() if (d / "window.toml").exists()):
        print(f"conflict path not declared: {args.path}")
        return 1
    # 找声明该路径的窗口
    owners = []
    for d in sorted((root / "windows").iterdir()) if (root / "windows").exists() else []:
        wt = d / "window.toml"
        if wt.exists():
            data = tomllib.loads(wt.read_text(encoding="utf-8"))
            if args.path in data.get("outputs", {}).get("files", []):
                owners.append(d.name)
    if args.keep not in owners:
        print(f"window '{args.keep}' does not own {args.path} (owners: {owners})")
        return 1
    # 被否窗口 blocked
    for wid in owners:
        if wid != args.keep:
            _set_state(root, wid, "blocked")
            print(f"  {wid} → blocked (产出被覆盖需重做)")
    # 清冲突标记：被否窗口的 outputs 移除该路径（或标注 resolved）
    for wid in owners:
        wt = root / "windows" / wid / "window.toml"
        text = wt.read_text(encoding="utf-8")
        if wid != args.keep:
            # 从 outputs.files 移除冲突路径
            text = text.replace(f'  "{args.path}",\n', "").replace(f'  "{args.path}"\n', "")
        wt.write_text(text, encoding="utf-8")
    print(f"conflict resolved: keep {args.keep} for {args.path}")
    return 0


def cmd_workflow_deploy(args):
    """v0.3 S2: 解析需求窗口配置（对话 JSON/YAML 或 --last 快照）→ 自动建窗。"""
    import re
    root = PROJECTS_ROOT / args.project

    # v0.9 补充3: deploy --last 用 .snapshots/last_analyze.json（不重新 analyze）
    if args.last:
        snap = root / ".snapshots" / "last_analyze.json"
        if not snap.exists():
            print("no last_analyze.json — run 'window analyze' first")
            return 1
        data = json.loads(snap.read_text(encoding="utf-8"))
        if not data.get("valid"):
            print(f"last analyze invalid: {data.get('error', 'unknown')}")
            return 1
        wins = data["windows"]
        stages = data["workflow_stages"]
        ok, err = validate_deploy_yaml(wins, stages)
        if not ok:
            print(f"deploy --last: INVALID — {err}")
            return 1
        return _deploy_windows_stages(root, args, wins, stages)

    conv = _read_conv(root, args.window)
    # v0.7: 找最后一条含配置的 assistant 消息——支持 JSON（function calling 产出）或 YAML 块
    yaml_text = None
    json_config = None
    for e in reversed(conv):
        if e.get("role") == "assistant" and e.get("content"):
            content = e["content"]
            # JSON 检测优先（function calling 产出；兼容 ```json 或 ```yaml 包裹的 JSON）
            inner = content
            m0 = re.search(r"```(?:json|yaml)\n(.*?)```", content, re.DOTALL)
            if m0:
                inner = m0.group(1)
            try:
                parsed = json.loads(inner)
                if isinstance(parsed, dict) and "windows" in parsed:
                    json_config = parsed
                    break
            except (json.JSONDecodeError, ValueError):
                pass
            m = re.search(r"```yaml\n(.*?)```", content, re.DOTALL)
            if m:
                yaml_text = m.group(1)
                break
    if not yaml_text and not json_config:
        print(f"no config block found in '{args.window}' conversation")
        return 1

    if json_config is not None:
        # v0.7: JSON 直接转 windows/stages（function calling 产出）
        wins = []
        for w in json_config.get("windows", []):
            budget = w.get("budget", {})
            wins.append({
                "id": w.get("id", ""), "role": w.get("role", ""),
                "prompt": w.get("prompt", ""), "depends_on": w.get("depends_on", []),
                "budget": budget, "outputs": w.get("outputs", []),
                "gate": w.get("gate", ""),
                "max_steps": budget.get("max_steps", 40),
                "max_cost": budget.get("max_cost_cny", 0.5),
                "provider": budget.get("provider", "deepseek"),
            })
        stages = json_config.get("workflow_stages", [])
        ok, err = validate_deploy_yaml(wins, stages)
        if not ok:
            print(f"deploy: INVALID config — {err}")
            return 1
        _deploy_windows_stages(root, args, wins, stages)
        return 0

    # 极简 YAML 解析（只支持本契约结构）
    def parse_windows(text):
        wins = []
        current = None
        in_budget = False
        for line in text.splitlines():
            stripped = line.strip()
            if stripped.startswith("workflow_stages:"):
                break  # v0.3.1: 只解析 windows 段（避免把 stages 的 - id 当窗口）
            if stripped.startswith("- id:"):
                if current:
                    wins.append(current)
                current = {"id": _yaml_val(stripped)}
                in_budget = False
            elif current is not None:
                if re.match(r"^\s*role:", line):
                    current["role"] = _yaml_val(stripped)
                elif re.match(r"^\s*prompt:", line):
                    current["prompt"] = _yaml_val(stripped)
                elif re.match(r"^\s*depends_on:", line):
                    deps = re.findall(r'"(win-[^"]+)"', stripped)
                    current["depends_on"] = deps
                elif re.match(r"^\s*outputs:", line):
                    outs = re.findall(r'"(shared/[^"]+|src/|web/|docs/)"', stripped)
                    current["outputs"] = outs
                elif "budget:" in stripped:
                    in_budget = True
                elif in_budget and "max_steps:" in stripped:
                    current["max_steps"] = int(stripped.split(":")[1].strip())
                elif in_budget and "max_cost_cny:" in stripped:
                    current["max_cost"] = float(stripped.split(":")[1].strip())
                elif in_budget and "provider:" in stripped:
                    current["provider"] = stripped.split(":")[1].strip().strip('"')
                elif re.match(r"^\s*gate:", line):
                    g = stripped.split(":", 1)[1].strip().strip('"')
                    current["gate"] = g
        if current:
            wins.append(current)
        return wins

    def parse_stages(text):
        stages = []
        current = None
        for line in text.splitlines():
            stripped = line.strip()
            if stripped.startswith("- id:"):
                if current:
                    stages.append(current)
                current = {"id": _yaml_val(stripped)}
            elif current is not None:
                if re.match(r"^\s*trigger:", line):
                    current["trigger"] = stripped.split(":")[1].strip().strip('"')
                elif re.match(r"^\s*windows:", line):
                    current["windows"] = re.findall(r'"(win-[^"]+)"', stripped)
                elif re.match(r"^\s*gate:", line):
                    current["gate"] = stripped.split(":", 1)[1].strip().strip('"')
        if current:
            stages.append(current)
        return stages

    # v0.6: 复用共享解析器（含 workflow_stages 截断 + 容错），弃用嵌套 parse_*
    windows = _parse_windows_shared(yaml_text)
    stages = _parse_stages_shared(yaml_text)
    if not windows:
        print("no windows parsed from YAML")
        return 1

    # 建窗口 + 写 workflow（v0.7: 抽成共享函数，JSON/YAML 路径复用）
    return _deploy_windows_stages(root, args, windows, stages)


def _deploy_windows_stages(root, args, windows, stages):
    """v0.7: deploy 主体——建窗口 + 写 outputs/gate + 写 workflow_stages。"""
    for w in windows:
        if not all(k in w for k in ("id", "role", "prompt")):
            print(f"window {w.get('id','?')} missing id/role/prompt")
            return 1
        gate = w.get("gate", "")
        # budget 兼容嵌套 dict（共享解析器）与顶层字段（旧解析器）
        budget = w.get("budget") if isinstance(w.get("budget"), dict) else {}
        has_budget = "max_steps" in budget or "max_steps" in w
        if not has_budget or "outputs" not in w or (
                not gate.startswith("human:") and not gate.endswith(".sh")):
            print(f"window {w['id']} missing budget/outputs/gate.sh (v0.3.1 补充1)")
            return 1
        max_steps = budget.get("max_steps", w.get("max_steps", 40))
        max_cost = budget.get("max_cost_cny", w.get("max_cost", 0.5))
        provider = budget.get("provider", w.get("provider", "deepseek"))
        win_id = w["id"].removeprefix("win-")
        # create 实际生成的目录名（_win_id 可能加 -2 后缀）
        final_win = _win_id(root, win_id)
        rc = cmd_window_create(type("A", (), {
            "project": args.project, "name": win_id, "role": w["role"],
            "prompt": w["prompt"], "max_steps": max_steps,
            "max_cost": max_cost, "provider": provider}))
        if rc != 0:
            return rc
        # 写 outputs/gate 到 window.toml（用 final_win——create 实际生成的目录名）
        wt = root / "windows" / final_win / "window.toml"
        text = wt.read_text(encoding="utf-8")
        outs = ",\n".join(f'  "{o}"' for o in w.get("outputs", []))
        text = text.replace('files = []', f'files = [\n{outs}\n]')
        # v0.7: gate 存脚本路径（去 auto:/human: 前缀）；human 存 "human:描述"
        raw_gate = w["gate"]
        if raw_gate.startswith("auto:"):
            gate_path = raw_gate[5:]
            stored_gate = gate_path
        elif raw_gate.startswith("human:"):
            gate_path = None
            stored_gate = raw_gate
        else:
            gate_path = raw_gate
            stored_gate = raw_gate
        text = text.replace('gate = ""', f'gate = "{stored_gate}"')
        wt.write_text(text, encoding="utf-8")
        # 生成 gate 占位脚本（auto 才需要）
        if gate_path:
            gp = root / gate_path
            gp.parent.mkdir(parents=True, exist_ok=True)
            if not gp.exists():
                gp.write_text("#!/bin/bash\necho VERIFY_PASS\nexit 0\n", encoding="utf-8")
                os.chmod(gp, 0o755)
        print(f"deploy: created {w['id']}")

    # 写 workflow_stages 到 project.toml
    pt = root / "project.toml"
    text = pt.read_text(encoding="utf-8")
    # v0.6: stage 引用的窗口 id 统一加 win- 前缀（对齐实际窗口目录名）
    actual_ids = {f"win-{w['id'].removeprefix('win-')}" for w in windows}
    actual_list = sorted(actual_ids)
    lines = []
    prev_id = None
    for s in stages:
        # dogfooding v1.0.1: trigger 规范化——auto/空 → 首个 project_start / 后续 prev:done
        trig = s.get("trigger", "")
        if trig in ("", "auto", "human"):
            trig = "project_start" if prev_id is None else f"{prev_id}:done"
        s_wins = []
        for w in s.get("windows", []):
            aw = f"win-{w.removeprefix('win-')}"
            if aw in actual_ids or (root / "windows" / aw).exists():
                s_wins.append(aw)
            else:
                s_wins.append(w)  # 保持原样（引擎会报 missing）
        # dogfooding v1.0.1: windows 引用自动补全（LLM 常漏）——按序分配未用窗口
        if not s_wins and actual_list:
            s_wins = [actual_list.pop(0)]
        wins = ", ".join(f'"{w}"' for w in s_wins)
        gate_spec = s.get("gate", "")
        lines.append(f'[[workflow.stages]]\nid = "{s["id"]}"\ntrigger = "{trig}"\n'
                     f'windows = [{wins}]\ngate = "{gate_spec}"\n')
        # v1.0.1 dogfooding: 为 stage 的 auto gate 生成占位脚本（引擎跑 stage gate）
        if gate_spec.startswith("auto:"):
            gpath = gate_spec[5:]
            # WT11 (audit-fix): 越权 gate 路径不生成脚本（防目录穿越写入）
            if not WorkflowEngine._gate_path_ok(root, gpath):
                print(f"WARN: gate path denied (WT11): {gpath!r} — skip placeholder")
            else:
                sgp = root / gpath
                sgp.parent.mkdir(parents=True, exist_ok=True)
                if not sgp.exists():
                    sgp.write_text("#!/bin/bash\necho VERIFY_PASS\nexit 0\n", encoding="utf-8")
                    os.chmod(sgp, 0o755)
        prev_id = s["id"]
    text = text.replace("[workflow]\nstages = []",
                        "[workflow]\n" + "\n".join(lines).rstrip())
    pt.write_text(text, encoding="utf-8")
    print(f"deploy: wrote {len(stages)} stages to project.toml")
    return 0


def cmd_framework_check(args):
    root = PROJECTS_ROOT / args.project
    if not (root / "project.toml").exists():
        print(f"project '{args.project}' not found")
        return 1
    return 0 if FrameworkCheck(root).run() else 1


def main():
    p = argparse.ArgumentParser(prog="codex", description=f"窗口群框架 v{VERSION}")
    sub = p.add_subparsers(dest="cmd")

    # project
    sp = sub.add_parser("project", help="项目管理")
    sp2 = sp.add_subparsers(dest="sub")
    c = sp2.add_parser("create"); c.add_argument("name"); c.add_argument("--type", default="software")
    c.set_defaults(fn=cmd_project_create)
    c = sp2.add_parser("open"); c.add_argument("name", nargs="?")
    c.set_defaults(fn=cmd_project_open)
    c = sp2.add_parser("status"); c.add_argument("name", nargs="?")
    c.set_defaults(fn=cmd_project_status)

    # window
    sp = sub.add_parser("window", help="窗口管理")
    sp2 = sp.add_subparsers(dest="sub")
    c = sp2.add_parser("list"); c.add_argument("project")
    c.set_defaults(fn=cmd_window_list)
    c = sp2.add_parser("create"); c.add_argument("project")
    c.add_argument("--name", required=True); c.add_argument("--role", default="generic")
    c.add_argument("--prompt", default="")
    c.add_argument("--max-steps", type=int, default=40)
    c.add_argument("--max-cost", type=float, default=0.5)
    c.add_argument("--provider", default="deepseek")
    c.set_defaults(fn=cmd_window_create)
    c = sp2.add_parser("start"); c.add_argument("project"); c.add_argument("id")
    c.add_argument("--goal", default="")
    c.add_argument("--max-turns", type=int, default=12)
    c.set_defaults(fn=cmd_window_start)
    # v0.9: window run（start + 实时 StepLog）
    c = sp2.add_parser("run"); c.add_argument("project"); c.add_argument("id")
    c.add_argument("--goal", default="")
    # WT4 (audit-fix): 默认 None——未显式指定时尊重 window.toml budget.max_steps
    c.add_argument("--max-turns", type=int, default=None)
    c.add_argument("--verbose", action="store_true")
    c.set_defaults(fn=cmd_window_run)
    # v0.9: window resume（断点续传）
    c = sp2.add_parser("resume"); c.add_argument("project"); c.add_argument("id")
    c.add_argument("--goal", default="")
    c.add_argument("--max-turns", type=int, default=12)
    c.set_defaults(fn=cmd_window_resume)
    for sub_cmd, fn in [("stop", cmd_window_stop)]:
        c = sp2.add_parser(sub_cmd); c.add_argument("project"); c.add_argument("id")
        c.set_defaults(fn=fn)
    c = sp2.add_parser("delete"); c.add_argument("project"); c.add_argument("id")
    c.add_argument("--hard", action="store_true")
    c.set_defaults(fn=cmd_window_delete)
    c = sp2.add_parser("restore"); c.add_argument("project"); c.add_argument("id")
    c.set_defaults(fn=cmd_window_restore)
    c = sp2.add_parser("export"); c.add_argument("project"); c.add_argument("id")
    c.add_argument("--format", choices=["markdown", "json"], default="markdown")
    c.add_argument("--output", default="")
    c.add_argument("--compress", action="store_true")   # v0.5: 只输出摘要+热层
    c.add_argument("--full", action="store_true")       # v0.5: 完整对话（默认）
    c.set_defaults(fn=cmd_window_export)
    c = sp2.add_parser("analyze"); c.add_argument("project"); c.add_argument("id")
    c.add_argument("--confirm", action="store_true")    # v0.5: 确认后写入产出
    c.set_defaults(fn=cmd_window_analyze)
    c = sp2.add_parser("import"); c.add_argument("project")
    c.add_argument("--name", required=True); c.add_argument("--role", default="imported")
    c.add_argument("--prompt", default=""); c.add_argument("--source", required=True)
    c.set_defaults(fn=cmd_window_import)
    c = sp2.add_parser("snapshot"); c.add_argument("project"); c.add_argument("id")
    c.set_defaults(fn=cmd_window_snapshot)
    c = sp2.add_parser("rollback"); c.add_argument("project"); c.add_argument("id")
    c.add_argument("--to", required=True)
    c.set_defaults(fn=cmd_window_rollback)
    c = sp2.add_parser("snapshot-list"); c.add_argument("project"); c.add_argument("id")
    c.set_defaults(fn=cmd_window_snapshot_list)
    c = sp2.add_parser("status"); c.add_argument("project"); c.add_argument("id")
    c.set_defaults(fn=cmd_window_status)
    c = sp2.add_parser("compress"); c.add_argument("project"); c.add_argument("id")
    c.set_defaults(fn=cmd_window_compress)

    # framework
    sp = sub.add_parser("framework", help="框架自检")
    sp2 = sp.add_subparsers(dest="sub")
    c = sp2.add_parser("check"); c.add_argument("project")
    c.set_defaults(fn=cmd_framework_check)

    # workflow（v0.3）
    sp = sub.add_parser("workflow", help="工作流")
    sp2 = sp.add_subparsers(dest="sub")
    c = sp2.add_parser("start"); c.add_argument("project")
    c.add_argument("--max-rounds", type=int, default=20)
    c.set_defaults(fn=cmd_workflow_start)
    c = sp2.add_parser("watch"); c.add_argument("project")
    c.add_argument("--timeout", type=int, default=30)
    c.add_argument("--interval", type=int, default=2)
    c.add_argument("--max-rounds", type=int, default=20)
    c.set_defaults(fn=cmd_workflow_watch)
    c = sp2.add_parser("gate"); c.add_argument("project"); c.add_argument("stage")
    c.add_argument("--approve", action="store_true")
    c.add_argument("--reject", action="store_true")
    c.set_defaults(fn=cmd_workflow_gate)
    # v0.9: workflow retry（只重置指定 stage）
    c = sp2.add_parser("retry"); c.add_argument("project"); c.add_argument("stage")
    c.set_defaults(fn=cmd_workflow_retry)
    c = sp2.add_parser("deploy"); c.add_argument("project"); c.add_argument("window", nargs="?")
    c.add_argument("--last", action="store_true")   # v0.9: 用 last_analyze.json
    c.set_defaults(fn=cmd_workflow_deploy)

    # v0.9: 全局 status（聚合所有项目）
    c = sub.add_parser("status")
    c.set_defaults(fn=cmd_status)

    # template（v0.5）
    sp = sub.add_parser("template", help="窗口模板")
    sp2 = sp.add_subparsers(dest="sub")
    c = sp2.add_parser("save"); c.add_argument("name")
    c.add_argument("--project", required=True); c.add_argument("--from-window", required=True)
    c.add_argument("--force", action="store_true")
    c.set_defaults(fn=cmd_template_save)
    c = sp2.add_parser("list")
    c.set_defaults(fn=cmd_template_list)
    c = sp2.add_parser("create"); c.add_argument("name")
    c.add_argument("--project", required=True); c.add_argument("--win-name", default="")
    c.set_defaults(fn=cmd_template_create)

    # conflict（v0.6）
    sp = sub.add_parser("conflict", help="共享层冲突仲裁")
    sp2 = sp.add_subparsers(dest="sub")
    c = sp2.add_parser("list"); c.add_argument("project")
    c.set_defaults(fn=cmd_conflict_list)
    c = sp2.add_parser("resolve"); c.add_argument("project")
    c.add_argument("path"); c.add_argument("--keep", required=True)
    c.set_defaults(fn=cmd_conflict_resolve)

    args = p.parse_args()
    if not hasattr(args, "fn"):
        p.print_help()
        return 1
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
