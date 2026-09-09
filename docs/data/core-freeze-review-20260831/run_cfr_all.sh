#!/bin/bash
# CFR orchestrator: Node 06 probe → 08(A+B) → 09(A+B) → 10(A+B) → 11
set -u
for d in cfr_n06 cfr_n08 cfr_n09 cfr_n09b cfr_n10a cfr_n10b cfr_n11; do
  rm -rf /tmp/$d
  mkdir -p /tmp/$d
done
bash /home/wutao/fa/run_cfr_n06.sh  > /home/wutao/fa/cfr_n06.log  2>&1
bash /home/wutao/fa/run_cfr_n08a.sh > /home/wutao/fa/cfr_n08a.log 2>&1
bash /home/wutao/fa/run_cfr_n08b.sh > /home/wutao/fa/cfr_n08b.log 2>&1
bash /home/wutao/fa/run_cfr_n09.sh  > /home/wutao/fa/cfr_n09.log  2>&1
bash /home/wutao/fa/run_cfr_n10.sh  > /home/wutao/fa/cfr_n10.log  2>&1
bash /home/wutao/fa/run_cfr_n11.sh  > /home/wutao/fa/cfr_n11.log  2>&1
echo ALL_DONE > /home/wutao/fa/cfr_all_done.flag
