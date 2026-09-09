#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# R9 包B · 四组底座对照 runner（.131 专用；一次性脚本，不落仓库根目录）
#
# 四组同尺：15 条 blindpack 逐字、fresh session、cwd 随机父目录隔离、
#           timeout 420s/条、flock 串行（禁同机并发 LLM）、同 key 同模型。
#
# 用法：r9pkgb.sh <G1|G2|G3|G4> [--only <id>]   （--only 仅供冒烟）
#
# 依据：R9包B开工指令 §二协议 + regression/run-regression.sh 口径（R8 门禁）
# ─────────────────────────────────────────────────────────────────────────────
set -u

GROUP="${1:?用法: r9pkgb.sh <G1|G2|G3|G4> [--only id]}"
ONLY=""
[[ "${2:-}" == "--only" ]] && ONLY="${3:-}"

KEY="cpk-f4UBH3NaHUUN2SAcW1LhZT0kyuqlu9TF5lTItbuXHSTP3w0c"
MODEL="agnes-2.5-flash"
BASE="https://api.agnes-ai.cn/v1"
HOST="https://api.agnes-ai.cn"
CORPUS="/home/wutao/codex-r6/regression/corpus/blindpack-15.jsonl"
TIMEOUT_S=420
GOOSE_BIN="/home/wutao/.local/bin/goose"
HEARTH_BIN="/usr/local/bin/hearth"
ROOT="/home/wutao/r9-pkgB"
OUT="$ROOT/results/$GROUP"
mkdir -p "$OUT"

# ── 底座版本留档（引擎身份纪律：对底座同款适用）──
case "$GROUP" in
  G1) VER="raw-agnes-api(curl,无框架无工具)";;
  G2) VER="codex-cli $(codex --version 2>&1 | head -1)";;
  G3) VER="goose $("$GOOSE_BIN" --version 2>&1 | head -1)";;
  G4) VER="$("$HEARTH_BIN" --version 2>/dev/null | grep -a '^hearth' | head -1) HEARTH_SINGLE_LOOP=1(A臂) md5=$(md5sum "$HEARTH_BIN" | cut -c1-8)";;
  *) echo "✗ 未知组 $GROUP"; exit 2;;
esac
TS0="$(date -u +%Y%m%dT%H%M%SZ)"
echo "group=$GROUP | base=$VER | model=$MODEL | started_utc=$TS0" | tee "$OUT/identity.txt"
if [[ "$GROUP" == "G4" ]]; then
  md5sum "$HEARTH_BIN" >> "$OUT/identity.txt"
  "$HEARTH_BIN" --version >> "$OUT/identity.txt"
fi

