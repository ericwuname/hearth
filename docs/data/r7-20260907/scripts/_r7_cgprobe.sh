export PATH="$HOME/.cargo/bin:$PATH"
echo "=== .131 cgroup 环境 ==="
ls -ld /sys/fs/cgroup 2>/dev/null
cat /proc/self/cgroup | head -2
ls ~/.hearth_env 2>/dev/null && grep -i cgroup ~/.hearth_env 2>/dev/null
mount | grep cgroup | head -3
echo "=== 试建 cgroup ==="
mkdir -p /sys/fs/cgroup/codex-test-probe 2>&1 | head -1 && rmdir /sys/fs/cgroup/codex-test-probe 2>/dev/null || echo "mkdir denied"
echo "=== systemd-run 可用性（delegation 通道）==="
which systemd-run
