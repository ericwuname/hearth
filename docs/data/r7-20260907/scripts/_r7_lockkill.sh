fuser -v ~/.cargo/.package-cache 2>&1 | head -4
for pid in $(fuser ~/.cargo/.package-cache 2>/dev/null); do
  echo "killing $pid: $(ps -o pid,ppid,cmd -p $pid | tail -1)"
  kill -9 $pid
done
sleep 1
fuser -v ~/.cargo/.package-cache 2>&1 | head -4 || echo "lock free"
