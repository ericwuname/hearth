export PATH="$HOME/.cargo/bin:$PATH"
cd ~/codex-r6
timeout 550 cargo build --release -p codex-cli 2>&1 | tail -2
./target/release/hearth --version
md5sum ./target/release/hearth
