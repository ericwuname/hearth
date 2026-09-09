pkill -f "run-regression" 2>/dev/null
pkill -f "hearth chat" 2>/dev/null
sleep 2
cd /home/wutao/regress-run
export HEARTH_BIN=/home/wutao/codex-r6/target/release/hearth
RUNNER=/home/wutao/codex-r6/regression/run-regression.sh
nohup bash -c "
  bash $RUNNER --arm B --extra-args '--approve-within session' > /home/wutao/regress-run/r7-armB-blind.log 2>&1
  bash $RUNNER --arm A --extra-args '--approve-within session' > /home/wutao/regress-run/r7-armA-blind.log 2>&1
  bash $RUNNER --arm B --corpus c --extra-args '--approve-within session' > /home/wutao/regress-run/r7-armB-corpusc.log 2>&1
  bash $RUNNER --arm A --corpus c --extra-args '--approve-within session' > /home/wutao/regress-run/r7-armA-corpusc.log 2>&1
  echo ALL_DONE > /home/wutao/regress-run/r7-pipeline2.done
" > /dev/null 2>&1 &
echo "pipeline v2 (with approve-within) pid=$!"
sleep 6
tail -1 /home/wutao/regress-run/r7-armB-blind.log
