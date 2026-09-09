#!/usr/bin/env python3
"""v17 helper v2: construct experiences from baseline FAILs using rules
(no LLM needed — zhipu returns empty content, deepseek is 402).

Each FAIL becomes an experience with:
  category = level-based
  problem  = goal text
  solution = "verify 失败: <verify_tail>；注意 <task> 类任务的常见陷阱"
  success  = false, effectiveness = 0.8

Inject to VM experience.jsonl + restart service.
Usage: python bench/gen_experiences_v17.py
"""
import json, os, sys, time, urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw"
BASELINE = RAW / os.environ.get("V18_SRC", "matrix-v17-baseline.jsonl")
TASKS = HERE / "tasks"

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex_work"

CATEGORY_BY_LEVEL = {
    "L1": "简单单文件修改",
    "L2": "单文件重构",
    "L3": "多文件+逻辑",
    "L4": "多文件+外部依赖",
    "L5": "泛型/并发重构",
}


def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def main():
    rows = []
    if BASELINE.exists():
        for line in BASELINE.read_text(encoding="utf-8").splitlines():
            if line.strip():
                try:
                    rows.append(json.loads(line))
                except Exception:
                    pass
    fails = [r for r in rows if not r.get("success")]
    log(f"baseline: {len(rows)} rows, {len(fails)} fails")

    experiences = []
    for r in fails:
        task = r.get("task", "?")
        run = r.get("run", 0)
        level = r.get("level", "?")
        error = (r.get("verify_tail") or r.get("error") or "").strip()
        goal_file = TASKS / task / "goal.txt"
        goal = goal_file.read_text(encoding="utf-8").strip() if goal_file.exists() else task

        # Rule-based construction (no LLM)
        exp = {
            "id": f"exp-{task}-r{run}-v17",
            "category": CATEGORY_BY_LEVEL.get(level, "unknown"),
            "problem": goal[:200],
            "solution": f"【{task} L{level} 验证失败教训】verify_tail={error[:80]}。"
                        f"此任务类型（{CATEGORY_BY_LEVEL.get(level, '?')}）需要精确匹配命名/接口/宏要求，"
                        f"改完必须跑 cargo test 自验证。",
            "success": False,
            "effectiveness": 0.8,
            "reference_count": 0,
            "created_at": time.strftime("%Y-%m-%dT%H:%M:%S+08:00"),
            "context": {"os": "linux", "toolchain_version": "0.1.0"},
        }
        experiences.append(exp)
        log(f"  constructed {task} r{run} L{level} -> {exp['category']}")

    if not experiences:
        log("no experiences generated")
        sys.exit(1)

    # v18: write to local file only — injection is handled by self-evolve-v18.py
    # (which restarts service with the right EMBEDDING flag per phase).
    out = RAW / os.environ.get("V18_OUT", "v18-rule-experiences.jsonl")
    jsonl_content = "\n".join(json.dumps(e, ensure_ascii=False) for e in experiences) + "\n"
    out.write_text(jsonl_content, encoding="utf-8")
    log(f"wrote {out} ({len(experiences)} experiences)")
    log("=== GEN EXPERIENCES DONE ===")
    return

    # Inject via SSH (legacy v17 path — kept for reference)
    import paramiko
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)

    jsonl_content = "\n".join(json.dumps(e, ensure_ascii=False) for e in experiences) + "\n"
    ftp = c.open_sftp()
    with ftp.file(f"{REMOTE}/memory/experience.jsonl", "w") as f:
        f.write(jsonl_content)
    ftp.close()

    _, so, _ = c.exec_command(
        f"pkill -x service; sleep 2; "
        f"( cd {REMOTE} && setsid nohup ./target/debug/service "
        f"> /home/wutao/service.log 2>&1 < /dev/null & disown ) "
        f"> /dev/null 2>&1; sleep 10; echo OK"
    )
    log(f"service restart: {so.read().decode(errors='replace').strip()}")

    # Verify
    time.sleep(3)
    try:
        req = urllib.request.Request("http://192.168.220.131:3000/api/v1/experience/metrics")
        resp = urllib.request.urlopen(req, timeout=10)
        m = json.loads(resp.read().decode())
        log(f"experience store after injection: {m}")
        if m.get("total_experiences", 0) < len(experiences):
            log("WARNING: not all experiences loaded!")
    except Exception as e:
        log(f"metrics check failed: {e}")
    c.close()

    out = RAW / "v17-condensed-experiences.jsonl"
    out.write_text(jsonl_content, encoding="utf-8")
    log(f"wrote {out} ({len(experiences)} experiences)")
    log("=== GEN EXPERIENCES DONE ===")


if __name__ == "__main__":
    main()
