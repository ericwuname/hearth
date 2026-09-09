grep -E "FLOCK|OFDLCK" /proc/locks | tail -10
echo "=== .package-cache 的 inode ==="
stat -c "%i %n" /home/wutao/.cargo/.package-cache
echo "=== lsof 全量 ==="
lsof /home/wutao/.cargo/.package-cache 2>/dev/null
