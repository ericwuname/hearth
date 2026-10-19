#!/usr/bin/env python3
"""本地（**不依赖 VM**）评测驱动（M0-6 / D-228）。

**为什么需要它**：`bench/runner.py` 与 `bench/replay.py` 都 SSH 到一台 VM
（`192.168.220.131`）并在上面跑 service + 判据。后果有两个：
  1. **用户在自己机器上跑不了**评测 ⇒ "可上手试用"无从谈起；
  2. 判定/工作区路径被 VM 的形状绑死（`/home/wutao/codex_work/sessions/<sid>`）。

本脚本把这条路**原样搬回本地**：
  · 在本地起 `service`（`target/debug/service[.exe]`），只绑回环；
  · 会话工作区 = **service 进程的 cwd** + `sessions/<sid>`（源码事实，见
    `agent-runtime/src/session.rs` 的 `workspace_dir`）⇒ 不需要猜路径、不需要 SSH；
  · `provider=replay` + `REPLAY_DIR=bench/replay/fixtures` ⇒ **零成本、确定性**
    （录制里的 VM 绝对路径由 `llm-replay::portable_paths` 重写到当前工作区）；
  · 判据走 `bench/verify_exec.py` 的**本地传输层**（唯一实现，和远端同契约）。

**关于 OPENAI_API_KEY**：service 启动时**硬要求**它（fail-closed，禁占位 key 静默降级）。
回放模式**从不调用 OpenAI**，所以本脚本在缺 key 时**显式**注入一个占位值并通过启动门槛——
这不违反那条红线的前提是：① 只允许 `provider=replay`；② 结果里记 `openai_key_injected=true`；
③ 启动日志里能看到它是占位。**换别的 provider 会直接拒绝启动**，不会静默降级。

用法：
  python bench/run_local.py                     # 全部 replay 录制
  python bench/run_local.py --only T03 --limit 2
"""
import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))

import judge  # noqa: E402
import verify_exec  # noqa: E402

DEFAULT_WORK = REPO / "target" / "local-run"
DEFAULT_FIXTURES = HERE / "replay" / "fixtures"
DEFAULT_OUT = HERE / "results" / "raw" / "local-replay.jsonl"
POST_TERMINAL = ("done", "completed", "error", "cancelled")


def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


# ---------------------------------------------------------------- .env 读取
def load_dotenv(path) -> dict:
    """极简 .env 解析（不引依赖）。只做 `KEY=VALUE`，忽略注释/空行/引号。"""
    out = {}
    p = pathlib.Path(path)
    if not p.is_file():
        return out
    for line in p.read_text(encoding="utf-8", errors="replace").splitlines():
        s = line.strip()
        if not s or s.startswith("#") or "=" not in s:
            continue
        k, v = s.split("=", 1)
        k = k.strip()
        v = v.strip().strip('"').strip("'")
        if k:
            out[k] = v
    return out


# ---------------------------------------------------------------- HTTP
def api(base, method, path, data=None, timeout=20):
    req = urllib.request.Request(f"{base}{path}", method=method)
    req.add_header("Content-Type", "application/json")
    if data is not None:
        req.data = json.dumps(data).encode()
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        raw = resp.read().decode(errors="replace")
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return raw


def wait_ready(base, secs=45):
    deadline = time.time() + secs
    last = None
    while time.time() < deadline:
        try:
            api(base, "GET", "/readyz", timeout=3)
            return True
        except Exception as e:  # noqa: BLE001（启动期任何异常都只是"还没好"）
            last = e
            time.sleep(0.5)
    log(f"service 未就绪：{last}")
    return False


