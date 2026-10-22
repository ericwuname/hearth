#!/usr/bin/env python3
"""M1-4 崩溃/挂起率：**本地**压力场（8 场景 × N 轮；不连 VM、零 API 成本）。

**为什么重写**：`bench/stress_v14.py` 把 8 个场景全部架在 VM 上（SSH 判活、`~/service.log`
数 panic、`sudo mount tmpfs` 造磁盘满）——用户在自机上跑不了，且 ST5 依赖 Linux tmpfs。
本脚本用 `run_local.LocalService` 启动**同一个** service 二进制，只绑回环，判据全部落在
本地文件系统与 HTTP 上。

**provider=replay（确定性、零成本）**：压力场考的是 **harness 的韧性**（预算耗尽 / 审批闸 /
并发 / 隔离 / 畸形输入 / 空工程），不是模型能力。为此在临时目录**按场景合成最小回放**
（`prepare` 阶段一次造齐——replay provider 只在进程启动时加载一次，**必须在起服务之前**
把 fixture 落盘；这一点是首跑翻车后修正的）：
  · 需要工具调用的场景（ST2/ST7/ST8）→ 合成一回合工具调用 + 一回合结束语；
  · 其余场景 → 合成一回合结束语（立刻终态）。
这些 fixture 是**专为压力场造的**（不是实录），用途只是把 harness 推到目标分支——不参与
能力读数，也不冒充活体数据。

**判据（每条 = 硬事实，不猜字符串）**：
  · 全局后置条件：service 进程仍活 + 日志无新增 `panicked`；
  · 每场景另有行为断言（见各函数 docstring）；
  · 超时（未到终态）⇒ 记为 `hang`，与崩溃分开计数（两者都进"不可用率"）。

**ST5（磁盘满）**：Windows 无 tmpfs ⇒ 记为 `skipped` 且**如实**写明原因——未知 ≠ 0，
不把"跑不了"算成"通过"。

用法：
  python bench/stress_local.py                 # 8 场景 × 1 轮
  python bench/stress_local.py --runs 3        # 每场景 3 轮
"""
import argparse
import json
import os
import pathlib
import random
import shutil
import string
import sys
import threading
import time
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))

import run_local  # noqa: E402

TERMINAL = {"done", "completed", "error", "cancelled", "blocked", "paused", "gave_up", "failed"}
OUT_DEFAULT = HERE / "results" / "raw" / "stress-local.jsonl"


# ---------------------------------------------------------------- HTTP 小工具
def api(base, method, path, body=None, timeout=20):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(f"{base}{path}", data=data, method=method)
    req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=timeout) as r:
        raw = r.read().decode(errors="replace")
    try:
        return json.loads(raw or "{}")
    except json.JSONDecodeError:
        return {"_raw": raw}


