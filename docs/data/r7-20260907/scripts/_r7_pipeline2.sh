export PATH="$HOME/.cargo/bin:$PATH"
cd /home/wutao/codex-r6
cd /home/wutao/regress-run
export HEARTH_BIN=/home/wutao/codex-r6/target/release/hearth
RUNNER=/home/wutao/codex-r6/regression/run-regression.sh
chmod +x $RUNNER 2>/dev/null
nohup bash -c "
  bash $RUNNER --arm B > /home/wutao/regress-run/r7-armB-blind.log 2>&1
  bash $RUNNER --arm A > /home/wutao/regress-run/r7-armA-blind.log 2>&1
  bash $RUNNER --arm B --corpus c > /home/wutao/regress-run/r7-armB-corpusc.log 2>&1
  bash $RUNNER --arm A --corpus c > /home/wutao/regress-run/r7-armA-corpusc.log 2>&1
  echo ALL_DONE > /home/wutao/regress-run/r7-pipeline.done
" > /dev/null 2>&1 &
echo "R7-1 pipeline restarted pid=$!"
sleep 5
tail -2 /home/wutao/regress-run/r7-armB-blind.log
