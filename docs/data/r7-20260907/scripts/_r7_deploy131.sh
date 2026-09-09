set -e
export PATH="$HOME/.cargo/bin:$PATH"
cd /home/wutao
if [ ! -d ~/codex-r6/.git ]; then
  git clone -b p0-usability-01 --single-branch r7.bundle codex-r6 2>&1 | tail -2
else
  cd ~/codex-r6 && git pull /home/wutao/r7.bundle p0-usability-01 --ff-only 2>&1 | tail -1
fi
cd ~/codex-r6
git log --oneline -1
cargo build --release -p codex-cli 2>&1 | tail -1
./target/release/hearth --version
md5sum ./target/release/hearth
echo "=== sandbox tests（R7-2 验证）==="
cargo test -p sandbox --lib 2>&1 | tail -4
