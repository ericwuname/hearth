for pid in 46834 47435; do
  echo "=== $pid ==="
  cat /proc/$pid/wchan 2>/dev/null; echo
  cat /proc/$pid/syscall 2>/dev/null
  cat /proc/$pid/status 2>/dev/null | grep -E "State|Threads"
done
