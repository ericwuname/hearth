export PATH="$HOME/.cargo/bin:$PATH"
cd /tmp && rm -rf b5-verify && mkdir b5-verify && cd b5-verify
echo "=== B5 场景复跑（修复后）==="
START=$(date +%s)
timeout 600 /home/wutao/codex-r6/target/release/hearth chat "2026-09-02T10:00:00.000000Z ERROR sandbox::seccomp: syscall denied op=clone3 pid=12345 comm=cargo — fail-closed（故意编的错误日志，直接粘贴）" --budget 25 --approve-within session > /home/wutao/b5-verify.log 2>&1
RC=$?
END=$(date +%s)
echo "exit=$RC 用时=$((END-START))s"
echo "=== hearth 进程应已退出（0=通过）==="
ps aux | grep "[h]earth chat" | wc -l
echo "=== defunct 检查（0=通过）==="
ps aux | grep "[d]efunct" | wc -l
echo "=== 输出尾部 ==="
tail -6 /home/wutao/b5-verify.log
