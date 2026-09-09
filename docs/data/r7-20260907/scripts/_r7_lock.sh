ps aux | grep -E "[c]argo|[r]ustc" | head -8
echo "=== package-cache 锁持有者 ==="
ls -la ~/.cargo/.package-cache 2>/dev/null
fuser -v ~/.cargo/.package-cache 2>&1 | head -5 || true
lsof ~/.cargo/.package-cache 2>/dev/null | head -5 || true
echo "=== build-dir 锁 ==="
fuser -v /home/wutao/codex-r6/target/.cargo-lock 2>&1 | head -5 || true
lsof /home/wutao/codex-r6/target/.cargo-lock 2>/dev/null | head -5 || true
