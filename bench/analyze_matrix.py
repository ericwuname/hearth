#!/usr/bin/env python3
"""v13 S5: turn the raw provider-matrix jsonl files into a comparison report.

Emits the pass/cost/duration triple per provider plus a failure funnel keyed by
the verify.sh tail, and a per-task cross-provider grid so a task that only one
provider fails is visible at a glance.

Providers are split into two pools on purpose:

* RANKED    -- healthy channels, comparable, they carry the capability ranking.
* DEGRADED  -- channels known to be unhealthy at run time (agnes was on the free
               fallback lane during v13, see ~/.workbuddy/MEMORY.md). Their runs
               measure *channel stability*, not model capability, so mixing them
               into the same pass-rate table would misread as "weak model".

Usage: python analyze_matrix.py [--out results/provider-matrix-v13.md]
"""
import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw"

RANKED = ["zhipu", "deepseek"]
DEGRADED = {
    "agnes": (
        "Agnes free fallback lane (membership line down on 2026-07-30). "
        "service.log shows repeated `error sending request for url "
        "(https://api.agnes-ai.cn/v1/chat/completions)` plus `no tool_calls after "
        "5 retries -- entering Done`, i.e. the run terminates before the task is "
        "finished. Tool wiring itself is proven live (`execute_tool_calls "
        "tools=[\"glob\"|\"grep\"|\"read\"]` present in the same sessions)."
    )
}
ALL_PROVIDERS = RANKED + list(DEGRADED)

LEVELS = ["L1", "L2", "L3", "L4", "L5"]

MARKERS = ("NO_DERIVE", "TEST_FAIL", "COMPILE_FAIL", "NOT_FOUND", "NO_FILE",
           "NO_GENERIC_FN", "NO_BENCH", "NO_TEST", "NO_CHANGE", "TIMEOUT",
           "VERIFY_FAIL")


