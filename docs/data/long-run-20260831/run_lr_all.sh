#!/bin/bash
# P2-LR orchestrator: Node 06 → 08(A+B) → 09 → 10 → 11 → 12(×2) → 13(A+B)
set -u
cd /home/wutao
for d in lr_n06 lr_n08 lr_n09 lr_n10 lr_n11 lr_n12_r1 lr_n12_r2 lr_n13; do
  rm -rf /tmp/$d
  mkdir -p /tmp/$d
done

bash /home/wutao/fa/run_n06.sh  > /home/wutao/fa/lr_n06.log  2>&1
bash /home/wutao/fa/run_n08a.sh > /home/wutao/fa/lr_n08a.log 2>&1
bash /home/wutao/fa/run_n08b.sh > /home/wutao/fa/lr_n08b.log 2>&1
bash /home/wutao/fa/run_n09.sh  > /home/wutao/fa/lr_n09.log  2>&1
bash /home/wutao/fa/run_n10.sh  > /home/wutao/fa/lr_n10.log  2>&1
bash /home/wutao/fa/run_n11.sh  > /home/wutao/fa/lr_n11.log  2>&1
bash /home/wutao/fa/run_n12.sh 1 > /home/wutao/fa/lr_n12r1.log 2>&1
bash /home/wutao/fa/run_n12.sh 2 > /home/wutao/fa/lr_n12r2.log 2>&1
bash /home/wutao/fa/run_n13a.sh > /home/wutao/fa/lr_n13a.log 2>&1
bash /home/wutao/fa/run_n13b.sh > /home/wutao/fa/lr_n13b.log 2>&1
echo ALL_DONE > /home/wutao/fa/lr_all_done.flag
