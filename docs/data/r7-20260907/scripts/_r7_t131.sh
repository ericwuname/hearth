export PATH="$HOME/.cargo/bin:$PATH"
echo '123456' | sudo -S mkdir -p /sys/fs/cgroup/hearth 2>/dev/null
echo '123456' | sudo -S chown wutao:wutao /sys/fs/cgroup/hearth 2>/dev/null
ls -ld /sys/fs/cgroup/hearth
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
cd ~/codex-r6
echo "=== sandbox 全量（含 R7-2 新测试）==="
cargo test -p sandbox --lib 2>&1 | tail -3
echo "=== R7-2 专项 ==="
cargo test -p sandbox --lib grandchild 2>&1 | grep -E "^test |test result" | head -3
