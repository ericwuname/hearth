export PATH="$HOME/.cargo/bin:$PATH"
cd ~/codex-r6
cargo test -p sandbox --lib test_linux_sandbox_echo 2>&1 | tail -20
