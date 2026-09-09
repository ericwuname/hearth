#!/usr/bin/env python3
"""v18 S2: LLM-refined experiences via zhipu glm-4.7 (glm-4.5-flash returns
empty content — verified unusable). Reads FAIL records from a baseline JSONL
(goal + verify_tail + level), asks glm-4.7 for a semantic problem/solution,
writes the refined experience.jsonl to the VM and restarts the service.

Usage:
    V18_SRC=matrix-v18-e0.jsonl python bench/refine_experiences_v18.py
Env:
    V18_SRC   baseline JSONL to read FAILs from (default matrix-v18-e0.jsonl)
    V18_OUT   remote experience.jsonl path (default under VM memory dir)
"""
import json
import os
import sys
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw"
SRC = RAW / os.environ.get("V18_SRC", "matrix-v18-e0.jsonl")
TASKS = HERE / "tasks"

ZHIPU_KEY = os.environ.get("ZHIPU_API_KEY", "")
ZHIPU_URL = "https://open.bigmodel.cn/api/paas/v4/chat/completions"
ZHIPU_MODEL = "glm-4.7"

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex_work"

CATEGORY_BY_LEVEL = {
    "L1": "简单单文件修改", "L2": "单文件重构",
    "L3": "多文件+逻辑", "L4": "多文件+外部依赖", "L5": "泛型/并发重构",
}


def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def refine(goal: str, error: str, level: str, task: str) -> dict | None:
    prompt = f"""你是一名资深 Rust 工程师。下面的 agent 在执行代码修改任务时失败了。
请分析失败原因，输出一条结构化经验（JSON，不要多余文字）：

{{
  "problem": "问题描述（50-100字，指出具体技术原因）",
  "solution": "解决方案（50-100字，给出可操作的修复要点）",
  "category": "错误分类（代码修改/编译错误/测试失败/逻辑错误/需求理解）"
}}

任务: {task} (难度 {level})
目标: {goal[:300]}
失败信号: {error[:200]}"""

    data = json.dumps({
        "model": ZHIPU_MODEL,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 400,
        "temperature": 0.3,
    }).encode()
    req = urllib.request.Request(ZHIPU_URL, data=data,
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {ZHIPU_KEY}"})
    for attempt in range(3):
        try:
            resp = urllib.request.urlopen(req, timeout=90)
            body = json.loads(resp.read().decode())
            content = body["choices"][0]["message"]["content"]
            # strip markdown fences
            if "```json" in content:
                content = content.split("```json")[1].split("```")[0]
            elif "```" in content:
                content = content.split("```")[1].split("```")[0]
            parsed = json.loads(content.strip())
            return parsed
        except urllib.error.HTTPError as e:
            if e.code == 429:
                time.sleep(5 + attempt * 5)
                continue
            log(f"  HTTP {e.code}: {e.read().decode()[:120]}")
            return None
        except Exception as e:
            log(f"  attempt {attempt + 1} failed: {e}")
            time.sleep(3)
    return None


def main():
    rows = []
    if SRC.exists():
        for line in SRC.read_text(encoding="utf-8").splitlines():
            if line.strip():
                try:
                    rows.append(json.loads(line))
                except Exception:
                    pass
    fails = [r for r in rows if not r.get("success")]
    log(f"source {SRC.name}: {len(rows)} rows, {len(fails)} fails")

    experiences = []
    for i, r in enumerate(fails, 1):
        task = r.get("task", "?")
        run = r.get("run", 0)
        level = r.get("level", "?")
        error = (r.get("verify_tail") or r.get("error") or "").strip()
        goal_file = TASKS / task / "goal.txt"
        goal = goal_file.read_text(encoding="utf-8").strip() if goal_file.exists() else task

        log(f"[{i}/{len(fails)}] refining {task} r{run} ...")
        refined = refine(goal, error, level, task)
        if not refined:
            log(f"  refine FAILED for {task} — using rule fallback")
            refined = {
                "problem": goal[:200],
                "solution": f"verify_tail={error[:80]}。此任务类型需要精确匹配命名/接口/宏要求，改完必须跑 cargo test。",
                "category": CATEGORY_BY_LEVEL.get(level, "unknown"),
            }
        exp = {
            "id": f"exp-{task}-r{run}-v18-refined",
            "category": refined.get("category", CATEGORY_BY_LEVEL.get(level, "unknown")),
            "problem": refined.get("problem", goal[:200]),
            "solution": refined.get("solution", ""),
            "success": False,
            "effectiveness": 0.85,
            "reference_count": 0,
            "created_at": time.strftime("%Y-%m-%dT%H:%M:%S+08:00"),
            "context": {"os": "linux", "toolchain_version": "0.1.0"},
        }
        experiences.append(exp)
        log(f"  -> {exp['category']} | {exp['problem'][:60]}")

    if not experiences:
        log("no experiences")
        sys.exit(1)

    import paramiko
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)

    jsonl_content = "\n".join(json.dumps(e, ensure_ascii=False) for e in experiences) + "\n"
    ftp = c.open_sftp()
    with ftp.file(f"{REMOTE}/memory/experience.jsonl", "w") as f:
        f.write(jsonl_content)
    ftp.close()
    log(f"injected {len(experiences)} experiences")

    _, so, _ = c.exec_command(
        f"pkill -x service; sleep 2; "
        f"( cd {REMOTE} && setsid nohup ./target/debug/service "
        f"> /home/wutao/service.log 2>&1 < /dev/null & disown ) "
        f"> /dev/null 2>&1; sleep 10; echo OK"
    )
    log(f"restart: {so.read().decode(errors='replace').strip()}")

    time.sleep(3)
    try:
        req = urllib.request.Request("http://192.168.220.131:3000/api/v1/experience/metrics")
        resp = urllib.request.urlopen(req, timeout=10)
        m = json.loads(resp.read().decode())
        log(f"store: {m}")
    except Exception as e:
        log(f"metrics: {e}")

    out = RAW / "v18-refined-experiences.jsonl"
    out.write_text(jsonl_content, encoding="utf-8")
    log(f"wrote {out}")
    c.close()
    log("=== DONE ===")


if __name__ == "__main__":
    main()
