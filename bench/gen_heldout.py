#!/usr/bin/env python3
"""held-out 变体生成器（M0 / D-227）。

**为什么需要它**：`bench/tasks/T00~T19` 是**固定的公开集**——对着它反复调优，通过率可以涨
而真实能力不涨（reward hacking 的一种）。业界的做法是"可见 validation 集 + 隐藏 held-out
集"；但**本仓是公开仓**，把 held-out 任务提交上去就等于公开，所以改成：
**只公开生成器，实例按种子现场生成**（`bench/tasks-heldout/` 与 `bench/solutions-heldout/`
都已加入 `.gitignore`）。

**诚实的局限（必须记住，别当成"保密"）**：
  1. 生成器公开 ⇒ 有动机的人可以枚举它的输出；它挡的是"**无意**把 held-out 当开发集用"，
     不是"蓄意对抗"。
  2. 每个家族只有一种"技能形状"，变的是标识符与常量、不是任务类型。通过率只在同族变体上
     上升，**不能**推断泛化到新任务类型。
  3. 家族数量有限（当前 4 个），覆盖面远小于公开集。

**判据一律纯行为级**（`vh_require_cargo_test`）：fixture 自带失败测试，修对才绿——
刻意不使用"grep 关键字"这类可被注释骗过的判据。

产物：
  <out>/<TID>/{goal.txt, meta.json, verify.sh, fixture/{Cargo.toml, src/lib.rs}}
  <solutions-out>/<TID>.sh      已知正解（与 bench/solutions/ 同约定：在工作区根执行）
  <out>/GENESIS.json            seed / count / 每任务家族（可复现、可审计）
"""
import argparse
import json
import random
import shutil
import sys
from pathlib import Path

BENCH = Path(__file__).resolve().parent

VERIFY_TEMPLATE = """#!/bin/bash
# __TID__ 判据（held-out 生成；生成器见 bench/gen_heldout.py）
# 纯行为级：fixture 自带失败测试，修对才绿。
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_require_cargo_test
vh_pass __TID__
"""

CARGO_TEMPLATE = """[package]
name = "__CRATE__"
version = "0.1.0"
edition = "2021"

[workspace]
"""

SOLUTION_TEMPLATE = """#!/bin/bash
# __TID__ 参考解（held-out 生成）
set -euo pipefail
cat > src/lib.rs <<'RUST'
__SOLUTION__RUST
grep -q '__MARKER__' src/lib.rs || { echo "solution(__TID__): 写入校验失败" >&2; exit 1; }
"""

# ---------------------------------------------------------------- 家族定义

FIX_BOUND_PRISTINE = """pub fn __FN__(n: u64) -> u64 {
    let mut acc = 0u64;
    for k in 1..n {
        acc += k;
    }
    acc
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_sum() { assert_eq!(__FN__(__N__), __EXPECTED__); }
}
"""

CHANGE_CONST_PRISTINE = """pub const __NAME__: u64 = __OLD__;
pub fn __FN__() -> u64 {
    __NAME__
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_value() { assert_eq!(__FN__(), __NEW__); }
}
"""

ADD_FN_PRISTINE = """#[cfg(test)] mod tests { use super::*;
  #[test] fn t_impl() { assert_eq!(__FN__(__ARGS__), __EXPECTED__); }
}
"""

ADD_FN_SOLUTION_PREFIX = """pub fn __FN____SIG__ {
    __BODY__
}
"""

FIX_LOGIC_PRISTINE = """pub fn __FN__(x: i32) -> bool {
    x > 0
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_zero() { assert!(__FN__(0)); }
  #[test] fn t_pos() { assert!(__FN__(5)); }
}
"""

# (名字后缀, 形参签名, 调用实参, 期望值字面量, 函数体)
ADD_FN_CASES = [
    ("double", "(x: i32) -> i32", "3", "6", "x * 2"),
    ("abs_diff", "(a: i32, b: i32) -> i32", "3, 8", "5", "(a - b).abs()"),
    ("count_char", "(s: &str, c: char) -> usize", '"banana", \'a\'', "3", "s.matches(c).count()"),
]

CONST_CASES = [
    ("MAX_RETRIES", 3, 5),
    ("PAGE_SIZE", 10, 20),
    ("TIMEOUT_SECS", 30, 60),
    ("BUFFER_ITEMS", 100, 200),
]


def _fill(tpl: str, **kw) -> str:
    out = tpl
    for k, v in kw.items():
        out = out.replace(f"__{k}__", str(v))
    return out


def fam_fix_bound(rnd: random.Random, i: int):
    n = rnd.choice([10, 12, 15, 20, 25, 30])
    fn = f"sum_to_{i}"
    expected = n * (n + 1) // 2
    pristine = _fill(FIX_BOUND_PRISTINE, FN=fn, N=n, EXPECTED=expected)
    solution = pristine.replace("for k in 1..n {", "for k in 1..=n {")
    assert solution != pristine, "fix-bound 补丁未生效"
    return dict(
        family="fix-bound",
        marker=fn,
        goal=(
            f"src/lib.rs 的 {fn} 有 off-by-one 错误：{fn}({n}) 应返回 {expected}。"
            "修复它，cargo test 全绿。"
        ),
        acceptance=f"{fn}({n}) == {expected}（行为级：cargo test 全绿）",
        level="L3",
        pristine=pristine,
        solution=solution,
    )


