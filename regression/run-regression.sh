#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# hearth 真机回归集 runner（R-12 / R6 贯穿项：真机回归集与 mock 门禁分列）
#
# 用途：每次发版在真机（.131/.133）跑一遍语料，产出原始日志束——
#       真实成功率（G-W1）从此有标尺。本脚本只负责**执行与取证**，
#       五维打分 / G-A 首句判定按语料 judge 字段人工或评审判读。
#
# 用法：
#   ./run-regression.sh [--arm B|A] [--budget N] [--corpus all|blind|c] [--extra-args "..."]
#
#   --arm B        B 臂（默认，六相基线）
#   --arm A        A 臂（HEARTH_SINGLE_LOOP=1 单循环——R6-9 判别实验 A 组）
#   --budget N     每条预算步数（默认 40）
#   --corpus       all=盲测包15条+C语料（默认）| blind | c
#
# 纪律（语料 BP-2 / 引擎身份）：
#   ① 每条 fresh session（hearth chat 一次性进程，天然满足；禁 resume）；
#   ② 跑前必须 hearth --version + md5 留档（版本不可验证 = 数据作废）；
#   ③ 全部产物落 results/<时间戳>/，人工判读后填 score.csv。
# ─────────────────────────────────────────────────────────────────────────────
set -u

ARM="B"
BUDGET="40"
CORPUS="all"
EXTRA_ARGS=""
FIXTURE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --arm) ARM="$2"; shift 2 ;;
    --budget) BUDGET="$2"; shift 2 ;;
    --corpus) CORPUS="$2"; shift 2 ;;
    --fixture) FIXTURE="$2"; shift 2 ;;
    --extra-args) EXTRA_ARGS="$2"; shift 2 ;;
    *) echo "未知参数: $1"; exit 2 ;;
  esac
done

# R7-1（收官委托书 §三）：C 语料的执行上下文——11 句"TUI 项目进度/纠偏"类
# 判据默认项目上下文存在，空 cwd 必然失真（首轮 C-progress 22 步找不到项目
# 实证）。corpus c 默认铺 TUI sample fixture；每条 fresh 复制（rm -rf 已有）。
# 判分时"环境受限"与"判据违反"分开记账。
if [[ -z "$FIXTURE" && "$CORPUS" == "c" ]]; then
  FIXTURE="$(cd "$(dirname "$0")" && pwd)/fixtures/tui-sample"
fi

HEARTH_BIN="${HEARTH_BIN:-hearth}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="results/${TS}-arm${ARM}"
mkdir -p "$OUT"

echo "── 引擎身份留档 ──"
"$HEARTH_BIN" --version | tee "$OUT/version.txt"
HEARTH_PATH="$(command -v "$HEARTH_BIN")" || { echo "✗ 找不到 $HEARTH_BIN"; exit 2; }
md5sum "$HEARTH_PATH" | tee "$OUT/md5.txt"
if [[ "$ARM" == "A" ]]; then
  echo "HEARTH_SINGLE_LOOP=1 (A 臂单循环)" | tee "$OUT/arm.txt"
else
  echo "B 臂（六相基线）" | tee "$OUT/arm.txt"
fi

CORPUS_FILES=()
case "$CORPUS" in
  blind) CORPUS_FILES=("$(dirname "$0")/corpus/blindpack-15.jsonl") ;;
  c)     CORPUS_FILES=("$(dirname "$0")/corpus/corpus-c.jsonl") ;;
  *)     CORPUS_FILES=("$(dirname "$0")/corpus/blindpack-15.jsonl" "$(dirname "$0")/corpus/corpus-c.jsonl") ;;
esac

PASS=0; FAIL=0; TOTAL=0
summary="$OUT/summary.csv"
echo "id,group,exit_code,steps_hint,artifact_count" > "$summary"

for corpus in "${CORPUS_FILES[@]}"; do
  while IFS= read -r line; do
    [[ -z "$line" || "$line" == \#* ]] && continue
    id="$(printf '%s' "$line" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')"
    grp="$(printf '%s' "$line" | sed -n 's/.*"group":"\([^"]*\)".*/\1/p')"
    input="$(printf '%s' "$line" | sed -n 's/.*"input":"\(.*\)","judge".*/\1/p')"
    # JSONL 里 \n 还原为真实换行（B4 多行粘贴场景）
    input="$(printf '%b' "${input//\\n/\n}")"
    [[ -z "$input" ]] && { echo "✗ 解析失败: $id"; continue; }
    TOTAL=$((TOTAL+1))
    echo "── [$id] 开始（arm=$ARM budget=$BUDGET）──"
    # 隔离工作区（R-1 仪器教训：agent cwd 与结果目录同区 = 驱动诱导污染——
    # 实测 .133 首轮 A4 agent 自行 ls 结果目录并改写 summary.csv）。每条
    # 在独立 scratch cwd 里跑，agent 摸不到本脚本的产物区。
    # R7-1 补强：WORKDIR 挂在**不可枚举**的随机父目录下——首轮干净对照
    # A9-A 实证兄弟条目残骸（B5 的 spill/报告）经共享父目录泄漏给后续
    # 条目（跨条目污染通道）。随机父目录使 ls 枚举失效。
    WORKDIR="/tmp/.hr-$$-${RANDOM}/${id}"
    rm -rf "$WORKDIR"; mkdir -p "$WORKDIR"
    # fixture 铺设（C 语料：每条 fresh 项目上下文）
    if [[ -n "$FIXTURE" && -d "$FIXTURE" ]]; then
      cp -r "$FIXTURE"/. "$WORKDIR"/
    fi
    log="$OUT/${id}.log"
    start=$(date +%s)
    if [[ "$ARM" == "A" ]]; then
      ( cd "$WORKDIR" && HEARTH_SINGLE_LOOP=1 "$HEARTH_BIN" chat "$input" --budget "$BUDGET" $EXTRA_ARGS ) >"$log" 2>&1
    else
      ( cd "$WORKDIR" && "$HEARTH_BIN" chat "$input" --budget "$BUDGET" $EXTRA_ARGS ) >"$log" 2>&1
    fi
    rc=$?
    end=$(date +%s)
    echo "$((end-start))s" > "$OUT/${id}.elapsed"
    # 终态行取证（✓/✗/paused 投影行——判读锚点）
    grep -E "✓|✗|⏹|completed|failed|paused" "$log" | tail -5 > "$OUT/${id}.terminal" || true
    if [[ $rc -eq 0 ]]; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); fi
    echo "$id,$grp,$rc,,$(( $(wc -c < "$log") / 1024 ))KB" >> "$summary"
  done < "$corpus"
done

echo "── 完成：$TOTAL 条（exit0=$PASS，非零=$FAIL）──"
echo "原始日志束：$OUT/  —— 五维打分/G-A 判读：按语料 judge 字段，结果填 $OUT/score.csv"
echo "score 模板：id,①符合预期,②无裸ERROR,③不用猜,④不误路由,⑤不假完成,备注"
