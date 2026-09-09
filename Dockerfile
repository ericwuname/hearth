# ── Build stage ──
FROM rust:1.82-bookworm AS builder
# v22.1: Cargo.lock 依赖需 edition2024（Rust 1.85+ 稳定）——1.82 工具链过旧。
# 慢速/不稳定网络下 rustup 下载大组件易中断：rustup 内置重试 + shell 循环兜底
ENV RUSTUP_MAX_RETRIES=10 RUSTUP_HTTP_TIMEOUT=180
RUN for i in $(seq 1 5); do \
        rustup toolchain install stable --profile minimal && break; \
        echo "rustup retry $i"; sleep 8; \
    done && rustup default stable
WORKDIR /build
COPY . .
RUN cargo build --release && cp target/release/service /build/service

# ── Runtime stage ──
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates bash ripgrep git && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/service /usr/local/bin/codex-service
RUN mkdir -p /workspace /data
WORKDIR /workspace
EXPOSE 3000
ENV PORT=3000 \
    MEMORY_DIR=/data \
    RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/codex-service"]
