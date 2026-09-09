#!/bin/sh
# Hearth 安装脚本（v0.1 发布：预编译安装器——curl | sh 风格）
#
# 用法（二选一）:
#   # ① 本地 tarball（当前 v0.1 交付模式——GitHub Release 未发）
#   LOCAL_TARBALL=/path/to/hearth-v0.1-x86_64-unknown-linux-gnu.tar.gz sh install.sh
#
#   # ② 远程 tarball（发布到公网后）
#   HEARTH_BASE_URL=https://<host>/hearth/releases/download sh install.sh
#
# 源码安装（功能一致，推荐开发/验证）:
#   cargo build --release -p codex-cli && install -m755 target/release/hearth ~/.cargo/bin/hearth
#
# 铁律（v0.1 任务书红线）：不留 example.com 占位——未指定安装源即报错引导，
# 绝不静默向占位地址下载。

set -e

VERSION="${1:-v0.2.8}"
BIN_DIR="${HEARTH_BIN_DIR:-/usr/local/bin}"
LOCAL_TARBALL="${LOCAL_TARBALL:-}"
BASE_URL="${HEARTH_BASE_URL:-}"

say() { printf '\033[1;36m%s\033[0m\n' "$*"; }
die() { printf '\033[1;31m%s\033[0m\n' "$*" >&2; exit 1; }

# 安装源校验：本地 tarball 或远程 URL 至少其一（不留占位）
if [ -z "$LOCAL_TARBALL" ] && [ -z "$BASE_URL" ]; then
    die "未指定安装源。二选一：
  ① 本地模式:  LOCAL_TARBALL=<tarball 路径> sh install.sh
  ② 远程模式:  HEARTH_BASE_URL=<下载基础 URL> sh install.sh
  （源码安装: cargo build --release -p codex-cli && install -m755 target/release/hearth $BIN_DIR/hearth）"
fi

# 平台检测（当前仅 Linux x86_64 有预编译产物；其他平台引导源码安装）
OS="$(uname -s)"
ARCH="$(uname -m)"
if [ "$OS" = "Linux" ] && [ "$ARCH" = "x86_64" ]; then
    TARGET="x86_64-unknown-linux-gnu"
else
    die "预编译产物暂不支持 $OS/$ARCH——请源码安装: cargo build --release -p codex-cli && install -m755 target/release/hearth $BIN_DIR/hearth"
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [ -n "$LOCAL_TARBALL" ]; then
    say "从本地 tarball 安装: $LOCAL_TARBALL ..."
    if [ ! -f "$LOCAL_TARBALL" ]; then
        die "找不到本地 tarball: $LOCAL_TARBALL"
    fi
    cp "$LOCAL_TARBALL" "$TMP/hearth.tar.gz"
else
    URL="$BASE_URL/$VERSION/hearth-$TARGET.tar.gz"
    say "下载 $URL ..."
    if ! curl -fsSL "$URL" -o "$TMP/hearth.tar.gz"; then
        die "下载失败: $URL（若版本不存在，用 'latest' 或指定 tag）"
    fi
fi

say "解压并安装到 $BIN_DIR ..."
tar -xzf "$TMP/hearth.tar.gz" -C "$TMP"
install -d "$BIN_DIR"
install -m755 "$TMP/hearth" "$BIN_DIR/hearth"
if [ -f "$TMP/codex" ]; then
    install -m755 "$TMP/codex" "$BIN_DIR/codex"
fi

say "✓ 已安装: $BIN_DIR/hearth（+ 别名 codex）"
if ! command -v hearth >/dev/null 2>&1; then
    say "提示: $BIN_DIR 不在 PATH——加一行: export PATH=\"$BIN_DIR:\$PATH\""
fi

# 首次引导提示（无 key 时 hearth chat 会给出可行动错误）
say "下一步: hearth init   （交互式填 API key）"
say "或:      hearth config set api-key <key>"
