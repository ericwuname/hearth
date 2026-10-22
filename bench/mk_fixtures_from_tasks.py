#!/usr/bin/env python3
"""把任务目录（tasks root）转成 run_local 可驱动的伪 fixture（M1-1 活体全量用，D-232）。

**为什么需要**：`bench/run_local.py` 以"录制 fixture JSON"为输入（从中取 goal 与 task 名）。
held-out 实例（D-227 生成）**没有录制会话**——本脚本从 `goal.txt`/`meta.json` 合成等价
输入（goal 原文、task 名、步数上限），**不掺任何录制内容**，使活体全量重测能覆盖
"公开 20 任务 + held-out 生成变体"两个套件。

产出：`<out>/<TID>__live.json`，形如 `{"meta":{"task","steps"},"history":{"goal"}}`。
"""
import argparse
import json
from pathlib import Path


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--tasks", required=True, help="任务根目录（含 <TID>/{goal.txt,verify.sh}）")
    ap.add_argument("--out", required=True, help="伪 fixture 输出目录")
    ap.add_argument("--max-steps", type=int, default=20, help="步数预算（run_local 会 +5 兜底）")
    a = ap.parse_args()

    tasks = Path(a.tasks)
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    n = 0
    for tdir in sorted(p for p in tasks.iterdir() if p.is_dir()):
        if not (tdir / "goal.txt").is_file() or not (tdir / "verify.sh").is_file():
            continue  # 与 pristine_must_fail.task_list 同一结构判定口径
        goal = (tdir / "goal.txt").read_text(encoding="utf-8").strip()
        if not goal:
            print(f"[skip] {tdir.name}: goal.txt 为空")
            continue
        meta = {}
        mf = tdir / "meta.json"
        if mf.is_file():
            try:
                meta = json.loads(mf.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                pass
        fx = {
            "meta": {
                "task": tdir.name,
                "steps": min(int(meta.get("max_steps", a.max_steps)), a.max_steps),
            },
            "history": {"goal": goal},
        }
        dest = out / f"{tdir.name}__live.json"
        dest.write_text(json.dumps(fx, ensure_ascii=False, indent=1), encoding="utf-8")
        n += 1
    print(f"synthesized {n} pseudo-fixtures -> {out}")


if __name__ == "__main__":
    main()
