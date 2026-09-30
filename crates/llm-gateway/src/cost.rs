//! CostMeter: session-scoped token usage accumulator.
//!
//! Tracks cumulative token usage across all providers during a session.
//! Used for budget tracking and cost accounting.

use crate::types::Usage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single cost entry for a specific (provider, model) pair.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostEntry {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    /// Number of calls made to this (provider, model).
    pub calls: u64,
    /// P1-1 (v0.2.4): 累计缓存命中 token（仅支持该字段的通道上报——Ollama 等为 0）。
    pub cache_hit_tokens: u64,
    /// P1-1: 累计缓存未命中 token。
    pub cache_miss_tokens: u64,
    /// P1-1: 上报过缓存字段的调用次数（区分"通道不支持"与"真零命中"）。
    pub cache_reported_calls: u64,
}

impl CostEntry {
    /// Record a single usage event.
    pub fn record(&mut self, usage: &Usage) {
        self.prompt_tokens += usage.prompt_tokens as u64;
        self.completion_tokens += usage.completion_tokens as u64;
        self.total_tokens += usage.total_tokens as u64;
        self.calls += 1;
        if let Some(hit) = usage.prompt_cache_hit_tokens {
            self.cache_hit_tokens += hit as u64;
            self.cache_reported_calls += 1;
        }
        if let Some(miss) = usage.prompt_cache_miss_tokens {
            self.cache_miss_tokens += miss as u64;
        }
    }
}

/// Session-scoped cost meter that accumulates usage per (provider, model).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostMeter {
    /// Key: "provider_name/model_name"
    entries: HashMap<String, CostEntry>,
}

impl CostMeter {
    /// Create a new empty cost meter.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Record a usage event for a given provider and model.
    pub fn record(&mut self, provider: &str, model: &str, usage: &Usage) {
        let key = format!("{provider}/{model}");
        self.entries.entry(key).or_default().record(usage);
    }

    /// Get the cost entry for a specific provider/model pair.
    pub fn get(&self, provider: &str, model: &str) -> Option<&CostEntry> {
        let key = format!("{provider}/{model}");
        self.entries.get(&key)
    }

    /// Total prompt tokens across all providers.
    pub fn total_prompt_tokens(&self) -> u64 {
        self.entries.values().map(|e| e.prompt_tokens).sum()
    }

    /// Total completion tokens across all providers.
    pub fn total_completion_tokens(&self) -> u64 {
        self.entries.values().map(|e| e.completion_tokens).sum()
    }

    /// Total tokens across all providers.
    pub fn total_tokens(&self) -> u64 {
        self.entries.values().map(|e| e.total_tokens).sum()
    }

    /// Total number of calls across all providers.
    pub fn total_calls(&self) -> u64 {
        self.entries.values().map(|e| e.calls).sum()
    }

    /// Number of distinct (provider, model) pairs recorded.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over all cost entries.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CostEntry)> {
        self.entries.iter()
    }

    /// D-46: USD 总量——按 [`PriceTable`] 对本 meter 内**每一条** (provider, model)
    /// 条目计价并求和。
    ///
    /// **None 语义（诚实，不把未知当 0）**：只要有任意一条条目在价表中查不到价，
    /// 就返回 `None`（"算不出来"），而不是把未知模型的费用按 0 累加、得出一个
    /// 看似精确实则偏低的数字。空 meter → `Some(0.0)`（无未知条目）。
    pub fn total_usd(&self, prices: &PriceTable) -> Option<f64> {
        let mut sum = 0.0;
        for (key, entry) in &self.entries {
            let (provider, model) = key.split_once('/')?;
            let usd = prices.usd(
                provider,
                model,
                entry.prompt_tokens,
                entry.completion_tokens,
            )?;
            sum += usd;
        }
        Some(sum)
    }

    /// Reset all counters to zero.
    pub fn reset(&mut self) {
        self.entries.clear();
    }
}

/// D-46: 单一 (provider, model) 的单价，单位 **USD / 1M tokens**。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ModelPrice {
    /// 输入（prompt）单价（USD / 1M tokens）。
    pub input_per_1m_usd: f64,
    /// 输出（completion）单价（USD / 1M tokens）。
    pub output_per_1m_usd: f64,
}

