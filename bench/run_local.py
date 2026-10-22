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

# provider → 其真实 key 的 env 名（活体运行前置校验）。
# 不在清单 = 不支持：拒绝而不是猜测（fail-closed，同本仓口径）。
PROVIDER_KEY_ENV = {"agnes": "AGNES_API_KEY"}
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


def fetch_claim(base, sid):
    """取 **agent 自报的结论**（D-230）：transcript 里最后一个 `done` 事件的 `report.ok`。

    为什么不用 `/status.phase`：`phase=="done"` 只表示**循环正常终止**，不表示它声称做成了。
    实测 31 条录制语料里 `report.ok==true` 只有 1 条，其余 30 条自报 `budget_exhausted`，
    而 `phase` 全是 `done`——用 phase 会把 30 条"agent 自己承认没做完"的会话算成"自报完成"。

    Returns: `(claim, status, usage, cost_usd)`；claim ∈ {True, False, None}（None=取不到/无 `ok` 字段），
    status 是 `report.status`（`completed` / `budget_exhausted` / …，仅诊断用）。
    usage/cost_usd 在 **report 内层**——实证（2026-10-19 探针 dump）：`Event::Done` 的
    payload 就是 report 本体，D-219 的 emit 注入即落在其中
    （transcript 形状：`{"type":"done","report":{...,"usage":{...},"cost_usd":...}}`）。
    【订正记录】本函数一度改读 payload 顶层——那是**错的方向**，恒 None（顶层只有
    report+type）；若再见到"payload 顶层 usage"的声称，以探针 dump 为准。
    """
    try:
        hist = api(base, "GET", f"/api/v1/sessions/{sid}/messages", timeout=20)
    except Exception as e:  # noqa: BLE001
        log(f"    [warn] 取 transcript 失败（claim 记为未知）：{e}")
        return None, None, None, None
    msgs = (hist.get("messages") if isinstance(hist, dict) else hist) or []
    for m in reversed(msgs):
        if m.get("event_type") != "done":
            continue
        payload = m.get("payload") or {}
        rep = payload.get("report") or {}
        status = rep.get("status")
        # D-231：usage/cost_usd 在 **report 内层**（见 docstring 的实证与订正记录）
        usage = rep.get("usage")
        cost_usd = rep.get("cost_usd")
        if "ok" not in rep:
            return None, status, usage, cost_usd
        return (True if rep.get("ok") is True else False), status, usage, cost_usd
    return None, None, None, None


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

        # service 启动**无条件**要求 OPENAI_API_KEY（openai provider 恒注册，fail-closed）。
        # · provider=replay：注入占位值（不出网，D-228 三条硬约束）。
        # · 真实 provider：先要求**该 provider 自己的 key** 在场（agnes→AGNES_API_KEY），
        #   再注入占位值仅用于通过 openai 注册门——会话创建时**钉死** provider，
        #   占位值不会被任何调用使用（靠的是会话级 provider 字段，不是靠自觉）。
        if not env.get("OPENAI_API_KEY"):
            if self.provider != "replay":
                key_env = PROVIDER_KEY_ENV.get(self.provider)
                if not key_env:
                    raise SystemExit(
                        f"provider={self.provider} 不在本脚本支持清单（{sorted(PROVIDER_KEY_ENV)}）；"
                        "如需支持请登记其 key env 名，不得静默放行。"
                    )
                if not env.get(key_env):
                    raise SystemExit(
                        f"provider={self.provider} 需要真实 {key_env}（由 .env 提供）；"
                        "拒绝在缺真实 key 的情况下活体运行。"
                    )
            env["OPENAI_API_KEY"] = "local-runner-placeholder-never-called"
            self.openai_key_injected = True
            log(
                "注意：OPENAI_API_KEY 未设置 ⇒ 注入**占位值**仅为通过 service 启动门槛"
                f"（openai provider 恒注册、无条件校验）；会话固定 provider={self.provider}，"
                "占位值不会被任何调用使用。"
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
def run_one(base, work, fx, tasks_dir, control_substr=None, provider="replay", deadline_s=300):
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

    # replay 才加 fixture 标记（便于溯源）；活体运行保持 goal 原样，不给 agent 额外噪声。
    marked = goal if provider != "replay" else f"{goal}\n\n[replay-fixture: {stem}]"
    try:
        sess = api(
            base,
            "POST",
            "/api/v1/sessions",
            {
                "provider": provider,
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

    deadline = time.time() + deadline_s
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

    # D-229 **已知正例对照**：把工作区**打回原样**（撤销 agent 的改动），模拟"自报完成、
    # 其实没改对"。没有这条对照，"假完成率 = 0"就只是**结构性 0**（因为录制语料全是 PASS），
    # 等于没测——本仓对此有专门口径（"只用手造数据自证"/"结构性 0"）。
    #
    # D-230 修：不能用 `copytree(..., dirs_exist_ok=True)` —— 它只**覆盖** fixture 里已有的
    # 文件，**不删** agent 新建的文件（实测 T11 的产物是新文件 ⇒ 打回后判据仍 PASS，对照失效）。
    # 必须整目录重建，才是真正的 pristine。
    control_applied = bool(control_substr) and control_substr in stem
    if control_applied:
        shutil.rmtree(ws, ignore_errors=True)
        shutil.copytree(task_dir / "fixture", ws)
        log(f"    [对照] 已把 {stem} 的工作区**整目录重建**为原始 fixture（模拟假完成）")

    # 判据：与远端同一条契约（verify_exec.verify_task + 本地传输层）
    put, exec_ = verify_exec.local_shell_adapter(ws)
    ok, out, rc = verify_exec.verify_task(put, exec_, verify_file)

    # agent **自报**的结论（D-230：不能用 /status.phase 代替）+ 真实用量（D-219，report 内层）
    claim, claim_status, usage, cost_usd = fetch_claim(base, sid)

    rec = {
        "fixture": stem,
        "task": task,
        "provider": provider,
        "ok": ok,
        "phase": phase,
        "claim": claim,
        "claim_status": claim_status,
        "usage": usage,
        "cost_usd": cost_usd,
        "steps": steps,
        "recorded_steps": meta.get("steps"),
        "wall_s": wall_s,
        "session_id": sid,
        "verify_rc": rc,
        "verify_tail": out.strip()[-300:],
        "control_applied": control_applied,
    }
    # **agent 自报** × 外部判据（跨进程比对）——一等指标"假完成率"的原料
    rec.update(judge.classify_run(claim, ok))
    return rec


def main(argv=None):
    ap = argparse.ArgumentParser(description="本地评测驱动（M0-6/D-228）")
    ap.add_argument("--only", default=None, help="只跑 fixture 名含该子串的")
    ap.add_argument("--limit", type=int, default=0, help="最多跑几个（0=全部）")
    ap.add_argument("--provider", default="replay")
    ap.add_argument(
        "--deadline",
        type=int,
        default=300,
        help="单条会话等待终态的秒数（活体含 cargo 编译，默认 300；replay 用不了这么久）",
    )
    ap.add_argument("--port", type=int, default=3011)
    ap.add_argument("--service-bin", default=str(REPO / "target" / "debug" / "service.exe")
                    if os.name == "nt" else str(REPO / "target" / "debug" / "service"))
    ap.add_argument("--work", default=str(DEFAULT_WORK))
    ap.add_argument("--fixtures", default=str(DEFAULT_FIXTURES))
    ap.add_argument("--tasks-dir", default=str(HERE / "tasks"))
    ap.add_argument("--out", default=str(DEFAULT_OUT))
    ap.add_argument(
        "--false-completion-control",
        default=None,
        help="**仅用于自测指标**：名字含该子串的 fixture，在会话结束后把工作区打回原始 "
        "fixture（模拟『自报完成但没改对』），用来证明假完成率能报出正例。"
        "建议用 T11-read-struct__run0——录制里**唯一**自报成功（status=completed）的那条，"
        "这样对照全部是真实数据。",
    )
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
            rec = run_one(
                base, a.work, fx, a.tasks_dir,
                a.false_completion_control, a.provider, a.deadline,
            )
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

    # ---- 一等指标（M1）：agent 自报 × 外部判据 -------------------------------------------------
    s = judge.summarize_runs(results)
    rate = s["false_completion_rate"]
    rate_s = (
        "n/a（没有自称做成的会话 ⇒ **未知**，不按 0 冒充）"
        if rate is None
        else f"{rate:.1%}"
    )
    log("--- 一等指标 ---")
    log(
        f"runs={s['runs']}  自称做成={s['claim_ok']}  "
        f"外部判通过={s['external_ok']}  自报结论未知={s['unknown_claim']}"
    )
    log(
        f"**假完成率 = {s['false_completion']}/{s['claim_ok']} = {rate_s}**"
        "（分子：自称做成但外部判失败；分母：自称做成的会话数）"
    )
    log(
        f"反向不一致（agent 自报没做成、外部判据说成了）={s['verdict_conflict']}"
        "——多见于预算耗尽但活其实干完了"
    )

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
