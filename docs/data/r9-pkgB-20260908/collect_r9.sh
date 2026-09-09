#!/usr/bin/env bash
# R9 包B 采集器：四组结果 → 对照表 CSV + Q1-Q5 待判表 + 判读包
# 只做机械采集，不判分（判分外包砺/外部窗——跑测窗不 self-judge）
set -u
ROOT="/home/wutao/r9-pkgB"
OUT="$ROOT/results"
CORPUS="/home/wutao/codex-r6/regression/corpus/blindpack-15.jsonl"

# ── 1) 对照表：group,id,rc,elapsed_s,calls_hint ──
csv="$OUT/comparison-table.csv"
echo "group,id,rc,elapsed_s,calls_hint" > "$csv"
for g in G1 G2 G3 G4; do
  while IFS= read -r line; do
    id=$(printf '%s' "$line" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
    [[ -z "$id" ]] && continue
    rc=$(cat "$OUT/$g/$id.rc" 2>/dev/null || echo "missing")
    el=$(awk '{print $NF}' "$OUT/$g/$id.elapsed" 2>/dev/null | tr -d 's' || echo "missing")
    calls=""
    case "$g" in
      G1) calls=1 ;;
      G2) calls=$(grep -aoE 'tokens used: [0-9,]+' "$OUT/$g/$id.log" 2>/dev/null | tail -1) ;;
      G3) calls=$(grep -aoE '[0-9]+ ([Aa]PI|[Tt]oken)' "$OUT/$g/$id.log" 2>/dev/null | tail -1) ;;
      G4) calls=$(grep -aoE 'calls=[0-9]+' "$OUT/$g/$id.log" 2>/dev/null | tail -1) ;;
    esac
    echo "$g,$id,$rc,$el,${calls:-}" >> "$csv"
  done < "$CORPUS"
done
echo "对照表: $csv"

# ── 2) Q1-Q5 待判表（全空——判分外包）──
q="$OUT/q1q5-pending.csv"
echo "group,id,rc,Q1直接回答,Q2说到做到,Q3做不到要交代,Q4信息完整,Q5可复现(3次波动≤10pp),备注" > "$q"
for g in G1 G2 G3 G4; do
  while IFS= read -r line; do
    id=$(printf '%s' "$line" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
    [[ -z "$id" ]] && continue
    rc=$(cat "$OUT/$g/$id.rc" 2>/dev/null || echo "missing")
    echo "$g,$id,$rc,,,,,," >> "$q"
  done < "$CORPUS"
done
echo "Q1-Q5待判表: $q"

# ── 3) 判读包（每条：身份行 + terminal + judge 字段提示 + 日志路径）──
pack="$OUT/adjudication-pack.txt"
: > "$pack"
for g in G1 G2 G3 G4; do
  {
    echo "════════ $g ════════"
    head -1 "$OUT/$g/identity.txt" 2>/dev/null
  } >> "$pack"
  while IFS= read -r line; do
    id=$(printf '%s' "$line" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
    [[ -z "$id" ]] && continue
    judge=$(printf '%s' "$line" | sed -n 's/.*"judge":"\(.*\)","source".*/\1/p')
    {
      echo "──── [$g/$id] rc=$(cat "$OUT/$g/$id.rc" 2>/dev/null) elapsed=$(cat "$OUT/$g/$id.elapsed" 2>/dev/null)"
      echo "  judge提示: $judge"
      echo "  terminal锚点:"
      sed 's/^/    /' "$OUT/$g/$id.terminal" 2>/dev/null
      echo "  完整日志: results/$g/$id.log"
    } >> "$pack"
  done < "$CORPUS"
done
echo "判读包: $pack"

# ── 4) 汇总统计（rc 口径）──
stat="$OUT/rc-summary.txt"
: > "$stat"
for g in G1 G2 G3 G4; do
  p=$(grep -c ",0," "$csv" 2>/dev/null || true)
  n=$(awk -F, -v g="$g" '$1==g && $3==0' "$csv" | wc -l)
  t=$(awk -F, -v g="$g" '$1==g' "$csv" | wc -l)
  med=$(awk -F, -v g="$g" '$1==g && $4 ~ /^[0-9]+$/ {print $4}' "$csv" | sort -n | awk '{a[NR]=$1} END {if(NR>0) print (NR%2)?a[(NR+1)/2]:int((a[NR/2]+a[NR/2+1])/2); else print "NA"}')
  echo "$g: rc0=$n/$t elapsed中位=${med}s" >> "$stat"
done
echo "rc汇总: $stat"; cat "$stat"
