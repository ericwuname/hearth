# P2 执行日志要点（补录）
- Task B 首跑红样本：2 个 QA 轮 1 步 stalled（T4 跨轮签名污染）→ 修复（run 重置 last_graph_sig/graph_stall_count）→ 重跑 11 轮 0 stalled。
- .133 磁盘满事故：codex_t/target 是指向 codex/target 的 symlink；清理 codex/target 76G 后重建为真实目录；磁盘满期间曾出现陈旧测试二进制假失败（9 个）与 target 变文件——全部由磁盘满导致，非代码问题。
- 本地 C: 盘 99% 满 → git stash 触发 gc → .git refs 丢失 + 今日 loose objects 丢失 → 仓库重建（463b315 v0.2.16 baseline），工作树完好。