def fam_change_const(rnd: random.Random, i: int):
    name, old, new = rnd.choice(CONST_CASES)
    fn = f"read_{i}"
    pristine = _fill(CHANGE_CONST_PRISTINE, NAME=name, OLD=old, NEW=new, FN=fn)
    solution = pristine.replace(f"{name}: u64 = {old}", f"{name}: u64 = {new}")
    assert solution != pristine, "change-const 补丁未生效"
    return dict(
        family="change-const",
        marker=name,
        goal=(
            f"把常量 {name} 从 {old} 改成 {new}（测试断言已按新值写好）。改完 cargo test 全绿。"
        ),
        acceptance=f"{name} == {new}（行为级：cargo test 全绿）",
        level="L2",
        pristine=pristine,
        solution=solution,
    )


def fam_add_fn(rnd: random.Random, i: int):
    suffix, sig, args, expected, body = rnd.choice(ADD_FN_CASES)
    fn = f"{suffix}_{i}"
    pristine = _fill(ADD_FN_PRISTINE, FN=fn, ARGS=args, EXPECTED=expected)
    solution = _fill(ADD_FN_SOLUTION_PREFIX, FN=fn, SIG=sig, BODY=body) + _fill(
        ADD_FN_PRISTINE, FN=fn, ARGS=args, EXPECTED=expected
    )
    return dict(
        family="add-fn",
        marker=fn,
        goal=(
            f"src/lib.rs 里有一个测试调用了尚不存在的函数 {fn}。请实现它"
            f"（签名 `pub fn {fn}{sig}`），使 cargo test 全绿。不要改测试。"
        ),
        acceptance=f"{fn} 实现正确（行为级：cargo test 全绿）",
        level="L3",
        pristine=pristine,
        solution=solution,
    )


def fam_fix_logic(rnd: random.Random, i: int):
    fn = f"is_valid_{i}"
    pristine = _fill(FIX_LOGIC_PRISTINE, FN=fn)
    solution = pristine.replace("x > 0", "x >= 0")
    assert solution != pristine, "fix-logic 补丁未生效"
    return dict(
        family="fix-logic",
        marker=fn,
        goal=(
            f"{fn} 对 x=0 返回了 false，但 0 应当被接受。修复判断条件，cargo test 全绿。"
        ),
        acceptance=f"{fn}(0) == true 且 {fn}(5) == true（行为级：cargo test 全绿）",
        level="L2",
        pristine=pristine,
        solution=solution,
    )


FAMILIES = {
    "fix-bound": fam_fix_bound,
    "change-const": fam_change_const,
    "add-fn": fam_add_fn,
    "fix-logic": fam_fix_logic,
}


def generate(seed: int, count: int, out: Path, solutions_out: Path, clean: bool = True):
    rnd = random.Random(seed)
    if clean:
        for d in (out, solutions_out):
            if d.exists():
                shutil.rmtree(d)
    out.mkdir(parents=True, exist_ok=True)
    solutions_out.mkdir(parents=True, exist_ok=True)

    names = list(FAMILIES)
    rnd.shuffle(names)
    genesis = []
    for i in range(1, count + 1):
        family = names[(i - 1) % len(names)]
        spec = FAMILIES[family](rnd, i)
        tid = f"H{i:02d}-{family}"
        task_dir = out / tid
        (task_dir / "fixture" / "src").mkdir(parents=True, exist_ok=True)

        (task_dir / "goal.txt").write_text(spec["goal"] + "\n", encoding="utf-8")
        (task_dir / "verify.sh").write_text(
            _fill(VERIFY_TEMPLATE, TID=tid), encoding="utf-8"
        )
        (task_dir / "meta.json").write_text(
            json.dumps(
                {
                    "level": spec["level"],
                    "expect_files_changed": 1,
                    "timeout_min": 8,
                    "suite": "held-out",
                    "acceptance": spec["acceptance"],
                    "heldout_family": family,
                    "heldout_seed": seed,
                },
                ensure_ascii=False,
            )
            + "\n",
            encoding="utf-8",
        )
        (task_dir / "fixture" / "Cargo.toml").write_text(
            _fill(CARGO_TEMPLATE, CRATE=f"heldout_{i:02d}"), encoding="utf-8"
        )
        (task_dir / "fixture" / "src" / "lib.rs").write_text(spec["pristine"], encoding="utf-8")

        sol = _fill(
            SOLUTION_TEMPLATE,
            TID=tid,
            SOLUTION=spec["solution"],
            MARKER=spec["marker"],
        )
        (solutions_out / f"{tid}.sh").write_text(sol, encoding="utf-8")

        genesis.append({"task": tid, "family": family, "marker": spec["marker"]})

    (out / "GENESIS.json").write_text(
        json.dumps(
            {"seed": seed, "count": count, "families": sorted(FAMILIES), "tasks": genesis},
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return genesis


def main(argv=None):
    ap = argparse.ArgumentParser(description="held-out 变体生成器（M0/D-227）")
    ap.add_argument("--seed", type=int, default=20261018, help="随机种子（决定实例）")
    ap.add_argument("--count", type=int, default=8, help="生成任务数")
    ap.add_argument("--out", type=Path, default=BENCH / "tasks-heldout")
    ap.add_argument(
        "--solutions-out", type=Path, default=BENCH / "solutions-heldout"
    )
    ap.add_argument(
        "--no-clean",
        action="store_true",
        help="不先清空输出目录（默认会清空，避免旧实例残留影响判据）",
    )
    a = ap.parse_args(argv)
    if a.count <= 0:
        print("count 必须为正", file=sys.stderr)
        return 2
    tasks = generate(a.seed, a.count, a.out, a.solutions_out, clean=not a.no_clean)
    print(f"已生成 {len(tasks)} 个 held-out 任务 -> {a.out}")
    print(f"参考解 -> {a.solutions_out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
