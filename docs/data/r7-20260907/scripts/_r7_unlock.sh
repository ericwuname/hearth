export PATH="$HOME/.cargo/bin:$PATH"
kill -9 46834 47435 2>/dev/null
sleep 1
rm -f /home/wutao/.cargo/.package-cache
ls -la /home/wutao/.cargo/.package-cache 2>/dev/null || echo "lock file removed (new inode on next create)"
cd ~/codex-r6
nohup bash -c 'export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth; cargo test -p sandbox --lib -- --test-threads=1 > /home/wutao/sbtest.log 2>&1; cargo build --release -p codex-cli >> /home/wutao/sbtest.log 2>&1; echo "rc=$?" >> /home/wutao/sbtest.log' >/dev/null 2>&1 &
echo "serial pipeline started"
