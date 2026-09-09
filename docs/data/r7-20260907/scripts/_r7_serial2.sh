export PATH="$HOME/.cargo/bin:$PATH"
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
cd ~/codex-r6
timeout 180 cargo test -p sandbox --lib -- --test-threads=1 2>&1 | grep -E "^test |test result|panicked" | head -40
