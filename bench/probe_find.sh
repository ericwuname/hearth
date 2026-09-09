#!/bin/bash
# RT3 find probe（上传到 VM 执行）
SID=$(curl -s -X POST http://127.0.0.1:3000/api/v1/sessions \
  -H 'Content-Type: application/json' \
  -d '{"provider":"replay","goal":"probe","budget":{"max_steps":1}}' \
  | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('session_id',''))")
echo "SID=$SID"
if [ -z "$SID" ]; then
  echo "SESSION_FAIL"
  exit 1
fi
curl -s -X POST "http://127.0.0.1:3000/api/v1/sessions/$SID/messages" \
  -H 'Content-Type: application/json' \
  -d '{"content":"run bash: find . -name \"*.nonexistent\" 2>&1; echo FIND_RC $?"}' > /dev/null
sleep 8
curl -s "http://127.0.0.1:3000/api/v1/sessions/$SID/events" | grep -o '"tool_result":{[^}]*}' | tail -1
echo DONE
