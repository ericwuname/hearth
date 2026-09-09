#!/usr/bin/env python3
"""P2 Node 01: chars<->tokens calibration on real Hearth workload samples.
Measures per corpus: naive_chars, estimate_koujing (Rust Debug+bytes replication),
API prompt_tokens (authoritative). Provider: Agnes agnes-2.5-flash.
"""
import json, urllib.request, os, sys, re

API = "https://api.agnes-ai.cn/v1/chat/completions"
KEY = "cpk-f4UBH3NaHUUN2SAcW1LhZT0kyuqlu9TF5lTItbuXHSTP3w0c"
MODEL = "agnes-2.5-flash"
REPO = r"C:/Users/87465/Desktop/codex-rust-v1.0-final"

def rust_debug_len(s: str) -> int:
    """Replicates Rust format!(\"{:?}\", content).len() for String content:
    wraps in quotes, escape_debug on \\ \" \\n \\r \\t; counts UTF-8 BYTES."""
    e = s.replace("\\", "\\\\").replace('"', '\\"')
    e = e.replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")
    dbg = '"' + e + '"'
    return len(dbg.encode("utf-8"))

def tokens_via_api(text: str) -> int:
    body = json.dumps({
        "model": MODEL,
        "messages": [{"role": "user", "content": text}],
        "max_tokens": 1,
    }).encode()
    req = urllib.request.Request(API, data=body, headers={
        "Content-Type": "application/json",
        "Authorization": f"Bearer {KEY}",
    })
    with urllib.request.urlopen(req, timeout=120) as r:
        data = json.loads(r.read().decode())
    return data["usage"]["prompt_tokens"]

def read(p):
    with open(p, encoding="utf-8", errors="replace") as f:
        return f.read()

corpora = {}

# system prompt 代表：constitution.md（system 层组成件）
corpora["system_constitution"] = read(os.path.join(REPO, "constitution.md"))
# Hearth.md / 项目说明
for cand in ["Hearth.md", "hearth.md"]:
    p = os.path.join(REPO, cand)
    if os.path.exists(p):
        corpora["hearth_md"] = read(p)
        break
