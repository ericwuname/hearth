//! D-210（2026-10-08, traecode）门禁：错误分类的状态码必须按**独立数字**匹配，不得裸子串命中。
//!
//! 病灶：`llm_gateway::types::classify_anyhow` 的兜底启发式用 `low.contains("500")` /
//! `contains("429")` / `contains("400")` 等**裸子串**匹配状态码——任何含这些数字的文本都会命中：
//! 端点 URL 的**端口**（`:5000`——Flask/多本地网关默认端口）、token 数（`4000`）、请求 id……
//! 由于 5xx 规则**先于** 4xx(Param) 规则，`"…400…:5000/…"` 会被判成 **5xx → Transient**：
//! **不该重试的 4xx 被重试**（最长 30 分钟重试窗口）；反向亦可能把 5xx 拉成 Param（不重试）。
//! 属"错误分类不一致"族（D-166/D-179/D-183）。

use llm_gateway::types::{classify_anyhow, ErrorClass};

#[test]
fn status_code_must_be_a_delimited_number() {
    // ① 4xx（参数错）但 URL 端口含 "500" ⇒ 必须 Param（不重试），不得因裸子串 "500" 判成 Transient。
    let e = anyhow::anyhow!(
        "Hunyuan error 400 Bad Request: invalid request to http://127.0.0.1:5000/chat/completions"
    );
    assert_eq!(
        classify_anyhow(&e),
        ErrorClass::Param,
        "端口 :5000 的 '500' 不得把 400 误判为 5xx/Transient：{e}"
    );

    // ② 同上但正文无 "invalid" 等词——仅靠状态码也必须判 Param。
    let e = anyhow::anyhow!(
        "Hunyuan error 400 Bad Request: model not found at http://127.0.0.1:5000/v1/models"
    );
    assert_eq!(
        classify_anyhow(&e),
        ErrorClass::Param,
        "4xx 必须 Param：{e}"
    );
}

/// 反向对照：真实状态码（独立数字）仍须正确分类——防把门禁"矫枉过正"。
#[test]
fn genuine_status_codes_still_classified() {
    for (msg, want) in [
        ("HTTP 500 Internal Server Error", ErrorClass::Transient),
        (
            "Hunyuan error 503 Service Unavailable",
            ErrorClass::Transient,
        ),
        ("HTTP 429 Too Many Requests", ErrorClass::Transient),
        ("Hunyuan error 401 Unauthorized", ErrorClass::Param),
        ("Hunyuan error 403 Forbidden", ErrorClass::Param),
        ("HTTP 400: bad request", ErrorClass::Param),
    ] {
        let e = anyhow::anyhow!("{msg}");
        assert_eq!(
            classify_anyhow(&e),
            want,
            "独立数字状态码必须正确分类：{msg}"
        );
    }
}
