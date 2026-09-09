export PATH="$HOME/.cargo/bin:$PATH"
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
cd ~/codex-r6
git log --oneline -1
ls target/release/hearth && ./target/release/hearth --version && md5sum ./target/release/hearth
echo "=== sandbox 全量 ==="
cargo test -p sandbox --lib 2>&1 | tail -3
echo "=== R7-2 专项 ==="
cargo test -p sandbox --lib grandchild 2>&1 | grep -E "^test |test result" | head -3
