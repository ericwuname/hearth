#!/bin/bash
# FA01 Node 13 run script — wordcount 非 mathlib Product 任务
rm -rf /tmp/fa_node13
mkdir -p /tmp/fa_node13
cd /tmp/fa_node13
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=600
hearth chat '创建一个 Rust 二进制项目 wordcount/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p wordcount/src
2) 用 write_file 创建 wordcount/Cargo.toml，内容为 [package] name=wordcount version=0.1.0 edition=2021
3) 用 write_file 创建 wordcount/src/lib.rs，实现：
   pub fn top_words(s: &str, n: usize) -> Vec<(String, usize)> { ... }
   （按空白切词、小写化、统计词频、按频次降序取前 n 个，频次相同按字母序）
4) 用 write_file 在 lib.rs 加测试模块：
   #[cfg(test)]
   mod tests {
       #[test]
       fn t_top() {
           assert_eq!(top_words("b a b", 1), vec![("b".to_string(), 2usize)]);
       }
   }
5) 用 bash 执行 cargo test --manifest-path wordcount/Cargo.toml —— 若失败，先记录输出，再用 write_file 修复 lib.rs 直到通过（唯一修复循环）
6) 用 write_file 创建 wordcount/src/main.rs：从 stdin 读入全部文本，调用 top_words(input.trim(), 3)，每行输出 "word count"
7) 用 bash 执行 printf "b a b c b" | cargo run --manifest-path wordcount/Cargo.toml -q 2>/dev/null 记录输出
8) 用 write_file 创建 RESULT.txt，内容写 WORDCOUNT_DONE。以上步骤完成后立即宣告任务完成（DONE），不要做额外验证' \
  --budget 50 \
  --acceptance 'cmd: cargo test --manifest-path wordcount/Cargo.toml' \
  --acceptance 'file: wordcount/src/main.rs contains top_words' \
  --acceptance 'file: RESULT.txt contains WORDCOUNT_DONE'
echo "EXIT_CODE=$?"
