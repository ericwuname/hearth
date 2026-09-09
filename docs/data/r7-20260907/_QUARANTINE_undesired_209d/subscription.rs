//! 订阅过滤（§7.2 requested_granularity）+ 动态订阅（M-10）。
//! 消费者按 manifest.subscribes 过滤 EVENT_LOG；粒度决定 delta 取法。

use crate::event::{Event, EventType};
use crate::manifest::Subscription;

/// 订阅过滤：事件是否被该订阅集接收。
pub fn matches(event: &Event, subs: &[Subscription]) -> bool {
    subs.iter().any(|s| {
        s.event_type == event.etype.as_str_loose()
            || s.event_type == format!("{}:{}", event.role, event.etype.as_str_loose())
    })
}

/// 粒度满足：订阅方期望粒度 ≥ 生产方提供粒度（full > file > summary）。
pub fn granularity_satisfied(requested: &str, provided: &str) -> bool {
    fn rank(g: &str) -> u8 {
        match g {
            "full" => 2,
            "file" => 1,
            _ => 0,
        }
    }
    rank(provided) <= rank(requested)
}

impl EventType {
    /// 宽松事件类型字符串（serde snake_case 名，供订阅匹配）。
    fn as_str_loose(&self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventType, SCHEMA_VERSION};
    use crate::manifest::Subscription;

    fn evt(role: &str, etype: EventType) -> Event {
        Event {
            schema_version: SCHEMA_VERSION.into(),
            seq: 1,
            event_id: String::new(),
            actor: "w1".into(),
            role: role.into(),
            etype,
            target_ref: None,
            content_hash: None,
            hands_off_to: None,
            in_reply_to: None,
            await_timeout: None,
            summary: None,
            status: None,
            prev_hash: None,
            provided_granularity: Some("file".into()),
        }
    }

    /// §7.2 正面：订阅 "施工:done" 匹配 role=施工 + type=task_done 的事件。
    #[test]
    fn test_subscription_match() {
        let subs = vec![Subscription {
            event_type: "施工:task_done".into(),
            requested_granularity: "file".into(),
        }];
        let e = evt("施工", EventType::TaskDone);
        assert!(matches(&e, &subs), "订阅应命中");
        let e2 = evt("审计", EventType::TaskDone);
        assert!(!matches(&e2, &subs), "不同角色不命中");
    }

    /// §7.2 正面：requested=file 满足 provided=file/summary；requested=summary 不满足 provided=file。
    #[test]
    fn test_granularity() {
        assert!(granularity_satisfied("file", "file"));
        assert!(granularity_satisfied("file", "summary"));
        assert!(
            !granularity_satisfied("summary", "file"),
            "期望 summary 收到 file 粒度不足"
        );
        assert!(granularity_satisfied("full", "file"));
    }
}
