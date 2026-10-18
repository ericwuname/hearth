#!/bin/bash
# ============================================================================
# bench/tests/pristine_must_fail.sh —— 「判据不得恒真」门禁（M0 / D-222）
#
# 判据只有两种作弊方式，本门禁两个方向都堵：
#   ① 恒真：未做任何修复（fixture 原样）却判通过  → 必须 FAIL（主检查，覆盖全部任务）
#   ② 恒假：做对了却判失败                        → 用 T03 的**已知正解**做反向对照
#
# 复刻 runner 的真实协议：把 fixture 内容**摊平**到工作区，verify.sh 写成
# `__verify.sh` 放进工作区（因为每个 verify.sh 都 `cd "$(dirname "$0")"`），
# 并把判据库 verify_lib.sh 一并放进去。
#
# 退出码：全部期望满足 => 0；任一不满足 => 1。
#
# 已知局限（如实登记，不掩盖）：本门禁只证明「未修复必判失败」，**不证明**
# 「判据充分覆盖目标」——完整覆盖仍需 held-out 套件与参考解（任务书 M0-4 / M1）。
# ============================================================================
set -uo pipefail

BENCH="$(cd "$(dirname "$0")/.." && pwd)"
TASKS="$BENCH/tasks"
LIB="$BENCH/verify_lib.sh"
: "${CARGO_TARGET_DIR:=${TMPDIR:-/tmp}/hearth-bench-verify-target}"
export CARGO_TARGET_DIR
export CARGO_TERM_COLOR=never
# ONLY=<子串>：只跑名字含该子串的任务（本地快速自查用；CI 不设则跑全部）。
: "${ONLY:=}"

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

failures=0
checked=0

echo "=== ① 恒真检查：未修复的 fixture 必须判失败 ==="
for taskdir in "$TASKS"/T*/; do
  [ -f "$taskdir/goal.txt" ] || continue
  tid="$(basename "$taskdir")"
  if [ -n "$ONLY" ] && [[ "$tid" != *"$ONLY"* ]]; then
    continue
  fi
  w="$(mktemp -d)"
  prepare "$taskdir" "$w"

  out=""
  rc=0
  out="$(run_verify "$w")" || rc=$?
  checked=$((checked + 1))

  has_pass=0
  if printf '%s\n' "$out" | grep -qE '^[[:space:]]*VERIFY_PASS'; then
    has_pass=1
  fi

  if [ "$rc" -eq 0 ] || [ "$has_pass" -eq 1 ]; then
    echo "GATE-FAIL $tid：未修复的 fixture 竟判通过（rc=$rc，输出含 VERIFY_PASS=$has_pass）"
    echo "---- $tid 输出尾部 ----"
    printf '%s\n' "$out" | tail -5
    echo "-----------------------"
    failures=$((failures + 1))
  else
    echo "ok  $tid：未修复 ⇒ 判失败（rc=$rc）"
  fi

  # 只保留尾部便于排查，同时避免临时目录堆积
  rm -rf "$w"
done

echo
echo "=== ② 恒假检查（反向对照）：T03 的已知正解必须判通过 ==="
T03="$TASKS/T03-fix-off-by-one"
if [ -f "$T03/goal.txt" ]; then
  w="$(mktemp -d)"
  prepare "$T03" "$w"
  # 已知正解：把 off-by-one 的循环上界从 `1..n`（n-1 次）改为 `1..=n`（n 次）。
  if ! grep -q 'for _ in 1\.\.n {' "$w/src/lib.rs"; then
    echo "GATE-FAIL T03 反向对照：fixture 形状已变，正解补丁无法施加（需人工更新门禁）"
    failures=$((failures + 1))
  else
    sed -i 's/for _ in 1\.\.n {/for _ in 1..=n {/' "$w/src/lib.rs"
    out=""
    rc=0
    out="$(run_verify "$w")" || rc=$?
    if [ "$rc" -eq 0 ] && printf '%s\n' "$out" | grep -qE '^[[:space:]]*VERIFY_PASS'; then
      echo "ok  T03：已知正解 ⇒ 判通过（rc=0）"
    else
      echo "GATE-FAIL T03 反向对照：已知正解却判失败（rc=$rc）"
      printf '%s\n' "$out" | tail -8
      failures=$((failures + 1))
    fi
  fi
  rm -rf "$w"
else
  echo "GATE-FAIL：找不到 T03-fix-off-by-one（反向对照缺失）"
  failures=$((failures + 1))
fi

echo
if [ "$failures" -eq 0 ]; then
  echo "GATE-PASS：共检查 $checked 个任务；无恒真判据，且 T03 反向对照成立"
  exit 0
fi
echo "GATE-FAIL：共 $failures 项不满足" >&2
exit 1
