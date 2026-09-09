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

    /// Reset all counters to zero.
    pub fn reset(&mut self) {
        self.entries.clear();
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
}
