#!/bin/bash
# CFR Node 10: Controlled Failure Benchmark ×2 拓扑
# Case A: test failure → repair → retest → pass（configlib 常量错误）
cd /tmp/cfr_n10a
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=480
hearth chat '创建一个 Rust 库项目 configlib/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p configlib/src
2) 用 write_file 创建 configlib/Cargo.toml，内容为 [package] name=configlib version=0.1.0 edition=2021
3) 用 write_file 创建 configlib/src/lib.rs，实现：
   pub const MAX_RETRIES: u32 = 5;（故意留 bug：测试期望 3）
   pub fn validate_timeout(t: u64) -> Result<u64, String> { if t == 0 { Err("timeout must be > 0".into()) } else if t > 3600 { Err("timeout too large".into()) } else { Ok(t) } }
   测试模块：#[cfg(test)] mod tests { use super::*; #[test] fn t_retries() { assert_eq!(MAX_RETRIES, 3); } #[test] fn t_valid() { assert!(validate_timeout(30).is_ok()); } #[test] fn t_zero() { assert!(validate_timeout(0).is_err()); } }
4) 用 bash 执行 cargo test --manifest-path configlib/Cargo.toml —— t_retries 失败，诊断原因（常量与测试期望不符）
5) 用 write_file 修复 MAX_RETRIES 为 3
6) 用 bash 执行 cargo test --manifest-path configlib/Cargo.toml 确认通过
每步完成后立即进入下一步。' \
  --budget 60 \
  --acceptance 'cmd: cargo test --manifest-path configlib/Cargo.toml' \
  --acceptance 'file: configlib/src/lib.rs contains MAX_RETRIES: u32 = 3'
echo "N10A_EXIT=$?"

# Case B: environment/tool failure → adaptation → recovery（目标目录缺失拓扑）
cd /tmp/cfr_n10b
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=480
hearth chat '在已存在的目录 legacy_app/ 中添加一个模块文件。注意：legacy_app/ 的子目录结构可能与你预期的不同，先探查再行动：
1) 用 bash 执行 ls legacy_app/src/ 2>&1 —— 观察真实结构（可能目录不存在或路径不同）
2) 根据探查结果用 write_file 把 legacy_app/src/feature.rs 写入正确位置（若 src/ 不存在先用 bash mkdir -p 创建）
3) feature.rs 内容：pub fn feature_enabled() -> bool { true }
4) 用 bash 确认文件存在（ls -la）
5) 用 write_file 创建 legacy_app/FINAL.txt，内容写 ADAPTED_OK
每步完成后立即进入下一步。' \
  --budget 40 \
  --acceptance 'file: legacy_app/FINAL.txt contains ADAPTED_OK'
echo "N10B_EXIT=$?"