# ── 逐条执行 ──
while IFS= read -r line; do
  [[ -z "$line" || "$line" == \#* ]] && continue
  id="$(printf '%s' "$line" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')"
  grp="$(printf '%s' "$line" | sed -n 's/.*"group":"\([^"]*\)".*/\1/p')"
  input="$(printf '%s' "$line" | sed -n 's/.*"input":"\(.*\)","judge".*/\1/p')"
  input="$(printf '%b' "${input//\\n/\n}")"
  [[ -z "$input" ]] && { echo "✗ 解析失败: $id"; continue; }
  if [[ -n "$ONLY" && "$id" != "$ONLY" ]]; then continue; fi

  # cwd 隔离：随机父目录防兄弟条目残骸污染（R7-1 教训）
  WORKDIR="/tmp/.hr-$$-${RANDOM}/$GROUP/$id"
  rm -rf "$WORKDIR"; mkdir -p "$WORKDIR"
  log="$OUT/$id.log"; body="$OUT/$id.body"
  start=$(date +%s)

  case "$GROUP" in
    G1)  # 裸跑基线：agnes chat/completions 直连，无系统提示词、无工具
      payload="$(jq -n --arg m "$MODEL" --arg c "$input" '{model:$m,messages:[{role:"user",content:$c}]}')"
      ( cd "$WORKDIR" && timeout "$TIMEOUT_S" curl -sS --max-time "$TIMEOUT_S" \
          -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" \
          -d "$payload" "$BASE/chat/completions" ) < /dev/null > "$body" 2>&1
      rc=$?
      ;;
    G2)  # Codex CLI 0.44.0（版本切换见报告：0.153.4 responses wire 与 agnes 网关工具回传
         # 不兼容 13/15 挂，0.153.4 数据留档 G2-v0153-invalid；0.44 chat wire 走通但
         # 模型 tool_call arguments 间歇非法——协议层 400 条目自动复跑一次并标 RERUN）；
         # --full-auto=0.44 官方非交互批准（workspace-write）；< /dev/null 防 stdin 吞语料
      run_g2() {
        ( cd "$WORKDIR" && OPENAI_API_KEY="$KEY" timeout "$TIMEOUT_S" codex exec \
            -c 'model_providers.agnesdirect.name="agnes-direct"' \
            -c 'model_providers.agnesdirect.base_url="'"$BASE"'"' \
            -c 'model_providers.agnesdirect.wire_api="chat"' \
            -c 'model_providers.agnesdirect.env_key="OPENAI_API_KEY"' \
            -c 'model_provider="agnesdirect"' \
            --full-auto --skip-git-repo-check -m "$MODEL" --color never "$input" ) < /dev/null
      }
      run_g2 > "$body" 2>&1
      rc=$?
      if [[ $rc -ne 0 ]] && grep -qaE 'must be valid JSON|stream error|ResponseInput' "$body"; then
        echo "[G2/$id] 协议层失败，复跑一次（RERUN 标注）" >&2
        run_g2 > "$body" 2>&1
        rc=$?
        echo "RERUN=1" > "$OUT/$id.rerun"
      fi
      ;;
    G3)  # Goose 原装默认；openai provider 经 OPENAI_BASE_URL 接入 agnes
         # （goose 1.49 openai_def.rs 实证读 OPENAI_BASE_URL/OPENAI_HOST，不认 OPENAI_API_HOST）
      ( cd "$WORKDIR" && OPENAI_API_KEY="$KEY" OPENAI_BASE_URL="$BASE" \
          GOOSE_PROVIDER=openai GOOSE_MODEL="$MODEL" \
          timeout "$TIMEOUT_S" "$GOOSE_BIN" run -t "$input" ) < /dev/null > "$body" 2>&1
      rc=$?
      ;;
    G4)  # Hearth A 臂（HEARTH_SINGLE_LOOP=1）+ HEARTH_BIN 显式路径铁律
      ( cd "$WORKDIR" && HEARTH_SINGLE_LOOP=1 timeout "$TIMEOUT_S" \
          "$HEARTH_BIN" chat "$input" --budget 40 --approve-within session ) < /dev/null > "$body" 2>&1
      rc=$?
      ;;
  esac
  end=$(date +%s)

  # 日志首行 = 组名+底座版本+模型+时间戳（身份块），正文另存 .body 判读
  { echo "[R9包B | $GROUP | $VER | $MODEL | $(date -u +%Y%m%dT%H%M%SZ) | id=$id]";
    cat "$body"; } > "$log"; rm -f "$body"
  echo "$((end-start))s" > "$OUT/$id.elapsed"
  echo "$rc" > "$OUT/$id.rc"

  # 终态锚点（判读素材；G4 沿用 R8 口径，其余组按各自终态特征）
  case "$GROUP" in
    G1) grep -o '"finish_reason"[^,}]*' "$log" | head -2 > "$OUT/$id.terminal" 2>/dev/null || true;;
    G2) grep -aE "error|Error|ERROR|tokens used" "$log" | tail -3 > "$OUT/$id.terminal" 2>/dev/null || true;;
    G3) grep -aE "error|Error|ERROR|Closing session|Exiting" "$log" | tail -3 > "$OUT/$id.terminal" 2>/dev/null || true;;
    G4) grep -aE "✓|✗|⏹|completed|failed|paused" "$log" | tail -5 > "$OUT/$id.terminal" 2>/dev/null || true;;
  esac
  echo "── [$GROUP/$id] rc=$rc $((end-start))s ──"
done < "$CORPUS"

echo "[$GROUP] 完成。产物：$OUT/"
