# Codex-Rust v4.0 部署指南

## 前提

- Linux x86_64（sandbox 依赖 landlock + seccomp Linux 内核特性）
- Rust 工具链 1.82+（`rustup`）
- OpenAI 兼容 API 密钥

## 1. 编译

```bash
git clone <repo-url> codex-rust
cd codex-rust
cargo build --release
```

编译产物：`target/release/service`

## 2. 最小运行

```bash
export OPENAI_API_KEY=sk-xxxxxxxx
export API_KEY=your-secret-token  # 生产必设
./target/release/service
```

服务默认监听 `http://0.0.0.0:3000`。

## 3. 验证

```bash
# 健康检查
curl http://localhost:3000/healthz   # → "OK"
curl http://localhost:3000/readyz   # → "OK"

# OpenAPI 文档
curl http://localhost:3000/openapi.json | jq

# Swagger UI（浏览器打开）
# http://localhost:3000/swagger-ui/
```

## 4. 反向代理（nginx 示例）

```nginx
server {
    listen 80;
    server_name codex.example.com;

    location /sse {
        # SSE 需要长连接
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Connection '';
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 86400s;
    }

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## 5. systemd 服务

```ini
# /etc/systemd/system/codex.service
[Unit]
Description=Codex-Rust Agent Service
After=network.target

[Service]
Type=simple
User=codex
WorkingDirectory=/opt/codex
Environment="OPENAI_API_KEY=sk-xxxxxxxx"
Environment="API_KEY=your-secret-token"
Environment="PORT=3000"
ExecStart=/opt/codex/service
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now codex
sudo systemctl status codex
```

## 6. Docker（见 `Dockerfile` 和 `docker-compose.yml`）

```bash
docker build -t codex-rust:latest .
docker run -d -p 3000:3000 \
  -e OPENAI_API_KEY=sk-xxxxxxxx \
  -e API_KEY=your-secret-token \
  codex-rust:latest
```

注意：容器内 sandbox 需要 `--privileged` 或 `--security-opt seccomp=unconfined`（landlock 需要非特权用户命名空间可用）。如果宿主机 kernel 禁用 `unprivileged_userns_clone`，sandbox 会降级到无 landlock 模式。

## 7. API 端点一览

| Method | Path | Auth Required | Description |
|---|---|---|---|
| GET | `/healthz` | No | Liveness probe |
| GET | `/readyz` | No | Readiness probe |
| GET | `/openapi.json` | No | OpenAPI spec |
| GET | `/swagger-ui/` | No | Swagger UI |
| POST | `/api/v1/sessions` | Yes* | Create session |
| GET | `/api/v1/sessions/{id}` | Yes* | Session status |
| POST | `/api/v1/sessions/{id}/messages` | Yes* | Send message (SSE response) |
| GET | `/api/v1/sessions/{id}/messages` | Yes* | History replay |
| POST | `/api/v1/sessions/{id}/approvals` | Yes* | Submit tool approval |
| POST | `/api/v1/sessions/{id}/cancel` | Yes* | Cancel session |
| GET | `/api/v1/models` | Yes* | List available models |

*Auth required only if `API_KEY` env is set.
