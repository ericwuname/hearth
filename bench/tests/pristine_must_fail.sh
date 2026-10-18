#!/bin/bash
# ============================================================================
# bench/tests/pristine_must_fail.sh —— 判据双向门禁（M0 / D-222 + D-225）
#
# 判据只有两种作弊方式，两个方向都堵：
#   ① 恒真：未做任何修复（fixture 原样）却判通过   → 必须 FAIL（覆盖**全部**任务）
#   ② 恒假：做对了却判失败                        → 用 bench/solutions/<task>.sh 施加
#                                                   参考解，必须判通过
#
# 为什么必须有 ②：只证"没做会判失败"，无法排除"判据过严导致正确解也失败"——那样
# 通过率会被系统性低估，且 M1 的"稳定产出基线"就分不清"agent 不行"与"判据有问题"。
#
# **覆盖率是如实登记的**：没有参考解的任务会在最后**逐条列出**，不静默跳过。
#
# 复刻 runner 的真实协议：把 fixture 内容**摊平**到工作区，verify.sh 写成
# `__verify.sh` 放进工作区（因为每个 verify.sh 都 `cd "$(dirname "$0")"`），
# 并把判据库 verify_lib.sh 一并放进去。
#
# 退出码：全部期望满足 => 0；任一不满足 => 1。
#
# 已知局限：① 本门禁不证明"判据充分覆盖目标"（仍需 held-out 套件与参考解全集）；
# ② T17 竞态类单次 cargo test 绿不等于无竞态。
# ============================================================================
set -uo pipefail

BENCH="$(cd "$(dirname "$0")/.." && pwd)"
# 根可被环境变量覆盖：held-out 实例生成在别处（D-227），同一套双向门禁直接复用。
# 见 bench/gen_heldout.py。
TASKS="${BENCH_TASKS_ROOT:-$BENCH/tasks}"
SOLUTIONS="${BENCH_SOLUTIONS_ROOT:-$BENCH/solutions}"
LIB="$BENCH/verify_lib.sh"
: "${CARGO_TARGET_DIR:=${TMPDIR:-/tmp}/hearth-bench-verify-target}"
export CARGO_TARGET_DIR
export CARGO_TERM_COLOR=never
# ONLY=<子串>：只跑名字含该子串的任务（本地快速自查用；CI 不设则跑全部）。
: "${ONLY:=}"
# REQUIRE_FULL_COVERAGE=1：要求**每个**任务都有参考解（CI 用；本地默认关，便于
# 新任务的作者先只跑 ① 方向）。没有这条，覆盖率会在新增任务时被**静默**摊薄。
: "${REQUIRE_FULL_COVERAGE:=0}"

if [ ! -f "$LIB" ]; then
  echo "GATE-ERROR: 缺少判据库 $LIB" >&2
  exit 1
fi

# prepare <taskdir> <目标目录>：按 runner 协议搭出可执行工作区。
prepare() {
  local taskdir="$1" w="$2"
  rm -rf "$w"
  mkdir -p "$w"
  cp -r "$taskdir/fixture/." "$w/"
  cp "$taskdir/verify.sh" "$w/__verify.sh"
  cp "$LIB" "$w/verify_lib.sh"
}

# run_verify <工作目录>：执行判据；回显输出，退出码由 $? 传出。
run_verify() {
  local w="$1"
  ( cd "$w" && bash __verify.sh 2>&1 )
}

# 判据输出里是否出现**独立成行**的通过标记。
has_pass_marker() {
  printf '%s\n' "$1" | grep -qE '^[[:space:]]*VERIFY_PASS'
}

task_list() {
  # 任务目录 = 同时具备 `fixture/` + `goal.txt` + `verify.sh` 的直接子目录。
  # 不用 `T*/` 这类命名 glob：held-out 实例叫 `H01-…`（D-227），名字前缀不该被门禁绑死。
  for taskdir in "$TASKS"/*/; do
    [ -d "$taskdir/fixture" ] || continue
    [ -f "$taskdir/goal.txt" ] || continue
    [ -f "$taskdir/verify.sh" ] || continue
    local tid
    tid="$(basename "$taskdir")"
    if [ -n "$ONLY" ] && [[ "$tid" != *"$ONLY"* ]]; then
      continue
    fi
    printf '%s\n' "$taskdir"
  done
}

failures=0
checked=0

echo "=== ① 恒真检查：未修复的 fixture 必须判失败 ==="
while IFS= read -r taskdir; do
  [ -n "$taskdir" ] || continue
  tid="$(basename "$taskdir")"
  w="$(mktemp -d)"
  prepare "$taskdir" "$w"

  out=""
  rc=0
  out="$(run_verify "$w")" || rc=$?
  checked=$((checked + 1))

  if [ "$rc" -eq 0 ] || has_pass_marker "$out"; then
    echo "GATE-FAIL $tid：未修复的 fixture 竟判通过（rc=$rc）"
    echo "---- $tid 输出尾部 ----"
    printf '%s\n' "$out" | tail -5
    echo "-----------------------"
    failures=$((failures + 1))
  else
    echo "ok  $tid：未修复 ⇒ 判失败（rc=$rc）"
  fi

  rm -rf "$w"
done < <(task_list)

echo
echo "=== ② 恒假检查：有参考解的任务，施加参考解后必须判通过 ==="
covered=0
uncovered=()
while IFS= read -r taskdir; do
  [ -n "$taskdir" ] || continue
  tid="$(basename "$taskdir")"
  sol="$SOLUTIONS/$tid.sh"
  if [ ! -f "$sol" ]; then
    uncovered+=("$tid")
    continue
  fi
  covered=$((covered + 1))
  w="$(mktemp -d)"
  prepare "$taskdir" "$w"

  if ! ( cd "$w" && bash "$sol" >/dev/null 2>&1 ); then
    echo "GATE-FAIL $tid：参考解施加失败（fixture 形状可能已变；见 bench/solutions/$tid.sh）"
    failures=$((failures + 1))
    rm -rf "$w"
    continue
  fi

  out=""
  rc=0
  out="$(run_verify "$w")" || rc=$?
  if [ "$rc" -eq 0 ] && has_pass_marker "$out"; then
    echo "ok  $tid：参考解 ⇒ 判通过（rc=0）"
  else
    echo "GATE-FAIL $tid：参考解却判失败（rc=$rc）"
    printf '%s\n' "$out" | tail -8
    failures=$((failures + 1))
  fi

  rm -rf "$w"
done < <(task_list)

echo
echo "正向覆盖：$covered 个任务有参考解；未覆盖 ${#uncovered[@]} 个（如实登记，不静默跳过）"
if [ "${#uncovered[@]}" -gt 0 ]; then
  echo "  未覆盖：${uncovered[*]}"
  if [ "$REQUIRE_FULL_COVERAGE" = "1" ]; then
    echo "GATE-FAIL：REQUIRE_FULL_COVERAGE=1 要求每个任务都有参考解（缺参考解 = 该任务" >&2
    echo "           \"判据是否过严\"未被证明，正向方向对它仍无防护）" >&2
    failures=$((failures + 1))
  fi
fi

echo
if [ "$failures" -eq 0 ]; then
  echo "GATE-PASS：① 检查 $checked 个任务均未修复必判失败；② 参考解 $covered 个均判通过"
  exit 0
fi
echo "GATE-FAIL：共 $failures 项不满足" >&2
exit 1
