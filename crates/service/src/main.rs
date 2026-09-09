use std::net::SocketAddr;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{
    routing::{get, get_service, post},
    Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;
use utoipa::OpenApi;

use llm_gateway::{FallbackChain, ProviderRegistry};
use llm_openai::OpenAiProvider;
use lsp_bridge::{NoopLspBridge, RustAnalyzerBridge};
use memory::JsonlMemoryStore;
use retriever::{Retriever, TantivyRetriever};
use service::routes;
use service::session;
use tool_runtime::{ToolContext, ToolDispatcher, ToolRegistry};
use tools_builtin::{BashTool, EditTool, GlobTool, GrepTool, ReadTool};

// ── B3 (trunk-freeze): OpenAPI document (components only; path annotations deferred) ──
#[derive(OpenApi)]
#[openapi(components(schemas(
    api::AgentEvent,
    api::SessionCreate,
    api::SessionCreateResponse,
    api::MessageReq,
    api::ApprovalReq,
    api::SessionStatus,
    api::ModelInfo,
    api::ErrorResponse,
    api::ErrorBody,
    api::SessionHistory,
    api::HistoryEntry,
)))]
struct ApiDoc;

// B3 (trunk-freeze): serve the generated OpenAPI document as JSON.
// Note: we deliberately avoid `utoipa-swagger-ui` — its build script downloads
// the Swagger UI bundle from github.com at compile time, which is unavailable
// in our CI/VM sandbox (no outbound access to github). The JSON endpoint is
// sufficient to expose the contract; the interactive UI can be vendored later
// for local dev if desired.
async fn openapi_json() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        ApiDoc::openapi()
            .to_json()
            .expect("serializing OpenAPI document must not fail"),
    )
}

