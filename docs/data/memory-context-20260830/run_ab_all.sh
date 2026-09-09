#!/bin/bash
# P2 Node 05/06: A/B interleaved orchestrator (A,B,A,B,... x5 pairs)
for pair in 1 2 3 4 5; do
  bash /home/wutao/fa/run_ab.sh A $pair > /home/wutao/fa/ab_A${pair}.log 2>&1
  bash /home/wutao/fa/run_ab.sh B $pair > /home/wutao/fa/ab_B${pair}.log 2>&1
done
echo ALL_DONE > /home/wutao/fa/ab_all_done.flag
