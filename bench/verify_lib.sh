#!/bin/bash
# ============================================================================
# bench/verify_lib.sh —— 任务判据共享库（M0 / D-222）
#
# 设计目标（唯一）：**判据不得恒真**。
#   未做任何修复的 fixture 必须判失败（退出码非 0，且不打印 VERIFY_PASS）。
#
# 用法（每个任务的 verify.sh）：
#   #!/bin/bash
#   set -euo pipefail
#   LIB="$(dirname "$0")/verify_lib.sh"
#   [ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh" >&2; exit 1; }
#   . "$LIB"
#   vh_require_cargo_test
#   vh_assert_src src/lib.rs 'fn multiply'
#   vh_pass T00
#
# 约定：
#   - 只有全部判据通过，才可能执行到 vh_pass（在 set -e 下，任一判据失败即退出）。
#   - VERIFY_PASS 是**末行唯一标记**；runner 判定 = (退出码 0) 且 (含 VERIFY_PASS)。
#   - 失败一律走 stderr 且带 VERIFY_FAIL 前缀，便于人读与机器筛。
# ============================================================================

# vh_fail <msg>：判据失败——落 stderr 并退出非 0。
vh_fail() {
  echo "VERIFY_FAIL: ${1:-未指明原因}" >&2
  exit 1
}

# vh_require_cargo：cargo 必须在场。
# 否则「找不到 cargo」会被静默当成「没什么可测」——这是 fail-open，必须堵死。
vh_require_cargo() {
  if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
  if [ -n "${CARGO_HOME:-}" ] && [ -f "$CARGO_HOME/env" ]; then . "$CARGO_HOME/env"; fi
  command -v cargo >/dev/null 2>&1 || vh_fail "找不到 cargo（PATH 与 ~/.cargo/env 均无）——判据无法执行，拒绝判通过"
}

# vh_require_cargo_test：行为判据——cargo test 真实全绿。
# 同时要求三件事，缺一不可：
#   ① cargo 本身退出码为 0（修复前用管道 `| grep` 吞掉了退出码 ⇒ 失败被判成通过）；
#   ② 输出中不存在 `test result: FAILED`（多测试二进制时，只要有一个失败就必须判失败）；
#   ③ 至少存在一个 `test result: ok`（防「无测试 ⇒ 空跑 ⇒ 通过」）。
vh_require_cargo_test() {
  vh_require_cargo
  local out="" rc=0
  out="$(cargo test 2>&1)" || rc=$?
  printf '%s\n' "$out"
  if [ "$rc" -ne 0 ]; then
    vh_fail "cargo test 退出码非 0（实得 $rc）——修复前此类失败会被判为通过"
  fi
  if printf '%s' "$out" | grep -q 'test result: FAILED'; then
    vh_fail "存在失败的测试二进制（test result: FAILED）"
  fi
  if ! printf '%s' "$out" | grep -q 'test result: ok'; then
    vh_fail "未见任何通过的测试二进制（test result: ok 缺失）——空跑不得判通过"
  fi
  return 0
}

# vh_assert_file <path>：产物存在性。
vh_assert_file() {
  [ -f "${1:?vh_assert_file 需要路径}" ] || vh_fail "缺少文件 ${1}"
  return 0
}

# vh_assert_src <file> <ERE>：源码判据。
# 先剥掉整行注释（首个非空白字符为 `//` 的行）再匹配——防「把关键字写进注释」骗过判据。
vh_assert_src() {
  local file="${1:?vh_assert_src 需要文件}" pat="${2:?vh_assert_src 需要模式}"
  [ -f "$file" ] || vh_fail "缺少文件 $file"
  if ! grep -v '^[[:space:]]*//' "$file" | grep -qE -- "$pat"; then
    vh_fail "$file 中未找到（且非整行注释内的）模式: $pat"
  fi
  return 0
}

# vh_assert_md <file> <ERE>：文本产物判据（ANSWER.md 一类）。
vh_assert_md() {
  vh_assert_src "${1:?vh_assert_md 需要文件}" "${2:?vh_assert_md 需要模式}"
}

# vh_assert_toml <file> <ERE>：TOML 判据。
# TOML 的注释是 `#`（不是 `//`），故剥的是 `#` 开头的整行；**不能**用 vh_assert_src，
# 否则「# serde」这样的注释会满足关键词（Rust 里 `#` 是属性前缀，两个场景不能共用剥法）。
vh_assert_toml() {
  local file="${1:?vh_assert_toml 需要文件}" pat="${2:?vh_assert_toml 需要模式}"
  [ -f "$file" ] || vh_fail "缺少文件 $file"
  if ! grep -v '^[[:space:]]*#' "$file" | grep -qE -- "$pat"; then
    vh_fail "$file 中未找到（且非整行注释内的）模式: $pat"
  fi
  return 0
}

# vh_pass <TID>：全部判据通过后的唯末行标记。
vh_pass() {
  echo "VERIFY_PASS: ${1:?vh_pass 需要任务号}"
}
