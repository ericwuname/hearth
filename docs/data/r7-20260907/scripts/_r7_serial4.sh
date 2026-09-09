export PATH="$HOME/.cargo/bin:$PATH"
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
cd ~/codex-r6
cargo test -p sandbox --lib -- --test-threads=1 > /home/wutao/sbtest.log 2>&1
echo "rc=$?"
grep -E "^test result|FAILED|panicked at" /home/wutao/sbtest.log | head -10
./target/release/hearth --version
md5sum ./target/release/hearth
