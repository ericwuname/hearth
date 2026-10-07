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
use memory::JsonlMemoryStore;
use service::routes;
use service::session;
use tool_runtime::{ToolContext, ToolDispatcher, ToolRegistry};

// ── D-90（2026-10-02, traecode）：paths 由 `service::openapi::API_ROUTES` 注入 ──
//
// 下面 `ApiDoc` 只声明 schemas（保持原样）；`paths` 由镜像表在运行时注入
// （见 `crates/service/src/openapi.rs` 的模块注释：病灶、处置、以及由
// `tests/openapi_route_gate.rs` 兜底的保真机制）。
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

// B3 (trunk-freeze): 以 JSON 暴露 OpenAPI 文档。
// 不使用 `utoipa-swagger-ui` 的原因：其 build script 在**编译期**从 github.com 下载
// Swagger UI bundle，而 CI/VM 沙箱无出网 ⇒ 改为 **vendor 静态 UI**（下方
// `.nest_service("/swagger-ui", …)`，读的正是本端点）。
// （原注释称"交互 UI 可日后再 vendor"——已过期：它**早已 vendor**，此处据实订正。）
//
// D-90（2026-10-02, traecode）两处收口：
// ① **注入 paths**：此前 `ApiDoc` 只有 `components(schemas(…))`、**零 paths**，
//    于是对外提供的是一份"**声明了 0 个端点**的 OpenAPI 文档"，Swagger UI 打开即空列表
//    （服务实际有 27 条路由）——典型"半接线活特性"。现由 `service::openapi::API_ROUTES`
//    镜像表注入（保真由 `tests/openapi_route_gate.rs` 双向兜底）。
// ② **去掉 `expect` panic**：请求路径里不得 panic（D-60 同族），序列化失败如实回 500。
async fn openapi_json() -> axum::response::Response {
    match service::openapi::build_openapi_document(ApiDoc::openapi()).to_json() {
        Ok(body) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "OpenAPI 文档序列化失败");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "openapi document unavailable",
            )
                .into_response()
        }
    }
}

