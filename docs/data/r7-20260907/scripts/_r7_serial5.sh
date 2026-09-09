export PATH="$HOME/.cargo/bin:$PATH"
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
cd ~/codex-r6
nohup bash -c 'cargo test -p sandbox --lib -- --test-threads=1 > /home/wutao/sbtest.log 2>&1; echo "rc=$?" >> /home/wutao/sbtest.log' >/dev/null 2>&1 &
echo "test started"