# code: agent-core lib.rs 前 500 行
code = read(os.path.join(REPO, "crates/agent-core/src/loop.rs"))
corpora["code_rust"] = "\n".join(code.splitlines()[:500])
# 中文用户对话（FA01 真机 goal）
corpora["chinese_dialog"] = """创建一个 Rust 库项目 calcpkg/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p calcpkg/src
2) 用 write_file 创建 calcpkg/Cargo.toml，内容为 [package] name=calcpkg version=0.1.0 edition=2021
3) 用 write_file 创建 calcpkg/src/lib.rs，其中包含 pub fn shout(s: &str) -> String { s.to_string() }（故意留一个 bug：未实现大写与感叹号），以及测试模块
4) 用 bash 执行 cargo test --manifest-path calcpkg/Cargo.toml —— 这次测试会失败（受控失败），记录失败输出
5) 用 write_file 修复 calcpkg/src/lib.rs 中的 shout 实现（改为 format!("{}!", s.to_uppercase())），测试保持不变
6) 用 bash 再执行 cargo test --manifest-path calcpkg/Cargo.toml 确认通过
7) 用 write_file 创建 RESULT.txt，内容写 CONTROLLED_FAILURE_RECOVERED
8) 用 bash 执行 cat RESULT.txt 确认。以上步骤完成后立即宣告任务完成（DONE），不要做额外验证
什么是所有权？继续。查看状态。这个任务现在完成了吗？"""
# 英文日志/tool result（真机 cargo test 输出样本）
corpora["english_tool_result"] = """   Compiling mathlib v0.1.0 (/tmp/fa_node12/mathlib)
error[E0425]: cannot find function `add` in this scope
 --> src/lib.rs:8:29
  |
8 |     fn t_add() { assert_eq!(add(2, 3), 5); }
  |                             ^^^ not found in this scope
help: consider importing this function
  |
7 +     use crate::add;
  |
error: could not compile `mathlib` (lib test) due to 2 previous errors
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
warning: unused variable: `x` --> src/main.rs:12:9 note: `#[warn(unused_variables)]` on by default
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.44s
Running unittests src/lib.rs (mathlib/target/debug/deps/mathlib-20a17a74b6b24878)
thread 'tests::test_add' panicked at 'assertion failed: `(left == right)` left: `6`, right: `5`'"""
# mixed（真机混合：规划草案/反思/状态 JSON）
corpora["mixed"] = """┌─ 🗺 规划草案（steps=8 gaps=0 待问=0 自动假设=0）
   · Use bash to create the directory structure calcpkg/src via mkdir -p [Pending]
   · Create calcpkg/Cargo.toml with package definition for calcpkg version 0.1.0 edition 2021 [Pending]
└─
💭 [reflect] 正在反思进度
2026-08-30T09:02:05.815171Z  WARN sandbox::linux_impl: cannot create cgroup codex-sandbox-231425: Permission denied (os error 13) — HEARTH_ALLOW_NO_CGROUP=1 显式降级
{"budget_max_steps": 50, "context_fill_pct": 46, "history_chars": 10058, "phase": "Init", "provider": "agnes", "steps_used": 36, "system_chars": 4796}
📄 产物 file: mathlib/src/lib.rs（+13 行）
✓ Task completed（46 步）——目标达成，产物见报告"""
# TaskGraph / Task Continuity 代表（planner 输出形态）
corpora["taskgraph_continuity"] = """## Task Topology (structure only; live status comes in Task Continuity):
  t1 - Use bash to execute mkdir -p mathlib/src (deps: )
  t2 - Use write_file to create mathlib/Cargo.toml (deps: t1)
  t3 - Use write_file to create mathlib/src/lib.rs (deps: t2)
  t4 - Use bash to execute cargo test and record the failure output (deps: t3)
## Task Continuity
goal: 创建一个 Rust 库项目 mathlib/，按顺序执行受控失败与修复
completed: t1, t2, t3
remaining: t4, t5, t6
next_action: t4 用 bash 执行 cargo test --manifest-path mathlib/Cargo.toml
artifacts: mathlib/Cargo.toml, mathlib/src/lib.rs, RESULT.txt
verification: acceptance_result=passed (cmd: cargo test ... exit 0)
user_constraints: 不要做额外验证；完成后立即宣告 DONE
[compacted 会话摘要] 轮次目标: 创建 mathlib 项目并修复 add/shout bug | 工具: bash, write_file | 写盘: mathlib/src/lib.rs, RESULT.txt"""

rows = []
for name, text in corpora.items():
    naive = len(text)                       # naive chars
    est = rust_debug_len(text)              # estimate_chars 口径 (Debug + bytes)
    tok = tokens_via_api(text)              # authoritative
    rows.append((name, naive, est, tok))
    print(f"{name:24s} chars={naive:6d} est口径={est:6d} tokens={tok:6d} "
          f"chars/token={naive/tok:6.2f} est/tokens={est/tok:6.2f} est_inflation={est/naive:4.2f}x")

# combined prompt (all corpora concatenated)
combined = "\n\n".join(corpora.values())
naive = len(combined); est = rust_debug_len(combined); tok = tokens_via_api(combined)
print(f"{'COMBINED':24s} chars={naive:6d} est口径={est:6d} tokens={tok:6d} "
      f"chars/token={naive/tok:6.2f} est/tokens={est/tok:6.2f} est_inflation={est/naive:4.2f}x")
rows.append(("COMBINED", naive, est, tok))

# 32k threshold conversion under both口径
print("\n=== 32,000 阈值换算（combined workload 口径）===")
c_per_tok = rows[-1][1] / rows[-1][3]
est_per_tok = rows[-1][2] / rows[-1][3]
print(f"naive chars/token = {c_per_tok:.2f}  → 32,000 chars ≈ {32000/c_per_tok:,.0f} tokens")
print(f"est口径 chars/token = {est_per_tok:.2f}  → 32,000 est-chars ≈ {32000/est_per_tok:,.0f} tokens")
print(f"est口径 32,000 = naive chars {32000/ (rows[-1][2]/rows[-1][1]):,.0f}（Debug+bytes 虚增后实际只代表这么多的原始字符）")
