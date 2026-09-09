ls -la /home/wutao/codex-r6/target/debug/.cargo-lock 2>/dev/null
fuser -v /home/wutao/codex-r6/target/debug/.cargo-lock 2>&1 | head -5
echo "=== 全部 cargo/rustc/live 进程 ==="
ps aux | grep -E "[c]argo|[r]ustc" | grep -v defunct
echo "=== hearth 进程 ==="
ps aux | grep "[h]earth" | grep -v grep | head -4
