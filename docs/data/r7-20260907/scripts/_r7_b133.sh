export PATH="$HOME/.cargo/bin:$PATH"
cd /home/wutao/codex-r6
git log --oneline -1
nohup cargo build --release -p codex-cli > /home/wutao/r7-build.log 2>&1 &
echo "build pid=$!"