def load(provider):
    """Load one provider's runs, de-duplicating (task, run) keeping the last write."""
    f = RAW / f"matrix-v13-{provider}.jsonl"
    if not f.exists():
        return []
    by_key = {}
    with open(f, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            by_key[(rec.get("task"), rec.get("run"))] = rec
    return list(by_key.values())


def failure_reason(rec):
    """Classify a failure from the verify tail into a funnel bucket."""
    tail = (rec.get("verify_tail") or "").strip()
    if not tail:
        return "NO_OUTPUT"
    for marker in MARKERS:
        if marker in tail:
            return marker
    last = [ln for ln in tail.splitlines() if ln.strip()]
    return (last[-1][:40] if last else "UNKNOWN")


def fmt_pct(n, d):
    return f"{100 * n / d:.0f}%" if d else "n/a"


def stats_row(p, rs):
    ok = sum(1 for r in rs if r.get("success"))
    durs = sorted(r.get("wall_s", 0) for r in rs)
    med = durs[len(durs) // 2] if durs else 0
    mean = sum(durs) / len(durs) if durs else 0
    return (f"| {p} | {len(rs)} | {ok} | {fmt_pct(ok, len(rs))} | "
            f"{med:.1f} | {mean:.1f} | {sum(durs) / 60:.1f} |")


def funnel(lines, p, rs):
    fails = [r for r in rs if not r.get("success")]
    lines.append(f"**{p}** -- {len(fails)} failures")
    if not fails:
        lines += ["", "- (none)", ""]
        return
    c = Counter(failure_reason(r) for r in fails)
    lines.append("")
    for reason, n in c.most_common():
        tasks = sorted({r["task"] for r in fails if failure_reason(r) == reason})
        lines.append(f"- `{reason}` x{n} -- {', '.join(tasks)}")
    lines.append("")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(HERE / "results" / "provider-matrix-v13.md"))
    a = ap.parse_args()

    data = {p: load(p) for p in ALL_PROVIDERS}
    have = [p for p in RANKED if data[p]]
    degraded = [p for p in DEGRADED if data[p]]

    lines = ["# v13 S4/S5 -- Cross-Provider Contrast Matrix", "",
             "Ranked pool: " + (", ".join(have) or "none") +
             " | Degraded pool (not ranked): " + (", ".join(degraded) or "none"), ""]

    # --- headline triple -------------------------------------------------
    lines += ["## 1. Pass / Duration / Steps (ranked pool)", "",
              "| provider | runs | pass | pass rate | median s | mean s | total min |",
              "|---|---|---|---|---|---|---|"]
    for p in have:
        lines.append(stats_row(p, data[p]))
    lines.append("")

    # --- per difficulty level -------------------------------------------
    lines += ["## 2. Pass rate by difficulty level (ranked pool)", "",
              "| provider | " + " | ".join(LEVELS) + " |",
              "|---|" + "---|" * len(LEVELS)]
    for p in have:
        cells = []
        for lv in LEVELS:
            sub = [r for r in data[p] if r.get("level") == lv]
            ok = sum(1 for r in sub if r.get("success"))
            cells.append(f"{ok}/{len(sub)}" if sub else "-")
        lines.append(f"| {p} | " + " | ".join(cells) + " |")
    lines.append("")

    # --- failure funnel ---------------------------------------------------
    lines += ["## 3. Failure funnel (ranked pool)", ""]
    for p in have:
        funnel(lines, p, data[p])

    # --- per-task grid ----------------------------------------------------
    all_tasks = sorted({r["task"] for p in have for r in data[p]})
    by_task = defaultdict(dict)
    level_of = {}
    for p in ALL_PROVIDERS:
        for r in data[p]:
            by_task[r["task"]][p] = r
            level_of.setdefault(r["task"], r.get("level", "?"))
    lines += ["## 4. Per-task grid (ranked pool)", "",
              "| task | level | " + " | ".join(have) + " |",
              "|---|---|" + "---|" * len(have)]
    for t in all_tasks:
        cells = []
        for p in have:
            r = by_task[t].get(p)
            cells.append("-" if not r else
                         ("PASS " if r.get("success") else "FAIL ") + f"{r.get('wall_s', 0):.0f}s")
        lines.append(f"| {t} | {level_of.get(t, '?')} | " + " | ".join(cells) + " |")
    lines.append("")

    # --- divergences ------------------------------------------------------
    diverge = []
    for t in all_tasks:
        res = {p: by_task[t][p].get("success") for p in have if p in by_task[t]}
        if len(set(res.values())) > 1:
            diverge.append((t,
                            [p for p, v in res.items() if v],
                            [p for p, v in res.items() if not v]))
    lines += ["## 5. Divergences (provider-specific weakness, ranked pool)", ""]
    if diverge:
        for t, ps, fs in diverge:
            lines.append(f"- **{t}** ({level_of.get(t, '?')}): "
                         f"pass={', '.join(ps) or 'none'} / fail={', '.join(fs) or 'none'}")
    else:
        lines.append("- none (all ranked providers agree on every task)")
    lines.append("")

    # --- degraded pool ----------------------------------------------------
    lines += ["## 6. Degraded channels (NOT a capability ranking)", ""]
    if not degraded:
        lines += ["- none", ""]
    for p in degraded:
        rs = data[p]
        lines += [f"### {p}", "", DEGRADED[p], "",
                  "| provider | runs | pass | pass rate | median s | mean s | total min |",
                  "|---|---|---|---|---|---|---|",
                  stats_row(p, rs), ""]
        funnel(lines, p, rs)
        # tasks the ranked pool solved but this channel did not -> channel loss
        lost = []
        for t in all_tasks:
            r = by_task[t].get(p)
            if r is None or r.get("success"):
                continue
            ranked_ok = [q for q in have
                         if by_task[t].get(q) and by_task[t][q].get("success")]
            if len(ranked_ok) == len(have) and have:
                lost.append(t)
        lines += [f"- tasks every ranked provider solved but `{p}` lost: "
                  f"**{len(lost)}** ({', '.join(lost) if lost else 'none'})",
                  "- reading: these are channel losses, not model losses.", ""]

    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"WROTE {out}")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