/// D-46: 价格表——(provider, model) → [`ModelPrice`]，外加别名解析。
///
/// # 诚实性（关键设计）
/// [`lookup`](Self::lookup) 对**未知模型返回 `None`**，而不是猜一个价。
/// 宁可"算不出来"也不编造——调用方据此显式报"成本不可用"，
/// 而不是拿一个虚假的 0 或臆测价格喂给预算守卫。
///
/// # 内置价来源（务必随官方调价更新 / 用配置覆盖）
/// - 抓取日期：2026-10-01
/// - 来源：<https://api-docs.deepseek.com/quick_start/pricing/>
/// - **价格会变**：请用环境变量 `HEARTH_PRICE_TABLE`（内联 JSON）或
///   `HEARTH_PRICE_FILE`（文件路径）覆盖，勿依赖此处快照。
/// - 官方有**高峰 / 空闲两档**价；为避免非确定性（同一次 run 因时段不同算出
///   不同费用），内置表**默认取高峰价（保守上限）**。可在 config 中覆盖为空闲价。
/// - [`Usage`] 只有 prompt / completion / total，**没有 cache-hit 细分** →
///   此处一律按 **cache-miss（标准）价**计（保守：真实费用只会更低，不会更高）。
#[derive(Debug, Clone, Default)]
pub struct PriceTable {
    /// key: (provider, model) → 单价。
    prices: HashMap<(String, String), ModelPrice>,
    /// key: (provider, model) → 规范 (provider, model)（别名解析，如
    /// `deepseek-v4-flash` → `deepseek-flash`）。
    aliases: HashMap<(String, String), (String, String)>,
}

/// `from_json_str` 的 JSON 结构：键为 `"provider/model"`。
/// （本节未在任务中给定格式——此处定义并文档化，与 `from_env` 一致。）
#[derive(Debug, Deserialize, Default)]
struct PriceTableJson {
    /// `"provider/model"` → 单价。
    #[serde(default)]
    prices: HashMap<String, ModelPrice>,
    /// `"provider/model"` → 规范 `"provider/model"`（同 provider 内别名）。
    #[serde(default)]
    aliases: HashMap<String, String>,
}

impl PriceTable {
    /// 内置默认价表（DeepSeek 官方价快照，见类型级文档的日期 / 来源 / 注意事项）。
    ///
    /// 别名：`deepseek-v4-flash` / `deepseek-v4-flash-vision-exp` 官方声明
    /// "由 V4.1-Flash 提供服务并按 Flash 计费" → 映射到 `deepseek-flash` 的价格。
    /// `deepseek-reasoner` 无当前公开价 → **不放进内置表**（查表得 `None`，不猜）。
    pub fn builtin() -> Self {
        let mut prices = HashMap::new();
        // DeepSeek Flash（含 V4.1-Flash）：高峰价。
        prices.insert(
            ("deepseek".to_string(), "deepseek-flash".to_string()),
            ModelPrice {
                input_per_1m_usd: 0.30,
                output_per_1m_usd: 1.20,
            },
        );
        // DeepSeek V4 Pro：高峰价。
        prices.insert(
            ("deepseek".to_string(), "deepseek-v4-pro".to_string()),
            ModelPrice {
                input_per_1m_usd: 1.32,
                output_per_1m_usd: 3.96,
            },
        );
        let mut aliases = HashMap::new();
        for legacy in ["deepseek-v4-flash", "deepseek-v4-flash-vision-exp"] {
            aliases.insert(
                ("deepseek".to_string(), legacy.to_string()),
                ("deepseek".to_string(), "deepseek-flash".to_string()),
            );
        }
        Self { prices, aliases }
    }

    /// 查价：先直接命中，再走别名解析；**未知 → `None`**（不编造）。
    pub fn lookup(&self, provider: &str, model: &str) -> Option<ModelPrice> {
        let key = (provider.to_string(), model.to_string());
        if let Some(p) = self.prices.get(&key) {
            return Some(*p);
        }
        if let Some(canonical) = self.aliases.get(&key) {
            return self.prices.get(canonical).copied();
        }
        None
    }

    /// 按 token 用量计费；查不到价 → `None`（不把未知当 0）。
    pub fn usd(
        &self,
        provider: &str,
        model: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
    ) -> Option<f64> {
        let price = self.lookup(provider, model)?;
        Some(
            (prompt_tokens as f64 / 1_000_000.0) * price.input_per_1m_usd
                + (completion_tokens as f64 / 1_000_000.0) * price.output_per_1m_usd,
        )
    }

    /// 从 JSON 字符串构造，**合并到 [`builtin`](Self::builtin) 之上**。
    pub fn from_json_str(s: &str) -> anyhow::Result<Self> {
        let parsed: PriceTableJson = serde_json::from_str(s)?;
        let mut table = Self::builtin();
        for (key, price) in parsed.prices {
            let (provider, model) = key
                .split_once('/')
                .ok_or_else(|| anyhow::anyhow!("price key 必须为 \"provider/model\"：{key}"))?;
            table
                .prices
                .insert((provider.to_string(), model.to_string()), price);
        }
        for (key, canonical) in parsed.aliases {
            let (provider, model) = key
                .split_once('/')
                .ok_or_else(|| anyhow::anyhow!("alias key 必须为 \"provider/model\"：{key}"))?;
            let (cprov, cmodel) = canonical.split_once('/').ok_or_else(|| {
                anyhow::anyhow!("alias 目标必须为 \"provider/model\"：{canonical}")
            })?;
            table.aliases.insert(
                (provider.to_string(), model.to_string()),
                (cprov.to_string(), cmodel.to_string()),
            );
        }
        Ok(table)
    }