/// v13 S3-b: Bridges agent-core's `CivWriter` to the concrete
/// `CivilizationStore` — the composition root is the ONLY place that sees
/// both sides (dependency inversion preserved). Locked by wiring assertion
/// `civ-auto-written` (docs/xray/wiring-v13.toml).
struct CivWriterAdapter {
    store: std::sync::Arc<memory::CivilizationStore>,
    /// P1-4 (audit-fix): 连续写失败计数——暴露到 /readyz（>5 次 → 503）。
    failures: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl agent_core::CivWriter for CivWriterAdapter {
    fn append_civ(&self, category: &str, content: &str, session_id: &str, tags: Vec<String>) {
        use agent_types::{CivAuthor, CivCategory, CivEntry};
        let cat = match category {
            "milestone" => CivCategory::Milestone,
            "lesson" => CivCategory::Lesson,
            "reflection" => CivCategory::Reflection,
            _ => CivCategory::Insight,
        };
        let entry = CivEntry {
            id: uuid::Uuid::new_v4().to_string(),
            author: CivAuthor {
                provider_model: "agent-loop/auto".to_string(),
                session_id: session_id.to_string(),
                bridge_id: None,
            },
            content: content.to_string(),
            category: cat,
            context: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            tags,
        };
        // Civ writing must never fail the agent loop — log and move on.
        // P1-4: 失败计数（连续失败暴露到 /readyz，不再是静默 warn）。
        if let Err(e) = self.store.append(entry) {
            self.failures
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tracing::warn!("civ auto-write failed: {e}");
        }
    }
}

/// P1-3 (audit-fix): 启动期 I/O 初始化失败 → 可操作错误信息 + exit(1)，不 panic。
/// 一个可恢复的配置/权限错误不应变成进程崩溃（CrashLoopBackOff 排障地狱）。
fn startup_fatal(context: &str, e: impl std::fmt::Display) -> ! {
    eprintln!(
        "FATAL: {context}: {e}
       MEMORY_DIR={:?} — 请检查目录是否存在且可写，然后重试。",
        std::env::var("MEMORY_DIR").unwrap_or_default()
    );
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // v7.0: structured logging initialization
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Load .env if present
    let _ = dotenvy::dotenv();

    // ── Build provider registry (P4: multi-backend via env) ──
    let mut registry = ProviderRegistry::new();

    // 1. OpenAI (always registered — primary)
    // D-extra (backend-intelligence): 占位 key 反模式根治——无 key 启动失败报原因
    // （与 CLI 路径 D2/D4 fail-closed 红线对齐；service 仅独立部署用，不静默降级）。
    let openai_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        startup_fatal(
            "OPENAI_API_KEY 未设置",
            "service 独立部署需要 API key（与 hearth CLI 的 fail-closed 一致，禁占位 key 静默降级）。请 export OPENAI_API_KEY=<key> 后重启。",
        );
    });
    let openai_url = std::env::var("OPENAI_BASE_URL").ok();
    // G11 (top-level-design): model name configurable via OPENAI_MODEL env.
    let openai_model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".into());
    info!("openai model: {openai_model}");
    // R2-C 采集器 v2: service 出口同样包 TelemetryProvider（全出口单点）
    let openai: Arc<dyn llm_gateway::LlmProvider> = Arc::new(llm_gateway::TelemetryProvider::wrap(
        Arc::new(OpenAiProvider::new(
            "openai",
            &openai_model,
            openai_url,
            openai_key.clone(),
        )),
        "service",
    ));
    // 6A v2: register with canonical "openai:{model}" key + alias
    registry.register_with_key("openai", &openai_model, openai, true);
    registry.register_alias("openai", &format!("openai:{openai_model}"));

    // 2. Ollama (optional — skip with warn if not configured)
    if let Ok(base_url) = std::env::var("OLLAMA_BASE_URL") {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qiyuan-8b:latest".into());
        let ollama = Arc::new(llm_local::OllamaProvider::new(
            "ollama",
            model,
            Some(base_url),
        ));
        registry.register(ollama, false);
        info!("ollama provider registered");
    } else {
        tracing::warn!("OLLAMA_BASE_URL not set — ollama provider skipped");
    }

    // 3. vLLM (optional — skip with warn if not configured)
    if let Ok(base_url) = std::env::var("VLLM_BASE_URL") {
        let model = std::env::var("VLLM_MODEL").unwrap_or_else(|_| "mistral-7b".into());
        let api_key = std::env::var("VLLM_API_KEY").ok();
        let vllm = Arc::new(llm_local::VllmProvider::new(
            "vllm",
            model,
            Some(base_url),
            api_key,
        ));
        registry.register(vllm, false);
        info!("vllm provider registered");
    } else {
        tracing::warn!("VLLM_BASE_URL not set — vllm provider skipped");
    }

    // 4. Hunyuan (optional — skip with warn if not configured)
    if let Ok(api_key) = std::env::var("HUNYUAN_API_KEY") {
        let model = std::env::var("HUNYUAN_MODEL").unwrap_or_else(|_| "hunyuan-pro".into());
        let base_url = std::env::var("HUNYUAN_BASE_URL").ok();
        let hunyuan = Arc::new(llm_cn::HunyuanProvider::new(
            "hunyuan", model, base_url, api_key,
        ));
        registry.register(hunyuan, false);
        info!("hunyuan provider registered");
    } else {
        tracing::warn!("HUNYUAN_API_KEY not set — hunyuan provider skipped");
    }

    // 5. Doubao (字节豆包) — OpenAI-compatible, multi-model rotation
    if let Ok(api_key) = std::env::var("DOUBAO_API_KEY") {
        let base_url = std::env::var("DOUBAO_BASE_URL")
            .unwrap_or_else(|_| "https://ark.cn-beijing.volces.com/api/v3".into());
        // Fallback chain: flash→pro→seed-code→seed-mini→glm5.2 (each has 50万 quota)
        let models: Vec<String> = std::env::var("DOUBAO_MODELS")
            .unwrap_or_else(|_| {
                "deepseek-v4-flash-260425,deepseek-v4-pro-260425,doubao-seed-2-0-code-preview-260215,doubao-seed-2-0-mini-260428,glm-5-2-260617".into()
            })
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for model in &models {
            let provider_name = if model == &models[0] {
                "doubao"
            } else {
                &format!("doubao-{}", model.split('-').next().unwrap_or(model))
            };
            let p = Arc::new(llm_openai::OpenAiProvider::new(
                provider_name,
                model,
                Some(base_url.clone()),
                &api_key,
            ));
            registry.register(p, false);
        }
        info!(
            "doubao providers registered ({} models: {:?})",
            models.len(),
            models
        );
    } else {
        tracing::warn!("DOUBAO_API_KEY not set — doubao provider skipped");
    }

    // 6. Agnes AI — free unlimited, OpenAI-compatible
    {
        // Y1-1: v22 硬编码真实 key 清零——key 只从 env 读，无 fallback 写死（密钥入库即泄露）
        let api_key = std::env::var("AGNES_API_KEY").unwrap_or_default();
        let model = std::env::var("AGNES_MODEL").unwrap_or_else(|_| "agnes-2.5-flash".into());
        let base_url =
            std::env::var("AGNES_BASE_URL").unwrap_or_else(|_| "https://api.agnes-ai.cn/v1".into());
        let agnes = Arc::new(llm_openai::OpenAiProvider::new(
            "agnes",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(agnes, false);
        info!("agnes provider registered (model={})", model);
    }

    // 7. ZhiPu (智谱 GLM) — OpenAI-compatible, large token quota
    {
        let api_key = std::env::var("ZHIPU_API_KEY").unwrap_or_default(); // v21: key via env/.env only
        let model = std::env::var("ZHIPU_MODEL").unwrap_or_else(|_| "glm-4.5-air".into());
        let base_url = std::env::var("ZHIPU_BASE_URL")
            .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4".into());
        let zhipu = Arc::new(llm_openai::OpenAiProvider::new(
            "zhipu",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(zhipu, false);
        info!("zhipu provider registered (model={})", model);
    }

    // 8. Google Gemini — OpenAI-compatible (⭐ 会员首选)
    {
        let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default(); // v21: key via env/.env only;
        let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.6-flash".into());
        let base_url = std::env::var("GEMINI_BASE_URL")
            .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/openai".into());
        let gemini = Arc::new(llm_openai::OpenAiProvider::new(
            "gemini",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(gemini, true); // true = required (会员首选)
        info!("gemini provider registered (model={})", model);
    }

    // 9. DeepSeek (官方直达) — OpenAI-compatible, 940ms (⭐ 主力)
    {
        let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(); // v21: key via env/.env only
        let model = std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".into());
        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
        let deepseek = Arc::new(llm_openai::OpenAiProvider::new(
            "deepseek",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(deepseek, true); // true = 主力
        info!("deepseek provider registered (model={})", model);
    }

    // 10. v15 S1b 天花板对照 — 同通道、同网关、只换大模型。
    //
    // 原计划用 gemini 做对照，但 v15 实测 gemini 3.x 在多轮 function calling 下
    // 要求回传 thought_signature，OpenAI 兼容层未透传 -> 第 2 步起 HTTP 400
    // （见 forge-report-v15.md「gemini 阻断」）。换通道会同时改变网关行为，
    // 无法把"失败"归因到模型强度上，因此改为在**已验证健康的同一通道**内
    // 只替换模型规模：flash -> pro / air -> 4.7。这样 S1b 的唯一自变量就是
    // 模型能力，正是天花板对照要测的东西。
    {
        let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(); // v21: key via env/.env only
        let model =
            std::env::var("DEEPSEEK_PRO_MODEL").unwrap_or_else(|_| "deepseek-v4-pro".into());
        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
        let pro = Arc::new(llm_openai::OpenAiProvider::new(
            "deepseek-pro",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(pro, false);
        info!("deepseek-pro ceiling provider registered (model={})", model);

        let zk = std::env::var("ZHIPU_API_KEY").unwrap_or_default(); // v21: key via env/.env only
        let zmodel = std::env::var("ZHIPU_MAX_MODEL").unwrap_or_else(|_| "glm-4.7".into());
        let zbase = std::env::var("ZHIPU_BASE_URL")
            .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4".into());
        let zmax = Arc::new(llm_openai::OpenAiProvider::new(
            "zhipu-max",
            &zmodel,
            Some(zbase),
            &zk,
        ));
        registry.register(zmax, false);
        info!("zhipu-max ceiling provider registered (model={})", zmodel);
    }

    // ── v15 line B: deterministic replay provider (env-gated, name = "replay") ──
    // REPLAY_DIR points at bench/replay/fixtures. Serves recorded assistant turns
    // so the LLM side is pinned and only the harness (tools/state machine/budget/
    // approval) is under test. Never registered as default.
    if let Ok(dir) = std::env::var("REPLAY_DIR") {
        match llm_replay::ReplayProvider::load_dir(&dir) {
            Ok(p) => {
                let n = p.session_count();
                registry.register(Arc::new(p), false);
                info!(
                    "replay provider registered ({} recorded sessions from {dir})",
                    n
                );
            }
            Err(e) => tracing::warn!("REPLAY_DIR set but replay provider failed to load: {e}"),
        }
    }

    // ── P4/P5: FallbackChain (env-gated, registered as "fallback" provider) ──
    if std::env::var("FALLBACK_CHAIN")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
    {
        let chain_providers: Vec<(String, Arc<dyn llm_gateway::LlmProvider>)> = registry
            .list()
            .into_iter()
            .filter_map(|name| registry.get(&name).ok().map(|p| (name, p)))
            .collect();
        if !chain_providers.is_empty() {
            let chain_names: Vec<String> = chain_providers.iter().map(|(n, _)| n.clone()).collect();
            let chain = Arc::new(FallbackChain::new(chain_providers));
            registry.register(chain, false);
            info!(
                "fallback chain registered as 'fallback' provider with {} backends: {:?}",
                chain_names.len(),
                chain_names,
            );
        }
    } else {
        info!("FALLBACK_CHAIN not enabled (set FALLBACK_CHAIN=1 to activate)");
    }

    let registry = Arc::new(registry);

    // ── Build tool dispatcher ──
    // E3 v5.0: config-based registration — all tools enabled by default,
    // disable with CODEX_DISABLE_TOOLS=bash,glob (comma-separated).
    let disabled_tools: std::collections::HashSet<String> = std::env::var("CODEX_DISABLE_TOOLS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let mut dispatcher = ToolDispatcher::new();
    if !disabled_tools.contains("bash") {
        dispatcher.register(Arc::new(BashTool::new()));
    }
    if !disabled_tools.contains("read") {
        dispatcher.register(Arc::new(ReadTool::new()));
    }
    if !disabled_tools.contains("edit") {
        dispatcher.register(Arc::new(EditTool::new()));
    }
    // WS2 (v0.2): SEARCH/REPLACE diff 编辑（小步精确、无截断）
    if !disabled_tools.contains("apply_patch") {
        dispatcher.register(Arc::new(tools_builtin::PatchTool::new()));
    }
    if !disabled_tools.contains("glob") {
        dispatcher.register(Arc::new(GlobTool::new()));
    }
    if !disabled_tools.contains("grep") {
        dispatcher.register(Arc::new(GrepTool::new()));
    }
    let dispatcher = Arc::new(dispatcher);

    // Build tool context
    // T10 (v0.2.3): 出网白名单注入 ctx.env——service 模式从进程 env 读取（HEARTH_EGRESS_ALLOWLIST）
    // RC13 (P3-BACKLOG): 补读 config.toml 的 egress_allowlist（与 CLI 侧 merge_allowlist 同构）——
    // 此前 service 只读 env，config 设置的白名单在 service 会话内不生效。
    let mut ctx = ToolContext::default();
    {
        let mut merged: Vec<String> = Vec::new();
        if let Ok(al) = std::env::var("HEARTH_EGRESS_ALLOWLIST") {
            for item in al.split(',') {
                let t = item.trim().to_lowercase();
                if !t.is_empty() && !merged.contains(&t) {
                    merged.push(t);
                }
            }
        }
        // RC13: config.toml（~/.config/hearth/config.toml）egress_allowlist 并入
        if let Ok(home) = std::env::var("HOME") {
            let cfg_path = std::path::PathBuf::from(home).join(".config/hearth/config.toml");
            if let Ok(body) = std::fs::read_to_string(&cfg_path) {
                #[derive(serde::Deserialize, Default)]
                struct MinimalCfg {
                    #[serde(default)]
                    egress_allowlist: Option<Vec<String>>,
                }
                if let Ok(mc) = toml::from_str::<MinimalCfg>(&body) {
                    if let Some(items) = mc.egress_allowlist {
                        for item in items {
                            let t = item.trim().trim_start_matches('.').to_lowercase();
                            if !t.is_empty() && !merged.contains(&t) {
                                merged.push(t);
                            }
                        }
                    }
                }
            }
        }
        if !merged.is_empty() {
            ctx.env
                .insert("HEARTH_EGRESS_ALLOWLIST".to_string(), merged.join(","));
        }
    }

    // Build session manager
    let mut sessions = session::SessionManager::new(registry.clone(), dispatcher, ctx);
    // Q3 (v24-post): Observer 注入——会话结束后评估事件流并落盘报告
    // （report.md/json → CODEX_OBSERVER_DIR/reports/{sid}/）。
    sessions.set_observer(Arc::new(observer::Observer::new()));

    // v10.4: Model auto-discovery — load from providers.json if present
    let providers_path = std::path::PathBuf::from(
        std::env::var("PROVIDERS_PATH").unwrap_or_else(|_| "./providers.json".into()),
    );
    if !providers_path.exists() {
        let default_providers = serde_json::json!({
            "models": [
                {"name": "gpt-4o", "provider": "openai", "default": true},
                {"name": "gpt-4o-mini", "provider": "openai"},
                {"name": "claude-sonnet-4-20250514", "provider": "anthropic"}
            ]
        });
        let _ = std::fs::write(
            &providers_path,
            serde_json::to_string_pretty(&default_providers).unwrap_or_default(),
        );
        tracing::info!(path=%providers_path.display(), "wrote default providers.json");
    }
    if let Ok(data) = std::fs::read_to_string(&providers_path) {
        if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(models) = cfg.get("models").and_then(|m| m.as_array()) {
                for m in models {
                    let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                    let provider = m
                        .get("provider")
                        .and_then(|p| p.as_str())
                        .unwrap_or("openai");
                    tracing::info!(name, provider, "model auto-discovered");
                }
            }
        }
    }

    // P5 A2: Wire JsonlMemoryStore for session persistence
    let memory_dir = std::env::var("MEMORY_DIR").unwrap_or_else(|_| "./memory".into());
    let memory_store = Arc::new(JsonlMemoryStore::new(&memory_dir));
    sessions.set_memory_store(memory_store);

    // E1 v5.0: retriever always enabled — semantic with embed provider (key always present).
    {
        let embed_model =
            std::env::var("EMBED_MODEL").unwrap_or_else(|_| "text-embedding-3-small".into());
        let embed_provider = Arc::new(OpenAiProvider::new(
            "embed",
            &embed_model,
            std::env::var("OPENAI_BASE_URL").ok(),
            openai_key.clone(),
        ));
        let mut retriever = TantivyRetriever::new();
        retriever.set_embed_provider(embed_provider);
        // v10.4: Real build — scan source tree for semantic search index
        let code_dir = std::env::var("CODE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });
        let mut chunks: Vec<code_index::CodeChunk> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&code_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "rs") {
                    if let Ok(content) = std::fs::read_to_string(&p) {
                        let preview = content.chars().take(4096).collect::<String>();
                        let line_count = preview.lines().count().max(1);
                        chunks.push(code_index::CodeChunk {
                            file: p.clone(),
                            start_line: 1,
                            end_line: line_count,
                            content: preview,
                            symbols: vec![],
                        });
                    }
                }
            }
        }
        tracing::info!(
            chunk_count = chunks.len(),
            dir = %code_dir.display(),
            "retriever: indexed source files"
        );
        retriever
            .build(&chunks, &[])
            .await
            .unwrap_or_else(|e| startup_fatal("retriever build failed", e));
        sessions.set_retriever(Arc::new(retriever));
        info!("retriever wired (embed model: {embed_model})");
    }

    // P3: LSP_ENABLED=1 enables real rust-analyzer bridge; else Noop.
    let lsp_enabled = std::env::var("LSP_ENABLED")
        .map(|v| v == "1")
        .unwrap_or(false);
    if lsp_enabled {
        tracing::info!("LSP real bridge: rust-analyzer (LSP_ENABLED=1)");
        sessions.set_lsp_bridge(Arc::new(RustAnalyzerBridge::new()));
    } else {
        sessions.set_lsp_bridge(Arc::new(NoopLspBridge::new()));
    }

    info!("memory store wired: {memory_dir}");

    // v11.0: Experience store for self-evolution
    // v16.0: File persistence enabled — JSONL stored alongside civilization data.
    // v21.0: removed zhipu embedding-3 injection (v18 proved embedding ≤ keyword;
    //        dead code + hardcoded key cleaned up). Store stays keyword-only.
    let experience_store = Arc::new(experience::ExperienceStore::new());
    let experience_path = std::path::PathBuf::from(&memory_dir).join("experience.jsonl");
    if let Err(e) = experience_store.set_path(experience_path.clone()).await {
        tracing::warn!(path = %experience_path.display(), "experience store path set failed: {e}");
    }
    sessions.set_experience_store(experience_store.clone());

    // 6C: civilization store (v13 S3-b: constructed BEFORE Arc::new(sessions)
    // so the CivWriter can be injected while sessions is still mutable).
    let civ_store = Arc::new(
        memory::CivilizationStore::new(
            &std::path::PathBuf::from(&memory_dir),
            "civilization.jsonl",
        )
        .unwrap_or_else(|e| startup_fatal("failed to init civilization store", e)),
    );
    // v13 S3-b: wire civ auto-write into every future AgentLoop.
    let civ_write_failures: std::sync::Arc<std::sync::atomic::AtomicU64> =
        std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    sessions.set_civ_writer(Arc::new(CivWriterAdapter {
        store: civ_store.clone(),
        failures: civ_write_failures.clone(),
    }));

    let sessions = Arc::new(sessions);

    // OOM-1 (global-audit): periodically evict finished sessions from the
    // in-memory map (TTL 1h, sweep every 5min) — it otherwise grows forever.
    // S1 (trunk-freeze): TTL configurable via env, defaults preserved.
    let oom_ttl = std::env::var("OOM_TTL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(3600));
    let oom_sweep = std::env::var("OOM_SWEEP_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(300));
    sessions.start_cleanup_task(oom_ttl, oom_sweep);

    // AUTH-0 (global-audit): optional Bearer-token auth via API_KEY env.
    // P0-1 (audit-fix): 默认安全反转——无 key 时默认拒绝启动（fail closed）；
    // 只有显式 ALLOW_NO_AUTH=1 才允许无鉴权模式，且该模式只绑 127.0.0.1。
    let api_key = std::env::var("API_KEY").ok().filter(|k| !k.is_empty());
    let allow_no_auth = std::env::var("ALLOW_NO_AUTH")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    if api_key.is_some() {
        info!("API auth enabled (Authorization: Bearer <API_KEY> required)");
    } else if allow_no_auth {
        tracing::warn!(
            "ALLOW_NO_AUTH=1 — API is OPEN but bound to loopback only \
             (127.0.0.1). For production set API_KEY."
        );
    } else {
        anyhow::bail!(
            "API_KEY is not set and ALLOW_NO_AUTH is not set. \
             Refusing to start an open API (insecure by default). \
             Set API_KEY for auth, or ALLOW_NO_AUTH=1 for loopback-only dev."
        );
    }

    // v10.0 10A: instance identity
    let instance_id = format!("codex-inst-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    // Write PID file for Observer health checks
    let _ = std::fs::write("/tmp/codex.pid", std::process::id().to_string());
    tracing::info!(
        "[instance: {instance_id}] mounted, pid={}",
        std::process::id()
    );

    // 6C: civ_store constructed earlier (v13 S3-b) — reused in AppState below.

    // 6D: work line store
    let workline_store = Arc::new(
        memory::WorkLineStore::new(&std::path::PathBuf::from(&memory_dir), "workline.jsonl")
            .unwrap_or_else(|e| startup_fatal("failed to init workline store", e)),
    );

    // v10.4: WorkLine 60s background scheduler
    let wl_bg = workline_store.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let pending = wl_bg.list(Some("pending"));
            if !pending.is_empty() {
                tracing::info!(workline_pending = pending.len(), "workline scheduler tick");
                // Advance any stale pending nodes
                for node in pending {
                    let _ = wl_bg.update(&node.id, node.progress + 1.0, None);
                }
            }
        }
    });

    // v7.0: telemetry collector
    let telemetry = Arc::new(routes::TelemetryCollector::default());

    // v8.0: per-user store multiplexer
    let per_user = Arc::new(service::per_user::PerUserStore::new(std::path::Path::new(
        &memory_dir,
    )));

    // v8.0: agent templates
    let templates_dir =
        std::env::var("CODEX_TEMPLATES_DIR").unwrap_or_else(|_| "./templates".into());
    let template_manager = Arc::new(
        service::templates::TemplateManager::load(std::path::Path::new(&templates_dir))
            .unwrap_or_else(|_| {
                service::templates::TemplateManager::load(std::path::Path::new(".")).unwrap_or_else(
                    |_| {
                        service::templates::TemplateManager::load(std::env::temp_dir().as_path())
                            .unwrap_or_else(|e| {
                                startup_fatal("cannot init template manager (tried all dirs)", e)
                            })
                    },
                )
            }),
    );

    // v8.0: multi-user store
    let user_store = Arc::new(service::user::UserStore::new(&[]));
    if let Some(ref key) = api_key {
        user_store.register(key.clone(), "default".into());
    }

    // v8.0: webhook manager
    let webhooks = Arc::new(service::webhook::WebhookManager::new());

    // v10.5: Tool registry with auto-discovery
    let tool_registry = Arc::new(ToolRegistry::new());
    let tools_dir = std::env::var("TOOLS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./tools"));
    let discovered = tool_registry.load_from_dir(&tools_dir).unwrap_or_else(|e| {
        tracing::warn!("tool auto-discovery: {}", e);
        0
    });
    if discovered > 0 {
        tracing::info!(count = discovered, "tools auto-discovered");
    }

    // Build app state
    let observer_iid = instance_id.clone();
    // v20.0 S3: experience store handle for the observer's prune/upgrade loop.
    let observer_experience = experience_store.clone();
    let state = Arc::new(routes::AppState {
        sessions,
        api_key,
        allow_no_auth,
        civ_write_failures,
        civ_store,
        workline_store,
        telemetry,
        // WP-4 (v23 phase4): Observer（第三权，零执行权）——L2 fail-closed：
        // 构造失败 → service 拒启（Observer 是纯结构体，new() 无失败路径，
        // 但保持 Arc 注入以便后续 L2 检查接入）。
        observer: Arc::new(observer::Observer::new()),
        user_store,
        template_manager,
        experience_store,
        tool_registry,
        webhooks,
        per_user,
        instance_id,
    });

    // v10.0 10D: observer background task (hourly JSONL)
    tokio::spawn(async move {
        let d = std::env::var("CODEX_OBSERVER_DIR").unwrap_or_else(|_| "./observer".into());
        let _ = std::fs::create_dir_all(&d);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            let df = format!(
                "{}/daily-{}.jsonl",
                d,
                chrono::Local::now().format("%Y-%m-%d")
            );
            let s = resource_monitor::snapshot(std::process::id());
            let e = serde_json::json!({"ts":chrono::Utc::now().to_rfc3339(),"iid":observer_iid,"cpu":s.cpu_percent,"mem":s.memory_mb,"disk_gb":s.disk_free_gb});
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&df)
            {
                let _ = std::io::Write::write_all(&mut f, format!("{}\n", e).as_bytes());
            }

            // v20.0 S3: experience store maintenance — prune weak/old entries and
            // promote high-value ones to core. Runs on the same hourly cadence.
            // prune: effectiveness < 0.3 AND older than 90 days are removed.
            let pruned = observer_experience.prune(0.3, 90).await;
            let cores = observer_experience.upgrade_core().await;
            if pruned > 0 || !cores.is_empty() {
                tracing::info!(
                    pruned,
                    core_candidates = cores.len(),
                    "experience maintenance (v20 observer)"
                );
            }
        }
    });

    // CORS — restrict allowed origin via CORS_ORIGIN env (L6). Defaults to the
    // local dev frontend. API-3 (global-audit): methods and headers are now an
    // explicit allowlist instead of `Any`.
    let allowed_origin = std::env::var("CORS_ORIGIN")
        .ok()
        .and_then(|o| o.parse::<axum::http::HeaderValue>().ok())
        .map(|v| vec![v])
        .unwrap_or_else(|| vec!["http://localhost:5173".parse().unwrap()]);
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origin))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    // Build router
    let app = Router::new()
        // P4 v4.1: health check endpoints (no auth required — bypass in require_api_key)
        .route("/healthz", get(routes::healthz))
        .route("/readyz", get(routes::readyz))
        .route(
            "/api/v1/sessions",
            get(routes::list_sessions).post(routes::create_session),
        )
        .route("/api/v1/sessions/:id", get(routes::get_session_status))
        .route("/api/v1/sessions/:id/messages", post(routes::send_message))
        .route(
            "/api/v1/sessions/:id/approvals",
            post(routes::submit_approval),
        )
        // WP-0: 通用交互响应路由（approvals 的泛化契约）
        .route(
            "/api/v1/sessions/:id/interaction/:iid",
            post(routes::submit_interaction),
        )
        .route("/api/v1/sessions/:id/cancel", post(routes::cancel_session))
        // WP-2: 录制导出（JSONL 信封事件流）
        .route(
            "/api/v1/sessions/:id/events",
            get(routes::session_events_export),
        )
        // WP-10: 实时 SSE 流（EventSource GET；/events 是 JSONL 录制非流）
        .route("/api/v1/sessions/:id/stream", get(routes::session_stream))
        // WP-8: 产物预览（路径校验 workspace 内）
        .route(
            "/api/v1/sessions/:id/artifact/open",
            get(routes::open_artifact),
        )
        // B3-3 (backend taskbook #01): 系统打开产物（file→xdg-open / url→浏览器）
        .route(
            "/api/v1/sessions/:id/artifact/open-external",
            get(routes::open_external),
        )
        .route("/api/v1/models", get(routes::list_models))
        // B1 (trunk-freeze): GET history replay on the same path as POST send_message.
        .route(
            "/api/v1/sessions/:id/messages",
            get(routes::get_session_history),
        )
        // B3 (trunk-freeze): serve OpenAPI doc as JSON (no compile-time github
        // download — see `openapi_json` above for rationale).
        .route("/openapi.json", get(openapi_json))
        // 6C v6.0: civilization line — collective AI memory
        .route("/api/v1/civilization", get(routes::get_civ_feed))
        .route("/api/v1/civilization", post(routes::post_civ_entry))
        // 6D v6.0: work line — task board
        .route("/api/v1/workline", get(routes::get_workline))
        .route("/api/v1/workline/nodes", post(routes::create_work_node))
        .route(
            "/api/v1/workline/nodes/:id",
            axum::routing::patch(routes::update_work_node),
        )
        // v6.1 L1: bridge discussion
        .route("/api/v1/bridge", post(routes::create_bridge))
        // v7.0: telemetry
        .route("/api/v1/telemetry", get(routes::get_telemetry))
        // v8.0: templates
        .route("/api/v1/templates", get(routes::list_templates))
        // v8.0: webhooks
        .route("/api/v1/webhooks", post(routes::register_webhook))
        // v10.0: resources
        .route("/api/v1/resources", get(routes::get_resources))
        // v10.0: tools
        .route("/api/v1/tools", get(routes::list_tools))
        // v10.5: tool ecosystem
        .route("/api/v1/tools/search", get(routes::search_tools))
        .route("/api/v1/tools/install", post(routes::install_tool))
        // v11.3: experience metrics
        .route(
            "/api/v1/experience/metrics",
            get(routes::experience_metrics),
        )
        // B3 (trunk-freeze): serve the interactive Swagger UI from vendored
        // assets (offline — no compile-time or runtime network fetch).
        .nest_service(
            "/swagger-ui",
            get_service(ServeDir::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/swagger-ui"
            ))),
        )
        // AUTH-0: auth runs inside CORS so preflight OPTIONS still works.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            routes::require_api_key,
        ))
        // P1 v4.1: concurrent request limiter (max 50 simultaneous requests).
        // Return 429 Too Many Requests when at capacity.
        .layer(axum::middleware::from_fn(routes::concurrency_limit))
        // API-1: axum already applies a 2 MB default body limit; make it
        // explicit so it survives future `Router` refactors.
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(cors)
        .with_state(state);

    // Bind — port configurable via PORT env var (L5), default 3000.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    // P0-1: 无鉴权模式只绑回环，防局域网暴露
    let bind_ip = if allow_no_auth {
        [127, 0, 0, 1]
    } else {
        [0, 0, 0, 0]
    };
    let addr = SocketAddr::from((bind_ip, port));
    info!("Codex Agent Service starting on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    // P2-1 (audit-fix): 注入 ConnectInfo<SocketAddr>——限流中间件按 per-IP 维度计数
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
