#!/usr/bin/env python3
"""成绩单生成器（M1-1）。

吃 `run_local.py` 产出的 jsonl（活体或 replay 均可），产出 markdown 成绩单：
  · 汇总表——通过率、假完成率（一等指标）、反向不一致、步数/墙钟、每解 token；
  · FAIL 明细——每条判失败的 task + agent 自报 + 判据输出首行（归因起点）。

**口径**（与 bench/judge.py 一致）：
  · 分母「自称做成」= `claim_ok`；假完成率 = `false_completion` / `claim_ok`；
    分母 0 ⇒ None（未知 ≠ 0，不冒充）；
  · 每解 token 只统计 `external_ok` 且 usage 可得的会话（usage 缺失 ⇒ 未知）；
  · `cost_usd` 恒 None（模型无价表条目）时如实记"未知"，不按 0 冒充。

用法：
  python bench/scorecard.py --input 公开集=bench/results/raw/live-agnes-full-v2.jsonl
"""
import argparse
import json
from pathlib import Path


def load_records(path: str) -> list[dict]:
    out = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def _nums(records, key):
    return [r[key] for r in records if isinstance(r.get(key), (int, float))]


def _tokens(records, only_solved: bool):
    out = []
    for r in records:
        if only_solved and not r.get("external_ok"):
            continue
        u = r.get("usage")
        if isinstance(u, dict) and isinstance(u.get("total_tokens"), (int, float)):
            out.append(u["total_tokens"])
    return out


def _fmt_stat(values, fmt="{:.0f}"):
    if not values:
        return "未知"
    return f"{fmt.format(sum(values) / len(values))}（n={len(values)}）"


def _rate(num: int, den: int):
    if den == 0:
        return "未知（分母 0）"
    return f"{num}/{den} = {num / den * 100:.1f}%"


def summarize(name: str, records: list[dict]) -> dict:
    runs = len(records)
    claim_ok = sum(1 for r in records if r.get("claim_ok"))
    ext_ok = sum(1 for r in records if r.get("external_ok"))
    fc = sum(1 for r in records if r.get("false_completion"))
    conflict = sum(1 for r in records if r.get("verdict_conflict"))
    unknown = sum(1 for r in records if r.get("unknown_claim"))
    walls = _nums(records, "wall_s")
    steps = _nums(records, "steps")
    return {
        "name": name,
        "runs": runs,
        "ext_ok": ext_ok,
        "pass_rate": _rate(ext_ok, runs),
        "claim_ok": claim_ok,
        "false_completion": _rate(fc, claim_ok),
        "conflict": conflict,
        "unknown_claim": unknown,
        "avg_steps": _fmt_stat(steps),
        "avg_wall_s": _fmt_stat(walls, "{:.1f}"),
        "tok_per_solve": _fmt_stat(_tokens(records, True)),
        "tok_all": _fmt_stat(_tokens(records, False)),
    }


SUMMARY_COLS = [
    ("套件", "name"),
    ("runs", "runs"),
    ("通过", "pass_rate"),
    ("假完成率", "false_completion"),
    ("反向不一致", "conflict"),
    ("自报未知", "unknown_claim"),
    ("均步数", "avg_steps"),
    ("均墙钟/s", "avg_wall_s"),
    ("每解 tokens", "tok_per_solve"),
    ("全会话 tokens", "tok_all"),
]


def render_summary(rows: list[dict]) -> str:
    lines = ["| " + " | ".join(h for h, _ in SUMMARY_COLS) + " |",
             "|" + "|".join("---" for _ in SUMMARY_COLS) + "|"]
    for row in rows:
        lines.append("| " + " | ".join(str(row[k]) for _, k in SUMMARY_COLS) + " |")
    return "\n".join(lines)


def render_fails(records: list[dict]) -> str:
    fails = [r for r in records if not r.get("external_ok")]
    if not fails:
        return "（无 FAIL）"
    lines = []
    for r in fails:
        tail = (r.get("verify_tail") or "").strip().splitlines()
        first_verify = next(
            (l for l in tail if l.startswith("VERIFY_FAIL")), tail[-1] if tail else ""
        )
        lines.append(
            f"- **{r.get('task')}**：自报={'做成' if r.get('claim_ok') else '没做成/未知'}，"
            f"steps={r.get('steps')}，wall={r.get('wall_s')}s —— `{first_verify[:140]}`"
        )
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description="成绩单生成器（M1-1）")
    ap.add_argument(
        "--input",
        action="append",
        required=True,
        help="套件=路径（可多次）",
    )
    ap.add_argument("--out", default=None, help="另存 markdown 文件")
    a = ap.parse_args()

    rows, fail_sections = [], []
    for spec in a.input:
        name, _, path = spec.partition("=")
        records = load_records(path)
        rows.append(summarize(name, records))
        fail_sections.append((name, render_fails(records)))

    md = render_summary(rows)
    for name, section in fail_sections:
        md += f"\n\n### {name} FAIL 明细\n\n{section}"

    print(md)
    if a.out:
        Path(a.out).write_text(md + "\n", encoding="utf-8")
        print(f"\n[已写入 {a.out}]")


if __name__ == "__main__":
    main()
