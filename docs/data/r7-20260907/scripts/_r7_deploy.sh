set -e
export PATH="$HOME/.cargo/bin:$PATH"
cd /home/wutao/codex-r6
git pull /home/wutao/r7.bundle p0-usability-01 --ff-only 2>&1 | tail -1
git log --oneline -1
cargo build --release -p codex-cli 2>&1 | tail -1
./target/release/hearth --version
md5sum ./target/release/hearth
