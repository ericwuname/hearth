#!/usr/bin/env bash
# 从 ~/.bashrc 提取 APIHUB_AGNES_AI_API_KEY 并生成 config.json（避免 source 非交互崩溃）
set -e
cd "$(dirname "$0")"
val="$(grep '^export APIHUB_AGNES_AI_API_KEY=' ~/.bashrc | head -1 | sed 's/^export APIHUB_AGNES_AI_API_KEY=//; s/^"//; s/"$//')"
if [ -z "$val" ]; then
  echo "未能从 ~/.bashrc 提取到 APIHUB_AGNES_AI_API_KEY" >&2
  exit 1
fi
APIHUB_AGNES_AI_API_KEY="$val" python3 _mkcfg.py
stat -c 'config.json 权限: %a' config.json