    /// 从环境构造：
    /// - `HEARTH_PRICE_TABLE`：**内联 JSON**（优先）；
    /// - `HEARTH_PRICE_FILE`：**文件路径**（读文件内容后按 JSON 解析）；
    /// - 都没设 → [`builtin`](Self::builtin)。
    ///
    /// 解析 / 读取失败 → 返回 `Err`（不 panic、不改动调用方状态）；**调用方
    /// 应 warn 后回退 `builtin()`**（内置价仍在，只是丢掉了覆盖）。
    pub fn from_env() -> anyhow::Result<Self> {
        if let Ok(inline) = std::env::var("HEARTH_PRICE_TABLE") {
            if !inline.trim().is_empty() {
                return Self::from_json_str(&inline);
            }
        }
        if let Ok(path) = std::env::var("HEARTH_PRICE_FILE") {
            if !path.trim().is_empty() {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| anyhow::anyhow!("读取 HEARTH_PRICE_FILE({path}) 失败：{e}"))?;
                return Self::from_json_str(&content);
            }
        }
        Ok(Self::builtin())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(prompt: u32, completion: u32) -> Usage {
        Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
        }
    }

    /// P1-1 (v0.2.4): 缓存字段累计——支持通道（DeepSeek）上报的 hit/miss 须累加；
    /// 负面：不上报缓存的通道（Ollama）cache 字段保持 0 且 cache_reported_calls=0
    /// （区分"不支持"与"零命中"，不误报）。
    #[test]
    fn test_cache_usage_accumulation() {
        let mut meter = CostMeter::new();
        // 支持缓存的通道：两次上报
        meter.record(
            "openai",
            "deepseek",
            &Usage {
                prompt_tokens: 1000,
                completion_tokens: 50,
                total_tokens: 1050,
                prompt_cache_hit_tokens: Some(800),
                prompt_cache_miss_tokens: Some(200),
            },
        );
        meter.record(
            "openai",
            "deepseek",
            &Usage {
                prompt_tokens: 2000,
                completion_tokens: 60,
                total_tokens: 2060,
                prompt_cache_hit_tokens: Some(1800),
                prompt_cache_miss_tokens: Some(200),
            },
        );
        // 不支持缓存的通道
        meter.record("ollama", "qwen", &usage(300, 40));

        let ds = meter.get("openai", "deepseek").unwrap();
        assert_eq!(ds.cache_hit_tokens, 2600, "命中累计 800+1800");
        assert_eq!(ds.cache_miss_tokens, 400, "未命中累计 200+200");
        assert_eq!(ds.cache_reported_calls, 2, "仅统计上报过缓存的调用");
        // 命中率 2600/3000 ≈ 86.7%
        let hit_rate =
            ds.cache_hit_tokens as f64 / (ds.cache_hit_tokens + ds.cache_miss_tokens) as f64;
        assert!(hit_rate > 0.86 && hit_rate < 0.87, "hit_rate={hit_rate}");

        let ollama = meter.get("ollama", "qwen").unwrap();
        assert_eq!(ollama.cache_hit_tokens, 0);
        assert_eq!(
            ollama.cache_reported_calls, 0,
            "不上报缓存的通道不得误报零命中"
        );
    }

    #[test]
    fn test_record_single() {
        let mut meter = CostMeter::new();
        meter.record("openai", "gpt-4o", &usage(100, 50));
        assert_eq!(meter.total_tokens(), 150);
        assert_eq!(meter.total_prompt_tokens(), 100);
        assert_eq!(meter.total_completion_tokens(), 50);
        assert_eq!(meter.total_calls(), 1);
    }

    #[test]
    fn test_record_multiple_same_provider() {
        let mut meter = CostMeter::new();
        meter.record("openai", "gpt-4o", &usage(100, 50));
        meter.record("openai", "gpt-4o", &usage(200, 100));
        assert_eq!(meter.total_tokens(), 450);
        assert_eq!(meter.total_calls(), 2);

        let entry = meter.get("openai", "gpt-4o").unwrap();
        assert_eq!(entry.calls, 2);
        assert_eq!(entry.prompt_tokens, 300);
        assert_eq!(entry.completion_tokens, 150);
    }

    #[test]
    fn test_record_multiple_providers() {
        let mut meter = CostMeter::new();
        meter.record("openai", "gpt-4o", &usage(100, 50));
        meter.record("ollama", "llama3.2", &usage(30, 20));
        meter.record("hunyuan", "hunyuan-pro", &usage(80, 40));

        assert_eq!(meter.total_tokens(), 320);
        assert_eq!(meter.total_calls(), 3);
        assert_eq!(meter.entry_count(), 3);

        let ollama = meter.get("ollama", "llama3.2").unwrap();
        assert_eq!(ollama.total_tokens, 50);
    }

    #[test]
    fn test_get_missing() {
        let meter = CostMeter::new();
        assert!(meter.get("nonexistent", "model").is_none());
    }

    #[test]
    fn test_reset() {
        let mut meter = CostMeter::new();
        meter.record("openai", "gpt-4o", &usage(100, 50));
        meter.reset();
        assert_eq!(meter.total_tokens(), 0);
        assert_eq!(meter.total_calls(), 0);
        assert_eq!(meter.entry_count(), 0);
    }

    #[test]
    fn test_iter() {
        let mut meter = CostMeter::new();
        meter.record("openai", "gpt-4o", &usage(10, 5));
        meter.record("ollama", "llama3.2", &usage(3, 2));

        let entries: Vec<_> = meter.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    // ── D-46：价格表 / USD 换算 ───────────────────────────────────────────

    /// 1) 未知模型必须返回 None（诚实——不编造价格）。
    #[test]
    fn unknown_model_yields_none() {
        let table = PriceTable::builtin();
        assert!(table.lookup("deepseek", "no-such-model").is_none());
        assert!(table.lookup("unknown-provider", "deepseek-flash").is_none());
        assert!(table
            .usd("deepseek", "deepseek-reasoner", 1_000_000, 1_000_000)
            .is_none());
    }

    /// 2) deepseek-flash：1M prompt + 1M completion = 0.30 + 1.20 = 1.50 USD。
    #[test]
    fn deepseek_flash_usd_matches_official_price() {
        let table = PriceTable::builtin();
        let usd = table
            .usd("deepseek", "deepseek-flash", 1_000_000, 1_000_000)
            .expect("flash 有价");
        assert!((usd - 1.5).abs() < 1e-9, "usd={usd}");
    }

    /// 3) 旧名 deepseek-v4-flash 与 deepseek-flash 得同一价格。
    #[test]
    fn legacy_deepseek_alias_maps_to_flash_price() {
        let table = PriceTable::builtin();
        let canonical = table.lookup("deepseek", "deepseek-flash").unwrap();
        let legacy = table.lookup("deepseek", "deepseek-v4-flash").unwrap();
        let vision = table
            .lookup("deepseek", "deepseek-v4-flash-vision-exp")
            .unwrap();
        assert_eq!(canonical, legacy);
        assert_eq!(canonical, vision);
        assert_eq!(
            table.usd("deepseek", "deepseek-v4-flash", 1_000_000, 1_000_000),
            table.usd("deepseek", "deepseek-flash", 1_000_000, 1_000_000)
        );
    }

    /// 4) meter 里混入一条未知 model → total_usd 返回 None（不把未知当 0 累加）。
    #[test]
    fn meter_total_usd_none_when_any_entry_unpriced() {
        let table = PriceTable::builtin();
        let mut meter = CostMeter::new();
        meter.record("deepseek", "deepseek-flash", &usage(1000, 500));
        // 单条已知 → 可算。
        assert!(meter.total_usd(&table).is_some());
        // 混入未知 → 整体不可算。
        meter.record("ollama", "qwen", &usage(300, 40));
        assert!(meter.total_usd(&table).is_none());
        // 空 meter → Some(0.0)（无未知条目）。
        assert_eq!(CostMeter::new().total_usd(&table), Some(0.0));
    }

    /// 5) from_json_str 覆盖内置价生效：flash 改成 1.0/2.0 → 1M+1M = 3.0。
    #[test]
    fn from_json_str_overrides_builtin_price() {
        let json = r#"{"prices":{"deepseek/deepseek-flash":{"input_per_1m_usd":1.0,"output_per_1m_usd":2.0}}}"#;
        let table = PriceTable::from_json_str(json).expect("JSON 合法");
        let usd = table
            .usd("deepseek", "deepseek-flash", 1_000_000, 1_000_000)
            .unwrap();
        assert!((usd - 3.0).abs() < 1e-9, "usd={usd}");
        // 未覆盖的内置项仍在（v4-pro 保留）。
        assert!(table.lookup("deepseek", "deepseek-v4-pro").is_some());
        // 非法 JSON → Err（不 panic）。
        assert!(PriceTable::from_json_str("{not json").is_err());
    }
}
