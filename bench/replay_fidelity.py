#!/usr/bin/env python3
"""M1-1 双轨保真门禁：**回放必须真的在起作用**。

为什么需要它：`run_local --provider replay` 很容易变成"录了不看"——fixture 躺在目录里、
回放跑出全绿，而没人证明"这些绿是录制驱动出来的"。任务书 M1-1 的红侧写得很清楚：
**篡改回放里的一条响应 ⇒ 断言结果必须改变**。本脚本把这句话做成可复跑的门禁：

  ① 保真（同轨）：把活体轨录下的 trace 拿去 replay，逐条比对
     `meta.live_verify_ok`（活体时的外部判据结论）与回放判据结论，必须一致；
  ② 红侧（篡改）：把其中一条 fixture 的**最后一个 tool_call** 删掉、换成 "DONE"，
     回放结论必须**翻转**（活体 PASS ⇒ 回放 FAIL）。不翻转 ⇒ 说明回放没在驱动 agent，
     或夹具与判据之间有断路 ⇒ 门禁 FAIL（这正是本仓"尺子自己先过秤"的口径）。

前置：录制由 `run_local.py --record-dir <dir> --provider agnes ...` 产生（同一次活体跑）。

用法：
  python bench/replay_fidelity.py --recorded-dir bench/replay/live --port 3021
"""
import argparse
import json
import pathlib
import shutil
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))

import run_local  # noqa: E402

TAMPER_STEM_SUFFIX = "__tampered"
# 写类工具（tools-builtin 实际名字：write_file / apply_patch / bash）。
WRITE_TOOLS = ("write_file", "apply_patch")
NOOP_PATH = "src/__tampered_noop.rs"


def tamper_writes(messages):
    """把回放里**每一个写操作**改道到无效目标，其余事件原样保留。

    为什么不用"删掉最后一条工具调用"（本脚本初版的做法，实测**没翻转**）：T01 这类任务的
    写入发生在**中途**，砍掉尾部事件后目标文件其实已经写好了 ⇒ 判据照样 PASS，篡改
    变成"改了个寂寞"。改道写目标才是真正动到"回放驱动的那一次动作"：
    agent 的每次写都落到 `NOOP_PATH`，被验收的文件一个字节都没变 ⇒ 结论必然翻转。

    返回施加的篡改次数（0 ⇒ 这条 fixture 没有写操作，无法用本手法证伪，调用方须报错）。
    """
    n = 0
    for m in messages:
        if m.get("event_type") != "tool_call":
            continue
        payload = m.get("payload") or {}
        if payload.get("name") not in WRITE_TOOLS:
            continue
        args = payload.get("args")
        if not isinstance(args, dict):
            continue
        args["path"] = NOOP_PATH
        if "content" in args:
            args["content"] = "// tampered by bench/replay_fidelity.py\n"
        n += 1
    return n


def build_tampered(src: pathlib.Path, dest_dir: pathlib.Path):
    """复制一条 fixture 并把写操作改道；返回 (新路径, 篡改次数)。"""
    d = json.loads(src.read_text(encoding="utf-8"))
    hist = d.get("history") or {}
    msgs = hist.get("messages") or []
    n = tamper_writes(msgs)
    if n == 0:
        return None, 0
    d.setdefault("meta", {})["tampered"] = f"redirect-{n}-writes-to-{NOOP_PATH}"
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / f"{src.stem}{TAMPER_STEM_SUFFIX}.json"
    dest.write_text(json.dumps(d, ensure_ascii=False, indent=1), encoding="utf-8")
    return dest, n


def replay_dir(fixtures_dir, tasks_dir, port, work, deadline=120, only=None):
    """用 run_local 的**同一套**驱动跑 replay（不另写一份执行路径）。"""
    env_extra = run_local.load_dotenv(REPO / ".env")
    svc = run_local.LocalService(
        str(REPO / "target" / "debug" / "service.exe")
        if run_local.os.name == "nt"
        else str(REPO / "target" / "debug" / "service"),
        work,
        port,
        fixtures_dir,
        "replay",
        env_extra,
    )
    if not svc.start():
        svc.stop()
        raise SystemExit("service 未就绪")
    out = []
    try:
        files = sorted(pathlib.Path(fixtures_dir).glob("*.json"))
        if only:
            files = [f for f in files if only in f.stem]
        for fx in files:
            rec = run_local.run_one(
                f"http://127.0.0.1:{port}", work, fx, tasks_dir, None, "replay", deadline
            )
            out.append(rec)
            print(
                f"  [replay] {fx.stem} -> {'PASS' if rec.get('ok') else 'FAIL'} "
                f"steps={rec.get('steps')} rc={rec.get('verify_rc')}",
                flush=True,
            )
    finally:
        svc.stop()
    return out


