#!/usr/bin/env bash
# Hearth 一键启动脚本（在你的 Linux 机器上用）
# 用法：
#   ./run-hearth.sh           # 后台起 service，然后给你 CLI 用法提示
#   ./run-hearth.sh stop      # 停掉后台 service
#
# 前置：
#   1. cp .env.example .env   并填入 DEEPSEEK_API_KEY
#   2. cargo 已装（rustup 默认）
#
set -euo pipefail

# 必须在仓库根目录运行
cd "$(dirname "$0")"

PIDFILE=.hearth-service.pid

if [[ "${1:-}" == "stop" ]]; then
  if [[ -f "$PIDFILE" ]]; then
    PID=$(cat "$PIDFILE")
    echo "▶ 停止 Hearth service (pid $PID)…"
    kill "$PID" 2>/dev/null || true
    rm -f "$PIDFILE"
    echo "✓ 已停止"
  else
    echo "（没有运行中的 service）"
  fi
  exit 0
fi

if [[ -f "$PIDFILE" ]] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
  echo "⚠ Hearth service 已在运行 (pid $(cat "$PIDFILE"))"
else
  echo "▶ 编译并后台启动 Hearth service（首次编译较慢，约 1-3 分钟）…"
  # 后台编译+跑 service，日志落 hearth-service.log
  nohup cargo run -p service --quiet > hearth-service.log 2>&1 &
  echo $! > "$PIDFILE"
  echo "  → 日志：tail -f hearth-service.log"
  echo "  → 等待 service 起来（healthz）…"
  for i in $(seq 1 60); do
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/healthz 2>/dev/null | grep -q 200; then
      echo "✓ service 已就绪 (http://localhost:3000)"
      break
    fi
    sleep 2
  done
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Hearth CLI 用法（新开一个终端）："
echo ""
echo "  # 一次性任务（创建会话→跑→退出）"
echo "  cargo run -p codex-cli -- chat \"<你的目标>\""
echo ""
echo "  # 交互式多轮（有审批时按 y/N）"
echo "  cargo run -p codex-cli -- repl"
echo ""
echo "  # 会话管理"
echo "  cargo run -p codex-cli -- sessions      # 列出"
echo "  cargo run -p codex-cli -- resume <id>   # 续跑中断的"
echo "  cargo run -p codex-cli -- history <id>  # 看历史"
echo "  cargo run -p codex-cli -- cancel <id>   # 取消"
echo ""
echo "  # 若 service 不在 localhost:3000，用 --url 覆盖"
echo "  cargo run -p codex-cli -- --url http://<host>:3000 chat \"…\""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