# ---------------------------------------------------------------- 服务进程
class LocalService:
    def __init__(self, bin_path, work, port, replay_dir, provider, env_extra):
        self.bin = pathlib.Path(bin_path)
        self.work = pathlib.Path(work)
        self.port = port
        self.replay_dir = pathlib.Path(replay_dir)
        self.provider = provider
        self.env_extra = env_extra
        self.proc = None
        self.openai_key_injected = False
        self.log_path = self.work / "service.log"

    def start(self):
        if not self.bin.is_file():
            raise SystemExit(
                f"找不到 service 二进制：{self.bin}\n先跑：cargo build -p service"
            )
        self.work.mkdir(parents=True, exist_ok=True)
        (self.work / "memory").mkdir(parents=True, exist_ok=True)

        env = dict(os.environ)
        env.update(self.env_extra)
        env.setdefault("ALLOW_NO_AUTH", "1")  # 无鉴权；service 会强制只绑回环
        env["PORT"] = str(self.port)
        env["REPLAY_DIR"] = str(self.replay_dir)
        env.setdefault("MEMORY_DIR", str(self.work / "memory"))

        if not env.get("OPENAI_API_KEY"):
            if self.provider != "replay":
                raise SystemExit(
                    f"provider={self.provider} 需要真实 OPENAI_API_KEY（或对应的 provider key）；"
                    "本脚本**只**在 provider=replay（不出网）时才注入占位值来通过启动门槛。"
                )
            env["OPENAI_API_KEY"] = "local-replay-placeholder-never-used"
            self.openai_key_injected = True
            log(
                "注意：OPENAI_API_KEY 未设置 ⇒ 已注入**占位值**仅为通过 service 启动门槛；"
                "provider=replay 不会出网调用它。"
            )

        self.log_fh = open(self.log_path, "ab")
        self.proc = subprocess.Popen(
            [str(self.bin)],
            cwd=str(self.work),
            env=env,
            stdout=self.log_fh,
            stderr=subprocess.STDOUT,
        )
        log(f"service 已启动 pid={self.proc.pid} 端口={self.port} cwd={self.work}")
        return wait_ready(f"http://127.0.0.1:{self.port}")

    def stop(self):
        if self.proc is None:
            return
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=10)
            log(f"service 已停止 pid={self.proc.pid}")
        try:
            self.log_fh.close()
        except Exception:  # noqa: BLE001
            pass


