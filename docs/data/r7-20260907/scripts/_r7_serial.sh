export PATH="$HOME/.cargo/bin:$PATH"
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
cd ~/codex-r6
kill -9 45600 2>/dev/null
sleep 1
timeout 120 cargo test -p sandbox --lib --test-threads=1 -- --test-threads=1 2>&1 | tail -30
