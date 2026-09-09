export PATH="$HOME/.cargo/bin:$PATH"
cd ~/codex-r6
cargo test -p sandbox --lib 2>&1 | grep -E "^test |FAILED|panicked|assert" | head -40