# ---------------------------------------------------------------- 单个 fixture
def run_one(base, work, fx, tasks_dir):
    stem = fx.stem
    try:
        d = json.loads(fx.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        return {"fixture": stem, "ok": False, "why": f"fixture 解析失败: {e}"}

    meta = d.get("meta", {}) or {}
    task = meta.get("task", "?")
    goal = ((d.get("history", {}) or {}).get("goal", "") or "").strip()
    if not goal:
        return {"fixture": stem, "task": task, "ok": False, "why": "fixture 里没有 goal"}

    task_dir = pathlib.Path(tasks_dir) / task
    verify_file = task_dir / "verify.sh"
    if not verify_file.is_file():
        return {
            "fixture": stem,
            "task": task,
            "ok": False,
            "why": f"找不到任务目录（{task_dir}）",
        }

    marked = f"{goal}\n\n[replay-fixture: {stem}]"
    try:
        sess = api(
            base,
            "POST",
            "/api/v1/sessions",
            {
                "provider": "replay",
                "goal": marked,
                "budget": {"max_steps": max(int(meta.get("steps", 20)) + 5, 25)},
            },
        )
        sid = sess["session_id"]
    except Exception as e:  # noqa: BLE001
        return {"fixture": stem, "task": task, "ok": False, "why": f"建会话失败: {e}"}

    ws = pathlib.Path(work) / "sessions" / sid
    # 本地"上传"＝复制 fixture 内容到工作区（与 runner.py 摊平上传等价）
    shutil.copytree(task_dir / "fixture", ws, dirs_exist_ok=True)

    t0 = time.time()
    try:
        api(base, "POST", f"/api/v1/sessions/{sid}/messages", {"content": marked})
    except (urllib.error.URLError, TimeoutError, OSError):
        pass  # SSE 流不关是预期行为
    except Exception as e:  # noqa: BLE001
        return {"fixture": stem, "task": task, "ok": False, "why": f"发消息失败: {e}", "session_id": sid}

    deadline = time.time() + 180
    phase, steps = "created", 0
    while time.time() < deadline:
        time.sleep(0.5)
        try:
            s = api(base, "GET", f"/api/v1/sessions/{sid}", timeout=5)
            if isinstance(s, dict):
                phase = s.get("phase", phase)
                steps = s.get("steps", steps)
                if phase in POST_TERMINAL:
                    break
        except Exception:  # noqa: BLE001
            pass
    wall_s = round(time.time() - t0, 1)

    # 判据：与远端同一条契约（verify_exec.verify_task + 本地传输层）
    put, exec_ = verify_exec.local_shell_adapter(ws)
    ok, out, rc = verify_exec.verify_task(put, exec_, verify_file)

    return {
        "fixture": stem,
        "task": task,
        "ok": ok,
        "phase": phase,
        "steps": steps,
        "recorded_steps": meta.get("steps"),
        "wall_s": wall_s,
        "session_id": sid,
        "verify_rc": rc,
        "verify_tail": out.strip()[-300:],
    }


def main(argv=None):
    ap = argparse.ArgumentParser(description="本地评测驱动（M0-6/D-228）")
    ap.add_argument("--only", default=None, help="只跑 fixture 名含该子串的")
    ap.add_argument("--limit", type=int, default=0, help="最多跑几个（0=全部）")
    ap.add_argument("--provider", default="replay")
    ap.add_argument("--port", type=int, default=3011)
    ap.add_argument("--service-bin", default=str(REPO / "target" / "debug" / "service.exe")
                    if os.name == "nt" else str(REPO / "target" / "debug" / "service"))
    ap.add_argument("--work", default=str(DEFAULT_WORK))
    ap.add_argument("--fixtures", default=str(DEFAULT_FIXTURES))
    ap.add_argument("--tasks-dir", default=str(HERE / "tasks"))
    ap.add_argument("--out", default=str(DEFAULT_OUT))
    a = ap.parse_args(argv)

    fixtures = sorted(pathlib.Path(a.fixtures).glob("*.json"))
    if a.only:
        fixtures = [f for f in fixtures if a.only in f.stem]
    if a.limit > 0:
        fixtures = fixtures[: a.limit]
    if not fixtures:
        log("没有可跑的 fixture")
        return 1

    env_extra = load_dotenv(REPO / ".env")
    if env_extra:
        log(f"已从 .env 载入 {len(env_extra)} 个变量（键名：{', '.join(sorted(env_extra))}）")

    svc = LocalService(a.service_bin, a.work, a.port, a.fixtures, a.provider, env_extra)
    base = f"http://127.0.0.1:{a.port}"
    if not svc.start():
        svc.stop()
        return 1

    results = []
    try:
        log(f"=== 本地驱动：{len(fixtures)} 个 fixture，provider={a.provider} ===")
        for i, fx in enumerate(fixtures, 1):
            rec = run_one(base, a.work, fx, a.tasks_dir)
            rec["openai_key_injected"] = svc.openai_key_injected
            results.append(rec)
            log(
                f"[{i}/{len(fixtures)}] {fx.stem} -> "
                f"{'PASS' if rec.get('ok') else 'FAIL'} "
                f"steps={rec.get('steps')} rc={rec.get('verify_rc')} "
                f"{rec.get('why', '')}"
            )
    finally:
        svc.stop()

    out = pathlib.Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "a", encoding="utf-8") as f:
        for rec in results:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")

    ok = sum(1 for r in results if r.get("ok"))
    log(f"=== 结果：{ok}/{len(results)} 判通过（明细：{out}）===")
    return 0


if __name__ == "__main__":
    sys.exit(main())