/// v13 S3-b: Bridges agent-core's `CivWriter` to the concrete
/// `CivilizationStore` — the composition root is the ONLY place that sees
/// both sides (dependency inversion preserved). Locked by wiring assertion
/// `civ-auto-written` (docs/xray/wiring-v13.toml).
///
/// D-109（2026-10-02, traecode）**已收口**：本适配器曾写**全局**文明线档
/// （`MEMORY_DIR/civilization.jsonl`），而 HTTP 读侧 `get_civ_feed` 读的是 **per-user**
/// 档（`per_user.civ_for(uid)`）⇒ 经此写入的 milestone/reflection 在 API 上**不可见**
/// （D-108 病灶的另一半）。修法是给这条写入补 **session → 归属用户** 的链，且**写侧对齐
/// 读侧**（不能反过来让读侧合并全局档：全局档含各用户 goal 文本 ⇒ 跨租户泄露）。
///
/// 实现：本适配器现在**绑定单个 owner**（`owner` 字段），经 `per_user.civ_for(owner)`
/// 落档——由组合根注册的**工厂**在 `create_session_with_owner(owner)` 时按 owner 构造
/// （见下方 `set_civ_writer_factory` 调用）。`PerUserStore::civ_for` 是同步方法，故
/// `append_civ`（同步 trait）内可直接落档，无需异步查表。全局 `civilization.jsonl`
/// 已随之**停止构造**（无人读它）。
struct CivWriterAdapter {
    /// per-user 存储多路复用器——`civ_for(owner)` 即 API 读侧读取的那个档。
    per_user: std::sync::Arc<service::per_user::PerUserStore>,
    /// 本适配器所属会话的**归属用户**（创建会话时由鉴权层解出的 uid）。
    owner: String,
    /// P1-4 (audit-fix): 写失败计数——暴露到 /readyz（>5 次 → 503）。
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
        // D-109：写进**该 owner 的可见档**——与 API 读侧（`per_user.civ_for(uid)`）
        // 同一个文件；store 构造失败与 append 失败都计入 failures 并留痕。
        let store = match self.per_user.civ_for(&self.owner) {
            Ok(s) => s,
            Err(e) => {
                self.failures
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                tracing::warn!(owner = %self.owner, "civ auto-write: per-user store 不可用: {e}");
                return;
            }
        };
        if let Err(e) = store.append(entry) {
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

    // 1. OpenAI (always registered)
    // D-94：原注释称 "primary"——**不实**：ProviderRegistry 的 default 语义已删，
    // 谁是"主"由客户端在 `SessionCreate.provider` 里显式指定（必填字段）。
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
    registry.register_with_key("openai", &openai_model, openai);
    registry.register_alias("openai", &format!("openai:{openai_model}"));

    // 2. Ollama (optional — skip with warn if not configured)
    if let Some(base_url) = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
    {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qiyuan-8b:latest".into());
        let ollama = Arc::new(llm_local::OllamaProvider::new(
            "ollama",
            model,
            Some(base_url),
        ));
        registry.register(ollama);
        info!("ollama provider registered");
    } else {
        tracing::warn!("OLLAMA_BASE_URL not set — ollama provider skipped");
    }

    // 3. vLLM (optional — skip with warn if not configured)
    if let Some(base_url) = std::env::var("VLLM_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
    {
        let model = std::env::var("VLLM_MODEL").unwrap_or_else(|_| "mistral-7b".into());
        let api_key = std::env::var("VLLM_API_KEY").ok();
        let vllm = Arc::new(llm_local::VllmProvider::new(
            "vllm",
            model,
            Some(base_url),
            api_key,
        ));
        registry.register(vllm);
        info!("vllm provider registered");
    } else {
        tracing::warn!("VLLM_BASE_URL not set — vllm provider skipped");
    }

    // 4. Hunyuan (optional — skip with warn if not configured)
    // D-96：`filter(!is_empty)` 收口空串——`HUNYUAN_API_KEY=""` 此前会被当作"已配置"
    // 而注册一个空 key provider（同 agnes/zhipu/… 的病灶）。
    if let Some(api_key) = std::env::var("HUNYUAN_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
        let model = std::env::var("HUNYUAN_MODEL").unwrap_or_else(|_| "hunyuan-pro".into());
        let base_url = std::env::var("HUNYUAN_BASE_URL").ok();
        let hunyuan = Arc::new(llm_cn::HunyuanProvider::new(
            "hunyuan", model, base_url, api_key,
        ));
        registry.register(hunyuan);
        info!("hunyuan provider registered");
    } else {
        tracing::warn!("HUNYUAN_API_KEY not set — hunyuan provider skipped");
    }

    // 5. Doubao (字节豆包) — OpenAI-compatible, multi-model rotation
    if let Some(api_key) = std::env::var("DOUBAO_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
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
            registry.register(p);
        }
        info!(
            "doubao providers registered ({} models: {:?})",
            models.len(),
            models
        );
    } else {
        tracing::warn!("DOUBAO_API_KEY not set — doubao provider skipped");
    }

    // D-96（2026-10-02, traecode）：以下 provider 原以 `unwrap_or_default()` **无条件注册**，
    // 未配 key 时注册的是一个空 key provider——它会出现在 `/api/v1/models`（客户端据此
    // 选中），却任何调用都 401；若开了 FALLBACK_CHAIN 还会被链选中间途失败。
    // "能选但不能用"是最坏的一种可用性缺陷。现统一收紧为**有非空 key 才注册**，
    // 与 ollama/vllm/hunyuan/doubao 的既有 gating 同口径（OpenAI 更严：无 key 拒启）。
    //
    // 6. Agnes AI — free unlimited, OpenAI-compatible
    if let Some(api_key) = std::env::var("AGNES_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
        // Y1-1: v22 硬编码真实 key 清零——key 只从 env 读，无 fallback 写死（密钥入库即泄露）
        let model = std::env::var("AGNES_MODEL").unwrap_or_else(|_| "agnes-2.5-flash".into());
        let base_url =
            std::env::var("AGNES_BASE_URL").unwrap_or_else(|_| "https://api.agnes-ai.cn/v1".into());
        let agnes = Arc::new(llm_openai::OpenAiProvider::new(
            "agnes",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(agnes);
        info!("agnes provider registered (model={})", model);
    } else {
        tracing::warn!("AGNES_API_KEY not set — agnes provider skipped");
    }

    // 7. ZhiPu (智谱 GLM) — OpenAI-compatible, large token quota
    if let Some(api_key) = std::env::var("ZHIPU_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
        let model = std::env::var("ZHIPU_MODEL").unwrap_or_else(|_| "glm-4.5-air".into());
        let base_url = std::env::var("ZHIPU_BASE_URL")
            .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4".into());
        let zhipu = Arc::new(llm_openai::OpenAiProvider::new(
            "zhipu",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(zhipu);
        info!("zhipu provider registered (model={})", model);
    } else {
        tracing::warn!("ZHIPU_API_KEY not set — zhipu provider skipped");
    }

    // 8. Google Gemini — OpenAI-compatible
    if let Some(api_key) = std::env::var("GEMINI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
        let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.6-flash".into());
        let base_url = std::env::var("GEMINI_BASE_URL")
            .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/openai".into());
        let gemini = Arc::new(llm_openai::OpenAiProvider::new(
            "gemini",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(gemini);
        info!("gemini provider registered (model={})", model);
    } else {
        tracing::warn!("GEMINI_API_KEY not set — gemini provider skipped");
    }

    // 9. DeepSeek (官方直达) — OpenAI-compatible
    if let Some(api_key) = std::env::var("DEEPSEEK_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
        let model = std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".into());
        let base_url = std::env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
        let deepseek = Arc::new(llm_openai::OpenAiProvider::new(
            "deepseek",
            &model,
            Some(base_url),
            &api_key,
        ));
        registry.register(deepseek);
        info!("deepseek provider registered (model={})", model);
    } else {
        tracing::warn!("DEEPSEEK_API_KEY not set — deepseek provider skipped");
    }

    // 10. v15 S1b 天花板对照 — 同通道、同网关、只换大模型。
    //
    // 原计划用 gemini 做对照，但 v15 实测 gemini 3.x 在多轮 function calling 下
    // 要求回传 thought_signature，OpenAI 兼容层未透传 -> 第 2 步起 HTTP 400
    // （见 forge-report-v15.md「gemini 阻断」）。换通道会同时改变网关行为，
    // 无法把"失败"归因到模型强度上，因此改为在**已验证健康的同一通道**内
    // 只替换模型规模：flash -> pro / air -> 4.7。这样 S1b 的唯一自变量就是
    // 模型能力，正是天花板对照要测的东西。
    if let Some(api_key) = std::env::var("DEEPSEEK_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
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
        registry.register(pro);
        info!("deepseek-pro ceiling provider registered (model={})", model);
    }

    if let Some(zk) = std::env::var("ZHIPU_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
        let zmodel = std::env::var("ZHIPU_MAX_MODEL").unwrap_or_else(|_| "glm-4.7".into());
        let zbase = std::env::var("ZHIPU_BASE_URL")
            .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4".into());
        let zmax = Arc::new(llm_openai::OpenAiProvider::new(
            "zhipu-max",
            &zmodel,
            Some(zbase),
            &zk,
        ));
        registry.register(zmax);
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
                registry.register(Arc::new(p));
                info!(
                    "replay provider registered ({} recorded sessions from {dir})",
                    n
                );
            }
            Err(e) => tracing::warn!("REPLAY_DIR set but replay provider failed to load: {e}"),
        }
    }

    // ── P4/P5: FallbackChain (env-gated, registered as "fallback" provider) ──
    //
    // D-97（2026-10-02, traecode）：原实现用 `registry.list()`（**HashMap 键序**）直接
    // 组装链，而 `FallbackChain` 的语义恰恰是"**第一个 = 主通道**，其后按序降级"
    // （fallback.rs 模块头）。HashMap 迭代序每次进程/构建都可能不同 ⇒ 等于"谁是主
    // provider"在运行期**随机**（S9 的 [fallback] 通道切换投影也会跟着飘）。
    //
    // 现改为**显式/确定性**顺序，两种写法：
    //   · FALLBACK_CHAIN=<a,b,c>  —— 按**给定顺序**取已注册 provider，即 a 是主通道；
    //   · FALLBACK_CHAIN=1 / true —— 取全部已注册 provider，按名字**排序**（稳定但无语义，
    //     仅作兼容写法；要指定主通道请用上面的列表写法）。
    if let Some(spec) = std::env::var("FALLBACK_CHAIN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        let all_registered = spec == "1" || spec.eq_ignore_ascii_case("true");
        let chain_providers: Vec<(String, Arc<dyn llm_gateway::LlmProvider>)> = if all_registered {
            let mut names = registry.list();
            names.sort();
            names
                .into_iter()
                .filter_map(|name| registry.get(&name).ok().map(|p| (name, p)))
                .collect()
        } else {
            let mut out = Vec::new();
            for name in spec.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                match registry.get(name) {
                    Ok(p) => out.push((name.to_string(), p)),
                    Err(_) => tracing::warn!(
                        provider = name,
                        "FALLBACK_CHAIN 指定的 provider 未注册（缺 key/未启用），已跳过"
                    ),
                }
            }
            out
        };
        if chain_providers.is_empty() {
            tracing::warn!(spec = %spec, "FALLBACK_CHAIN 未能组装出任何后端——不注册 fallback provider");
        } else {
            let chain_names: Vec<String> = chain_providers.iter().map(|(n, _)| n.clone()).collect();
            let chain = Arc::new(FallbackChain::new(chain_providers));
            registry.register(chain);
            info!(
                "fallback chain registered as 'fallback' provider with {} backends (primary = {}): {:?}",
                chain_names.len(),
                chain_names.first().cloned().unwrap_or_default(),
                chain_names,
            );
        }
    } else {
        info!("FALLBACK_CHAIN not enabled (set FALLBACK_CHAIN=1, or FALLBACK_CHAIN=a,b,c for ordered)");
    }

    let registry = Arc::new(registry);

    // ── Build tool dispatcher ──
    // E3 v5.0: config-based registration — all tools enabled by default,
    // disable with CODEX_DISABLE_TOOLS=bash,glob (comma-separated).
    // D-149：过滤判定收敛到 `routes::enabled_builtin_tools`——**注册与 `GET /api/v1/tools`
    // 共用同一份**，杜绝"禁用已生效、API 却仍报它可用"（D-44 同族复发）。`key` 保持原有
    // 别名语义不变（如 `edit`→`write_file`）。
    let mut dispatcher = ToolDispatcher::new();
    for (_key, tool) in routes::enabled_builtin_tools(&routes::disabled_tools_from_env()) {
        dispatcher.register(tool);
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
        // RC13: config.toml 的 egress_allowlist 并入
        // D-144（2026-10-04, traecode）：路径解析改用 `agent_types::hearth_config_path()`
        // （**单一真相源**）——此前写死 `$HOME/.config/hearth/config.toml`（全平台），
        // 在 Windows 上与 CLI 的 `%APPDATA%\hearth\config.toml` 分叉 ⇒ 用户用
        // `hearth config set` 写入的白名单在 service 会话内**静默不生效**（RC13 自称
        // "与 CLI 同构/代码 ✓"实为声称≠实现）。
        let cfg_path = agent_types::hearth_config_path();
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

    // D-95（2026-10-02, traecode）：**删除 v10.4 "模型自动发现" 整块**。
    // 病灶（"半接线活特性" + "声称≠实现"）：该块在 ./providers.json 不存在时写一份
    // 默认清单，再把它读回来逐个 `tracing::info!("model auto-discovered")` 后**丢弃**——
    // 发现的模型既不注册进 ProviderRegistry、也不影响任何路由（provider 实际来自
    // 上方按 env 逐个 register 的那批）。日志在对外宣称"发现了模型"，实际什么都没发生。
    // 该结论早被 docs/global-panorama-v11.5.md:39 记为半接线债，此次收口。
    //
    // 为何是删除而非接线：默认清单里就有 `{"provider": "anthropic"}`，而本仓
    // **没有 anthropic provider 实现**——"发现即注册"按设计无法成立（还缺一层
    // provider-name → 构造器 的工厂）。给一个不会工作、且与 env 注册重复的机制
    // 补工厂属过度设计，故按纪律退役，并把旋钮 PROVIDERS_PATH 记进《已失效》节。

    // P5 A2: Wire JsonlMemoryStore for session persistence
    let memory_dir = std::env::var("MEMORY_DIR").unwrap_or_else(|_| "./memory".into());
    let memory_store = Arc::new(JsonlMemoryStore::new(&memory_dir));
    sessions.set_memory_store(memory_store);

    // P1-04（2026-10-01, traecode）：**删除 retriever 与 LSP 的启动接线**（顶层裁决「删除」）。
    //
    // 原实现（E1 v5.0 / P3）：启动时扫描 CODE_DIR 下所有 .rs 建 Tantivy 索引、
    // 视 LSP_ENABLED 起 rust-analyzer，然后 `sessions.set_retriever(..)` /
    // `set_lsp_bridge(..)`。但 P1-01 体检实证：这两个注入在 agent-core 内
    // **只被赋值、从未被读取** —— 即"花真实启动成本（索引 + 可能起一个
    // rust-analyzer 进程）换一个永不生效的接线"。
    //
    // 删除后：不再有 EMBED_MODEL / CODE_DIR / LSP_ENABLED 的启动期副作用；
    // 三个 env 变量随之失效（保留在文档里作为历史，不在此处虚挂）。
    // 相关能力若将来要恢复，需**先接线到消费点**再谈注入（单独立卡）。

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

    // v8.0: per-user store multiplexer（D-109：提前到此处构造——文明线写入器**工厂**
    // 需要它把每个会话的写入绑定到该用户的可见档）。
    let per_user = Arc::new(service::per_user::PerUserStore::new(std::path::Path::new(
        &memory_dir,
    )));

    // D-109: 注册**按 owner 构造**文明线写入器的工厂——使 agent-loop 的自动写入与 API
    // 读侧（`per_user.civ_for(uid)`）落到**同一个文件**。旧的**全局** `civilization.jsonl`
    // store 已随之停止构造（全仓无人读它，见 D-108）。
    // P1-4: 失败计数（连续失败暴露到 /readyz，>5 → 503）。
    let civ_write_failures: std::sync::Arc<std::sync::atomic::AtomicU64> =
        std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    {
        let pu = per_user.clone();
        let fails = civ_write_failures.clone();
        sessions.set_civ_writer_factory(Arc::new(move |owner: &str| {
            Arc::new(CivWriterAdapter {
                per_user: pu.clone(),
                owner: owner.to_string(),
                failures: fails.clone(),
            }) as Arc<dyn agent_core::CivWriter>
        }));
    }

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

    // v10.0 10A: instance identity（**现役**：经 `GET /api/v1/resources` 与
    // `hearth whoami` 对外暴露，见 routes.rs 的 `instance_id` 字段与 CLI）。
    let instance_id = format!("codex-inst-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    // D-130（2026-10-02, traecode）：此处原有一行
    // `let _ = std::fs::write("/tmp/codex.pid", process::id())`，注释称
    // "Write PID file for Observer health checks"——**该消费者不存在**：
    // 全仓检索（含 observer crate、CLI、脚本）**无任何读取方**；运维面的 pid 文件
    // 由 `run-hearth.sh` 自己维护（`.hearth-service.pid`，另一个文件）。
    // 且硬编码 `/tmp` 在 Windows 上不存在 ⇒ 这行**永远静默失败**（错误还被 `let _` 吞掉）。
    // 属"只写不读 + 不实注释"的死物，故删除；进程号仍由下一行的启动日志如实打印。
    tracing::info!(
        "[instance: {instance_id}] mounted, pid={}",
        std::process::id()
    );

    // D-108/D-109：文明线的**可见面**是 per-user store（`PerUserStore`，见上方工厂）。
    // 全局 `civilization.jsonl` store 已停止构造——它既无人读（D-108），也不再有写入方
    // （D-109 已把 CivWriterAdapter 改为按 owner 写 per-user 档）。

    // D-138（2026-10-04, traecode）：此处原有「6D: work line store」——一个**全局**
    // store（`MEMORY_DIR/workline.jsonl`）——及其「v10.4: WorkLine 60s background
    // scheduler」。两者一并**退役**，理由与 D-108/D-109 退役全局文明线同源：
    //   - 该全局 store **无任何读取方**：HTTP `/api/v1/workline*` 全部读 per-user 档
    //     （`per_user.workline_for(uid)`，见 `routes.rs`）；`AppState` 早在 D-108 已
    //     删除 `workline_store` 字段。它唯一的使用者就是下面这个调度器自己 ⇒ "只写不读"。
    //   - 调度器对**所有** pending 节点每 60s `progress + 1.0`、status 传 `None`
    //     （⇒ 状态永不流转、进度无界增长：1.0/分 → 1440/天），且每次 `update` 重写整份
    //     jsonl；其注释 "Advance any stale pending nodes" 承诺"仅 stale"，代码却无任何
    //     时效判定 ⇒ "声称≠实现"。
    //   - 语义上"无人做事却每 60s 涨进度"本身就是**谎报进度**，不是真功能。
    // work line 的可见面仍由 per-user store 提供（`per_user.rs` 的
    // `WorkLineStore::new(&dir, "workline.jsonl")`，落在 `<uid>/` 下），不受影响。

    // v7.0: telemetry collector
    let telemetry = Arc::new(routes::TelemetryCollector::default());

    // v8.0: per-user store multiplexer —— 已在上方（D-109）提前构造并同时供
    // 文明线写入器工厂使用；此处不再重复构造。

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
    //
    // D-169（2026-10-05, traecode）：**接通多租户的配置面**。此前 `UserStore::new(&[])` 恒为空
    // 且只 `register(api_key,"default")` ⇒ 全仓只会产生 `"default"` 一个 uid，per-user 档
    // （`MEMORY_DIR/<uid>/civ.jsonl`）恒指向同一份——D-108/D-109 以"跨租户泄露"为由拒绝合并
    // 全局档的那条隔离，**运行期并不存在**。现支持 `HEARTH_USERS="key1=alice,key2=bob"`
    // （**未设 ⇒ 行为与今完全相同**）；鉴权侧 `require_api_key` 亦改为"命中任一已注册 key 即通过"。
    let user_store = Arc::new(service::user::UserStore::new(&[]));
    for (key, user_id) in parse_users_env(std::env::var("HEARTH_USERS").ok().as_deref()) {
        user_store.register(key, user_id);
    }
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
        telemetry,
        // D-111②：原此处注入 `observer: Arc::new(observer::Observer::new())` —— 该字段
        // 零读取方（真实的第三权接线是上方 `sessions.set_observer(...)`），已删。
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

            // v20.0 S3: experience store maintenance —— 每小时执行**剪枝**
            // （effectiveness < 0.3 且超过 90 天者移除），防只增文件无限膨胀。
            //
            // D-107（2026-10-02, traecode）：原同处还有 `upgrade_core()`（"核心经验晋级"）
            // 并把返回**条数**打进日志。但该查询的过滤条件依赖 `reference_count`，
            // 而该字段**无任何写入方**（唯一自增点 `search()` 已随 D-47 删除）⇒ 恒返回空、
            // 日志里的 `core_candidates` 恒 0 —— 属"给死接口打日志"。已随接口一并删除；
            // 经验库现定位为**只写审计档**（复用需另行设计，见 `experience` 模块头与 D-100 裁决）。
            let pruned = observer_experience.prune(0.3, 90).await;
            if pruned > 0 {
                tracing::info!(pruned, "experience maintenance (v20 observer)");
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
        // D-92（2026-10-02, traecode）：补挂工具注册表**读**端点。
        // 病灶：handler `routes::registry_list` 早已实现、`tool_runtime::registry`
        // 契约文档也把它与 `search`/`install` 并列为三件套，但**从未挂到 Router** ⇒
        // 按文档调用必 404（"声称≠实现"在 API 面的投射，D-44/D-90 同族）。
        .route("/api/v1/tool-registry", get(routes::registry_list))
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
        // D-158：路由级 fallback 也走统一错误形状（axum 默认：404/405 均空体、无 JSON）。
        // 注：`method_not_allowed_fallback` 只作用于**此前已注册**的路由，故须置于所有
        // `.route(...)` 之后、`.layer(...)` 之前。
        .fallback(routes::not_found)
        .method_not_allowed_fallback(routes::method_not_allowed)
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
    // P2-1 (audit-fix): 注入 ConnectInfo<SocketAddr>——限流中间件按 per-IP 维度计数。
    // D-165（2026-10-05, traecode）：改走**优雅停机**——收到 SIGINT/SIGTERM 后停止接受新连接、
    // 排空在途请求再退出（此前裸 `axum::serve(..).await` 会在 SIGTERM 时被**立即终止**，
    // 硬切断在途请求与 SSE 流）。排空的墙钟上界交给编排方（terminationGracePeriodSeconds）。
    service::serve::serve_with_shutdown(app, listener, service::serve::shutdown_signal()).await?;

    Ok(())
}

/// D-169（2026-10-05, traecode）：解析 `HEARTH_USERS`——`"k1=alice,k2=bob"`
/// （逗号分隔；两侧裁剪；**忽略**空项与无 `=` 的项）。
///
/// 纯函数（不读进程环境，便于单测）。键或值任一为空即丢弃该对——避免注册出"空 key"
/// 这种只能靠空 Bearer 命中的畸形条目。
fn parse_users_env(raw: Option<&str>) -> Vec<(String, String)> {
    raw.unwrap_or("")
        .split(',')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            let (k, v) = (k.trim(), v.trim());
            if k.is_empty() || v.is_empty() {
                None
            } else {
                Some((k.to_string(), v.to_string()))
            }
        })
        .collect()
}

#[cfg(test)]
mod parse_users_env_tests {
    use super::parse_users_env;

    #[test]
    fn parses_pairs_and_skips_garbage() {
        assert!(parse_users_env(None).is_empty());
        assert!(parse_users_env(Some("")).is_empty());
        assert_eq!(
            parse_users_env(Some("k1=alice, k2=bob")),
            vec![
                ("k1".to_string(), "alice".to_string()),
                ("k2".to_string(), "bob".to_string())
            ]
        );
        // 空项 / 无 `=` / 空键 / 空值 一律丢弃
        assert_eq!(
            parse_users_env(Some(",broken,k3=carol,,=novalue,k4=")),
            vec![("k3".to_string(), "carol".to_string())]
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-109 行为锁：文明线**自动写入**必须落到**该 owner 的可见档**
    /// （`MEMORY_DIR/<owner>/civ.jsonl`）——即 API/CLI 读侧 `per_user.civ_for(uid)`
    /// 读取的同一个文件。
    ///
    /// 红侧（修复前）：适配器写的是**全局**档（`civilization.jsonl`）⇒ `alice/civ.jsonl`
    /// 不会出现 ⇒ 本测失败。
    #[test]
    fn civ_adapter_writes_to_owner_per_user_store() {
        let tmp = tempfile::tempdir().unwrap();
        let per_user = std::sync::Arc::new(service::per_user::PerUserStore::new(tmp.path()));
        let failures = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let writer = CivWriterAdapter {
            per_user: per_user.clone(),
            owner: "alice".to_string(),
            failures: failures.clone(),
        };

        agent_core::CivWriter::append_civ(
            &writer,
            "milestone",
            "交付完成",
            "sess-1",
            vec!["auto".to_string()],
        );

        assert_eq!(
            failures.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "写入不应失败"
        );
        let file = tmp.path().join("alice").join("civ.jsonl");
        assert!(
            file.exists(),
            "写入必须落在 owner 的 per-user 档（读接口读的就是它）: {}",
            file.display()
        );
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("交付完成"), "条目内容应已落盘: {text}");
        // 跨租户隔离：不得写到别的用户档。
        assert!(
            !tmp.path().join("bob").join("civ.jsonl").exists(),
            "不得写到他人档（写侧必须按 owner 隔离）"
        );
        // 读回路径与 API 一致：`civ_for(owner)` 能看到刚写入的条目。
        let visible = per_user.civ_for("alice").unwrap();
        assert!(
            visible
                .recent(10)
                .iter()
                .any(|e| e.content.contains("交付完成")),
            "经 per_user.civ_for(owner) 读回应能看到该条目"
        );
    }
}
