echo "=== .133 cgroup 环境 ==="
ls -ld /sys/fs/cgroup
cat /proc/self/cgroup | head -2
ls -ld /sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service 2>/dev/null || echo "no user@1000.service dir"
ls /sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/ 2>/dev/null | head -5
echo "=== .hearth_env ==="
grep -i -E "cgroup|CGROUP" ~/.hearth_env 2>/dev/null || echo "no cgroup in .hearth_env"
echo "=== codex-sandbox cgroup 目录 ==="
ls -d /sys/fs/cgroup/codex-sandbox-* 2>/dev/null | head -3
find /sys/fs/cgroup -maxdepth 4 -name "codex-sandbox-*" 2>/dev/null | head -3
echo "=== 试建探针 ==="
mkdir -p /sys/fs/cgroup/codex-test-probe 2>&1 | head -1 && rmdir /sys/fs/cgroup/codex-test-probe 2>/dev/null && echo "mkdir OK at root" || echo "root mkdir denied"
