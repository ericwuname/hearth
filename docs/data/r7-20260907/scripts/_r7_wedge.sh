find /home/wutao/codex-r6/target -type p -o -type s 2>/dev/null | head -5
echo "=== 清 debug 目录 ==="
rm -rf /home/wutao/codex-r6/target/debug
echo cleaned
