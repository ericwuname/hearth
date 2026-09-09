export PATH="$HOME/.cargo/bin:$PATH"
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
cd ~/codex-r6
nohup bash -c 'timeout 300 cargo test -p sandbox --lib -- --test-threads=1 > /home/wutao/sbtest.log 2>&1; echo "rc=$?" >> /home/wutao/sbtest.log' >/dev/null 2>&1 &
echo started
sleep 115
tail -6 /home/wutao/sbtest.log 2>/dev/null
grep -cE "^test " /home/wutao/sbtest.log 2>/dev/null