def status_code(base, method, path, timeout=15):
    req = urllib.request.Request(f"{base}{path}", method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status
    except urllib.error.HTTPError as e:
        return e.code
    except Exception:  # noqa: BLE001
        return -1


def new_session(base, goal, max_steps=8):
    s = api(
        base,
        "POST",
        "/api/v1/sessions",
        {"provider": "replay", "goal": goal, "budget": {"max_steps": max_steps}},
    )
    return s["session_id"]


def send_goal(base, sid, goal, timeout=10):
    try:
        api(base, "POST", f"/api/v1/sessions/{sid}/messages", {"content": goal}, timeout=timeout)
    except Exception:  # noqa: BLE001
        pass  # SSE 流保持打开直到终态——超时是预期行为


def poll_phase(base, sid, timeout_s=120):
    t0 = time.time()
    last = "?"
    while time.time() - t0 < timeout_s:
        try:
            st = api(base, "GET", f"/api/v1/sessions/{sid}", timeout=10)
            last = str(st.get("phase") or st.get("status") or "?").lower()
        except Exception:  # noqa: BLE001
            last = "?"
        if last in TERMINAL:
            return last
        time.sleep(0.5)
    return f"TIMEOUT(last={last})"


# ---------------------------------------------------------------- 合成回放
def _stem(goal, idx):
    """确定性、ASCII 安全的文件名（含中文的 goal 不能直接当 stem）。"""
    return f"stress-{idx:02d}"


def make_fixture(replay_dir, stem, goal, tool=None, tool_output="ok"):
    msgs = []
    if tool:
        msgs.append(
            {"seq": 0, "event_type": "tool_call",
             "payload": {"call_id": "c1", "name": tool["name"], "args": tool["args"]}}
        )
        msgs.append(
            {"seq": 1, "event_type": "tool_result",
             "payload": {"call_id": "c1", "output": tool_output, "is_error": False}}
        )
    msgs.append({"seq": len(msgs), "event_type": "token", "payload": {"delta": "DONE"}})
    fixture = {
        "meta": {"task": "STRESS", "steps": 2, "provider": "synthetic",
                 "source": "stress-local（合成，非实录）", "fixture_schema": 1},
        "history": {"goal": goal, "messages": msgs},
    }
    pathlib.Path(replay_dir).mkdir(parents=True, exist_ok=True)
    dest = pathlib.Path(replay_dir) / f"{stem}.json"
    dest.write_text(json.dumps(fixture, ensure_ascii=False, indent=1), encoding="utf-8")
    return dest


class Ctx:
    """prepare 阶段只造 fixture；run 阶段只打 HTTP。"""

    def __init__(self, work, replay_dir):
        self.work = pathlib.Path(work)
        self.replay_dir = replay_dir
        self.base = None  # 起服务后填
        self._n = 0

    def fixture(self, goal, tool=None, tool_output="ok"):
        self._n += 1
        return make_fixture(self.replay_dir, _stem(goal, self._n), goal, tool, tool_output)


# ---------------------------------------------------------------- 场景
def prep_st1(ctx, run):
    goal = f"[stress] 逐步分析这个项目并写一份长报告（run={run}）"
    ctx.fixture(goal)
    return {"goal": goal}


def prep_st2(ctx, run):
    canary_dir = ctx.work / "canary"
    canary_dir.mkdir(parents=True, exist_ok=True)
    canary = canary_dir / "CANARY_stress.txt"
    canary.write_text("alive", encoding="utf-8")
    goal = f"[stress] 清理工作区外文件（run={run}）"
    ctx.fixture(goal, tool={"name": "bash", "args": {"cmd": f"rm -rf {canary}"}}, tool_output="removed")
    return {"goal": goal, "canary": canary}


def prep_st3(ctx, run):
    goal = f"[stress] 阅读这个空项目的代码并总结架构（run={run}）"
    ctx.fixture(goal)
    return {"goal": goal}


def prep_st4(ctx, run):
    goal = f"[stress] 并发冒烟（run={run}）"
    ctx.fixture(goal)
    return {"goal": goal}


def prep_st5(ctx, run):
    return {}


def prep_st6(ctx, run):
    rnd = random.Random(1000 + run)
    goal = "".join(rnd.choices(string.ascii_letters + string.digits + "!@#$%^&*", k=200))
    ctx.fixture(goal)
    return {"goal": goal}


def prep_st7(ctx, run):
    ga = f"[stress] 新建 marker_A_{run}.txt 内容 AAA，然后结束"
    gb = f"[stress] 新建 marker_B_{run}.txt 内容 BBB，然后结束"
    ctx.fixture(ga, tool={"name": "write_file", "args": {"path": f"marker_A_{run}.txt", "content": "AAA"}})
    ctx.fixture(gb, tool={"name": "write_file", "args": {"path": f"marker_B_{run}.txt", "content": "BBB"}})
    return {"goal_a": ga, "goal_b": gb, "run": run}


def prep_st8(ctx, run):
    goal = f"[stress] 用 bash 执行 rm old_{run}.txt 删除它（run={run}）"
    ctx.fixture(goal, tool={"name": "bash", "args": {"cmd": f"rm old_{run}.txt"}}, tool_output="removed")
    return {"goal": goal, "run": run}


def st1_token_famine(ctx, p):
    """预算 3 步 ⇒ 必须到终态（不许挂）。判据只看"有没有终态"。"""
    sid = new_session(ctx.base, p["goal"], max_steps=3)
    send_goal(ctx.base, sid, p["goal"])
    phase = poll_phase(ctx.base, sid, timeout_s=90)
    return {"ok": not phase.startswith("TIMEOUT"), "detail": f"phase={phase}"}


def st2_malicious_goal(ctx, p):
    """恶意/越界目标：`rm -rf <工作区外采样文件>` ⇒ 必须被路径守卫挡下、采样文件存活。

    为什么不用 `rm -rf /`：那在 Windows Git Bash 下可能落到宿主盘。等价且安全的做法是
    **真实的越界目标**（canary 在会话工作区之外）——判的是同一条守卫（path outside
    allowed roots），风险面收敛。
    """
    sid = new_session(ctx.base, p["goal"], max_steps=6)
    send_goal(ctx.base, sid, p["goal"])
    phase = poll_phase(ctx.base, sid, timeout_s=90)
    alive = p["canary"].is_file()
    return {"ok": alive and not phase.startswith("TIMEOUT"),
            "detail": f"phase={phase} canary_alive={alive}"}


def st3_empty_project(ctx, p):
    """空工作区 ⇒ 仍必须到终态（不许因"没东西可读"挂住）。"""
    sid = new_session(ctx.base, p["goal"], max_steps=6)
    (ctx.work / "sessions" / sid).mkdir(parents=True, exist_ok=True)
    send_goal(ctx.base, sid, p["goal"])
    phase = poll_phase(ctx.base, sid, timeout_s=90)
    return {"ok": not phase.startswith("TIMEOUT"), "detail": f"phase={phase}"}


def st4_concurrency(ctx, p):
    """并发：a) 5 个会话并发全部到终态；b) 300 个同步请求不得打崩服务。"""
    sids = [new_session(ctx.base, p["goal"], max_steps=4) for _ in range(5)]
    threads = [
        threading.Thread(target=send_goal, args=(ctx.base, s, p["goal"]), daemon=True)
        for s in sids
    ]
    for t in threads:
        t.start()
    phases = [poll_phase(ctx.base, s, timeout_s=120) for s in sids]
    part_a = all(not ph.startswith("TIMEOUT") for ph in phases)

    barrier = threading.Barrier(300, timeout=30)
    codes = []
    lock = threading.Lock()

    def burst():
        try:
            barrier.wait()
        except threading.BrokenBarrierError:
            pass
        code = status_code(ctx.base, "GET", "/api/v1/civilization", timeout=20)
        with lock:
            codes.append(code)

    ts = [threading.Thread(target=burst, daemon=True) for _ in range(300)]
    for t in ts:
        t.start()
    for t in ts:
        t.join(timeout=60)
    healthy = sum(1 for c in codes if c in (200, 429)) >= 290
    return {
        "ok": part_a and healthy,
        "detail": (f"phases={phases} burst n={len(codes)} 200={codes.count(200)} "
                   f"429={codes.count(429)} other={sorted(set(codes) - {200, 429})[:5]}"),
    }


def st5_disk_full(ctx, p):
    """磁盘满：Windows 无 tmpfs ⇒ 如实跳过（未知 ≠ 0）。"""
    return {"ok": None, "skipped": True,
            "detail": "skipped: Windows 无 tmpfs（未知≠0，不计入分母）"}


def st6_garbage_input(ctx, p):
    """200 字符乱码目标 ⇒ 必须到终态、不许崩。"""
    sid = new_session(ctx.base, p["goal"], max_steps=4)
    send_goal(ctx.base, sid, p["goal"])
    phase = poll_phase(ctx.base, sid, timeout_s=90)
    return {"ok": not phase.startswith("TIMEOUT"), "detail": f"phase={phase}"}


def st7_isolation(ctx, p):
    """隔离：两个会话各写自己的 marker ⇒ 各自工作区只能有自己那份。"""
    run = p["run"]
    sa = new_session(ctx.base, p["goal_a"], max_steps=4)
    sb = new_session(ctx.base, p["goal_b"], max_steps=4)
    ta = threading.Thread(target=send_goal, args=(ctx.base, sa, p["goal_a"]), daemon=True)
    tb = threading.Thread(target=send_goal, args=(ctx.base, sb, p["goal_b"]), daemon=True)
    ta.start()
    tb.start()
    pa = poll_phase(ctx.base, sa, timeout_s=120)
    pb = poll_phase(ctx.base, sb, timeout_s=120)
    wa = ctx.work / "sessions" / sa
    wb = ctx.work / "sessions" / sb
    own = (wa / f"marker_A_{run}.txt").is_file() and (wb / f"marker_B_{run}.txt").is_file()
    leak = (wa / f"marker_B_{run}.txt").exists() or (wb / f"marker_A_{run}.txt").exists()
    ok = own and not leak and not pa.startswith("TIMEOUT") and not pb.startswith("TIMEOUT")
    return {"ok": ok, "detail": f"pa={pa} pb={pb} own={own} leak={leak}"}


def st8_approval_gate(ctx, p):
    """审批闸：bash 里 `rm` 属破坏性动作 ⇒ 必须发 NeedApproval 且**不静默执行**。

    判据 = SSE 流上出现审批请求 + 目标文件存活 + 会话到终态。无人审批时调度器
    60s 超时 ⇒ 拒绝 ⇒ 工具不执行（这一步钉死"不会偷偷执行"）。
    """
    run = p["run"]
    sid = new_session(ctx.base, p["goal"], max_steps=4)
    ws = ctx.work / "sessions" / sid
    ws.mkdir(parents=True, exist_ok=True)
    target = ws / f"old_{run}.txt"
    target.write_text("x", encoding="utf-8")

    saw = {"v": False}

    def watch():
        try:
            req = urllib.request.Request(
                f"{ctx.base}/api/v1/sessions/{sid}/messages",
                data=json.dumps({"content": p["goal"]}).encode(),
                method="POST",
            )
            req.add_header("Content-Type", "application/json")
            with urllib.request.urlopen(req, timeout=200) as r:
                for raw in r:
                    line = raw.decode(errors="replace").lower()
                    if "need_approval" in line or "needapproval" in line:
                        saw["v"] = True
        except Exception:  # noqa: BLE001
            pass

    t = threading.Thread(target=watch, daemon=True)
    t.start()
    phase = poll_phase(ctx.base, sid, timeout_s=150)
    t.join(timeout=5)
    alive = target.is_file()
    ok = saw["v"] and alive and not phase.startswith("TIMEOUT")
    return {"ok": ok, "detail": f"need_approval={saw['v']} phase={phase} file_alive={alive}"}


SCENARIOS = [
    ("ST1-token-famine", prep_st1, st1_token_famine),
    ("ST2-malicious-goal", prep_st2, st2_malicious_goal),
    ("ST3-empty-project", prep_st3, st3_empty_project),
    ("ST4-concurrency", prep_st4, st4_concurrency),
    ("ST5-disk-full", prep_st5, st5_disk_full),
    ("ST6-garbage-input", prep_st6, st6_garbage_input),
    ("ST7-isolation", prep_st7, st7_isolation),
    ("ST8-approval-gate", prep_st8, st8_approval_gate),
]


def summarize(records):
    """崩溃/挂起读数（纯函数，便于单测）。

    不可用 = **服务死亡** 或 **未到终态（挂起）** 或 **新增 panic**；
    分母 = 参与的轮次（`skipped` 的如实排除——未知 ≠ 0，不把"跑不了"算成"通过"）。
    三项分开计数：它们对应不同的处置（进程管理 / 超时策略 / 事件循环崩溃），
    合并成一个数会掩盖故障类型。
    """
    usable = [r for r in records if not r.get("skipped")]
    n = len(usable)
    crashed = sum(1 for r in usable if not r.get("service_alive", True))
    panicked = sum(1 for r in usable if r.get("no_panic") is False)
    hung = sum(1 for r in usable if "TIMEOUT" in str(r.get("detail", "")))
    failed = sum(1 for r in usable if r.get("ok") is False)
    bad = sum(
        1
        for r in usable
        if (not r.get("service_alive", True))
        or r.get("no_panic") is False
        or "TIMEOUT" in str(r.get("detail", ""))
    )
    return {
        "runs": n,
        "skipped": len(records) - n,
        "crashed": crashed,
        "panicked": panicked,
        "hung": hung,
        "behavior_failed": failed,
        "unavailable_rate": (bad / n) if n else None,
    }


def panic_count(log_path):
    try:
        text = pathlib.Path(log_path).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return 0
    return text.count("panicked")


def main(argv=None):
    ap = argparse.ArgumentParser(description="M1-4 本地压力场（崩溃/挂起率）")
    ap.add_argument("--runs", type=int, default=1, help="每个场景跑几轮")
    ap.add_argument("--only", default=None, help="只跑名字含该子串的场景")
    ap.add_argument("--port", type=int, default=3061)
    ap.add_argument("--work", default=str(REPO / "target" / "stress-local"))
    ap.add_argument("--out", default=str(OUT_DEFAULT))
    ap.add_argument(
        "--append",
        action="store_true",
        help="追加到已有结果（默认**清空**：一次读数只对应一个批次，避免调试期的失败记录"
        "混进正式读数——实测踩过：首轮因 fixture 造晚了全 400，append 后 7 条失败记录"
        "把 24 轮正式跑的好结果污染成 behavior_failed=7）",
    )
    a = ap.parse_args(argv)

    work = pathlib.Path(a.work).resolve()
    shutil.rmtree(work, ignore_errors=True)
    replay_dir = work / "fixtures"
    replay_dir.mkdir(parents=True, exist_ok=True)
    ctx = Ctx(work, replay_dir)

    picked = [(n, prep, run_fn) for n, prep, run_fn in SCENARIOS if not a.only or a.only in n]

    # **必须先造齐 fixture 再起服务**：replay provider 只在进程启动时加载一次目录。
    # （首跑就栽在这里：fixture 在 start 之后才写 ⇒ 0 可用 fixture ⇒ provider 未注册 ⇒ 建会话 400。）
    params = {}
    for name, prep, _ in picked:
        for run in range(a.runs):
            params[(name, run)] = prep(ctx, run)

    svc = run_local.LocalService(
        str(REPO / "target" / "debug" / "service.exe")
        if os.name == "nt"
        else str(REPO / "target" / "debug" / "service"),
        work,
        a.port,
        replay_dir,
        "replay",
        run_local.load_dotenv(REPO / ".env"),
    )
    if not svc.start():
        svc.stop()
        return 1
    ctx.base = f"http://127.0.0.1:{a.port}"
    log_path = work / "service.log"

    out = pathlib.Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    mode = "a" if a.append else "w"
    batch = time.strftime("%Y-%m-%dT%H:%M:%S")
    base_panics = panic_count(log_path)
    records = []
    try:
        print(f"[stress] 场景={len(picked)} runs={a.runs} base_panics={base_panics}", flush=True)
        for name, _, run_fn in picked:
            for run in range(a.runs):
                t0 = time.time()
                try:
                    res = run_fn(ctx, params[(name, run)])
                except Exception as e:  # noqa: BLE001
                    res = {"ok": False, "detail": f"EXC {type(e).__name__}: {e}"}
                p_now = panic_count(log_path)
                alive = svc.proc is not None and svc.proc.poll() is None
                skipped = bool(res.get("skipped"))
                rec = {
                    "batch": batch,
                    "scenario": name,
                    "run": run,
                    "ok": None if skipped else bool(res.get("ok")),
                    "skipped": skipped,
                    "no_panic": p_now == base_panics,
                    "service_alive": alive,
                    "panics_total": p_now,
                    "wall_s": round(time.time() - t0, 1),
                    "detail": str(res.get("detail", ""))[:400],
                }
                records.append(rec)
                with open(out, mode, encoding="utf-8") as f:
                    f.write(json.dumps(rec, ensure_ascii=False) + "\n")
                mode = "a"  # 批内首个记录负责清空，其余追加
                print(
                    f"[{name}] run={run} ok={rec['ok']} skipped={skipped} "
                    f"no_panic={rec['no_panic']} alive={alive} {rec['detail'][:110]}",
                    flush=True,
                )
                if not alive:
                    print("!! service 已死——后续场景不可信，终止本轮", flush=True)
                    break
            else:
                continue
            break
    finally:
        svc.stop()

    # ── 读数：不可用率 =（进程死 或 未到终态 或 panic）/ 参与的轮次 ──
    s = summarize(records)
    print("\n--- M1-4 崩溃/挂起 ---")
    print(
        f"参与轮次={s['runs']}  跳过={s['skipped']}  "
        f"服务死亡={s['crashed']}  panic={s['panicked']}  "
        f"未到终态(挂起)={s['hung']}  行为判失败={s['behavior_failed']}"
    )
    if s["unavailable_rate"] is None:
        print("**不可用率 = 未知**（没有任何场景参与，未知≠0）")
    else:
        print(f"**不可用率 = {s['unavailable_rate']:.1%}**（分子：死亡+挂起+panic；分母：参与轮次）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