def main():
    ap = argparse.ArgumentParser(description="M1-1 双轨保真门禁")
    ap.add_argument("--recorded-dir", default=str(HERE / "replay" / "live"))
    ap.add_argument("--tasks-dir", default=str(HERE / "tasks"))
    ap.add_argument("--port", type=int, default=3021)
    ap.add_argument("--work", default=str(REPO / "target" / "replay-fidelity"))
    ap.add_argument("--only", default=None, help="只验名字含该子串的（其余豁免并如实列出）")
    a = ap.parse_args()

    recorded = sorted(pathlib.Path(a.recorded_dir).glob("*.json"))
    recorded = [p for p in recorded if not p.stem.endswith(TAMPER_STEM_SUFFIX)]
    if a.only:
        recorded = [p for p in recorded if a.only in p.stem]
    if not recorded:
        print(f"FIDELITY_FAIL: {a.recorded_dir} 下没有录制（先跑 run_local --record-dir）")
        return 1

    work = pathlib.Path(a.work)
    shutil.rmtree(work, ignore_errors=True)

    print(f"=== ① 保真：{len(recorded)} 条录制 → replay ===")
    got = replay_dir(a.recorded_dir, a.tasks_dir, a.port, work)

    bad = []
    for p, rec in zip(recorded, got):
        live_ok = bool((json.loads(p.read_text(encoding="utf-8")).get("meta") or {}).get("live_verify_ok"))
        replay_ok = bool(rec.get("ok"))
        mark = "OK  " if live_ok == replay_ok else "DIFF"
        print(f"  [{mark}] {p.stem}: live_ok={live_ok} replay_ok={replay_ok}")
        if live_ok != replay_ok:
            bad.append((p.stem, live_ok, replay_ok))
    if bad:
        print(f"FIDELITY_FAIL: {len(bad)} 条回放结论与活体不一致：{bad}")
        return 1
    print(f"① PASS：{len(recorded)}/{len(recorded)} 条回放结论与活体一致")

    # ── ② 红侧：篡改一条 ⇒ 结论必须翻转 ──
    # 只取一条**活体 PASS** 的来篡改（活体就 FAIL 的，篡改后仍 FAIL ⇒ 看不翻转）。
    target = None
    for p in recorded:
        meta = json.loads(p.read_text(encoding="utf-8")).get("meta") or {}
        if meta.get("live_verify_ok") is True:
            target = p
            break
    if target is None:
        print("FIDELITY_FAIL: 录制里没有一条活体 PASS 的样本 ⇒ 篡改方向无法证伪")
        return 1

    tamper_dir = work / "tampered"
    dest, n_tamper = build_tampered(target, tamper_dir)
    if dest is None:
        print(f"FIDELITY_FAIL: {target.stem} 里没有可改道的写操作（无法证伪）")
        return 1
    print(f"\n=== ② 红侧：{target.stem} 的 {n_tamper} 处写操作改道 → replay ===")
    got2 = replay_dir(tamper_dir, a.tasks_dir, a.port + 1, work / "tamper-run")
    rec2 = got2[0]
    if bool(rec2.get("ok")):
        print(
            "FIDELITY_FAIL: 篡改后判据**仍 PASS** ⇒ 回放没有真正驱动 agent"
            f"（或夹具与判据断路）：{dest.name}"
        )
        return 1
    print("② PASS：篡改一条响应后结论翻转（PASS → FAIL）⇒ 回放确在起作用")

    print("\nFIDELITY PASS：双轨保真 + 篡改可证伪 两项均成立")
    return 0


if __name__ == "__main__":
    sys.exit(main())
