echo "=== hearth cgroup 目录属主 ==="
ls -ld /sys/fs/cgroup/hearth 2>/dev/null || echo "no /sys/fs/cgroup/hearth"
echo "=== sudo 可用性 ==="
echo '123456' | sudo -S true 2>&1 | tail -1 && echo "sudo OK"
