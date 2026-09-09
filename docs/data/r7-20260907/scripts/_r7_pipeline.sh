cd /home/wutao/regress-run
export HEARTH_BIN=/home/wutao/codex-r6/target/release/hearth
RUNNER=/home/wutao/codex-r6/regression/run-regression.sh
nohup bash -c "
  $RUNNER --arm B > /home/wutao/regress-run/r7-armB-blind.log 2>&1
  $RUNNER --arm A > /home/wutao/regress-run/r7-armA-blind.log 2>&1
  $RUNNER --arm B --corpus c > /home/wutao/regress-run/r7-armB-corpusc.log 2>&1
  $RUNNER --arm A --corpus c > /home/wutao/regress-run/r7-armA-corpusc.log 2>&1
  echo ALL_DONE > /home/wutao/regress-run/r7-pipeline.done
" > /dev/null 2>&1 &
echo "R7-1 pipeline started pid=$!"
