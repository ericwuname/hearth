#!/bin/bash
# Y1-4: Docker build 自动化验证（VM）
# 流程：等 rust 镜像拉完 → 拉 debian → docker build → run → healthz/readyz → 日志落盘
LOG=/home/wutao/docker_verify.log
exec > "$LOG" 2>&1

echo "=== [1] 等待 rust:1.82-bookworm 拉取完成 ==="
while ! grep -q "PULL_RC=" /home/wutao/pull_rust3.log 2>/dev/null; do
    sleep 60
done
grep "PULL_RC=" /home/wutao/pull_rust3.log
docker images | head -4

echo "=== [2] 拉取 debian:bookworm-slim ==="
docker pull debian:bookworm-slim
echo "PULL2_RC=$?"

echo "=== [3] docker build（legacy builder 走 daemon mirrors + 本地层缓存）==="
cd ~/codex
DOCKER_BUILDKIT=0 docker build -t codex-rust:v22 .
echo "BUILD_RC=$?"

echo "=== [4] docker run + 健康检查 ==="
# 宿主 3000 已被旧 service 占用 → 映射 3001
docker run --rm -d -p 3001:3000 --name codex-docker-test codex-rust:v22
sleep 12
echo "--- /healthz ---"
curl -s -w "\nHTTP=%{http_code}\n" http://127.0.0.1:3001/healthz
echo "--- /readyz ---"
curl -s -w "\nHTTP=%{http_code}\n" http://127.0.0.1:3001/readyz
echo "--- 容器日志（前 20 行）---"
docker logs codex-docker-test 2>&1 | head -20
echo "--- 容器进程 ---"
docker ps | grep codex-docker-test
docker stop codex-docker-test 2>/dev/null

echo "=== [5] 沙箱能力检查（容器内 sandbox 模块加载）==="
docker run --rm codex-rust:v22 /usr/local/bin/codex-service --help 2>&1 | head -8

echo "ALL_DONE"
