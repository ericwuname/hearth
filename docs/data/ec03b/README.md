# 项目目录说明

本目录是一个 AI 代理（Agnes-2.5-Flash）的实验/测试工作区，记录了多轮任务执行的过程与结果。

## 文件结构

```
├── calc.html                  # 简单计算器 Web 应用（HTML/CSS/JS）
├── cache_b.jsonl              # AI 模型请求缓存日志（JSONL 格式）
├── T1_ambiguous_B.log         # 任务1（歧义任务）执行日志 B
├── T1_ambiguous_C.log         # 任务1（歧义任务）执行日志 C
├── T2_multistep_B.log         # 任务2（多步任务）执行日志 B
├── T2_multistep_C.log         # 任务2（多步任务）执行日志 C
├── T3_implicit_B.log          # 任务3（隐式任务）执行日志 B
├── .hearth/                   # Hearth 运行时配置目录
└── results/
    └── transcripts/           # 任务执行结果转录文件
        ├── 61228e4b-*.jsonl   # 会话转录（任务1-歧义-B）
        ├── 682420f4-*.jsonl   # 会话转录（任务1-歧义-C）
        ├── 71ecdc15-*.jsonl   # 会话转录（任务2-多步-B）
        ├── f199ce75-*.jsonl   # 会话转录（任务2-多步-C）
        └── 797872f8-*.jsonl   # 会话转录（任务3-隐式-B）
```

## 文件说明

### calc.html
一个轻量级单页计算器，支持加、减、乘、除四则运算，采用现代 CSS Grid 布局，可直接在浏览器中打开使用。

### cache_b.jsonl
包含 120 条 AI 模型调用记录，每条记录为 JSON 格式，包含：
- `session_id`: 会话标识
- `model`: 模型名称（agnes-2.5-flash）
- `provider`: 提供商（agnes）
- `prompt_tokens` / `completion_tokens` / `total_tokens`: Token 用量
- `ts`: 时间戳
- `exit`: 退出方式（cli）

### T*_*.log
任务执行日志文件，记录 AI 代理的推理过程，包括：
- **T1_ambiguous_*.log**: 歧义任务（Ambiguous Task）的执行轨迹
- **T2_multistep_*.log**: 多步任务（Multistep Task）的执行轨迹
- **T3_implicit_*.log**: 隐式任务（Implicit Task）的执行轨迹

日志采用结构化格式，展示 Observe → Reflect → Plan → Act 的循环过程。

### results/transcripts/*.jsonl
每个 JSONL 文件对应一次完整的任务执行会话，记录：
- `goal`: 任务目标
- `status`: 执行状态（ok/failed）
- `steps`: 执行步数
- `wall_secs`: 实际耗时（秒）
- `tool_calls`: 使用的工具列表
- `written_files`: 写入的文件列表

## 运行环境
- 模型：Agnes-2.5-Flash（Sapiens AI 开发）
- 运行时：Hearth（AI 代理框架）
- 日期：2026-08-28 ~ 2026-08-29
