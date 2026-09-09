//! 天赋基因调度器（v0.2.2 · docs/hearth-meta-capability-genes-final.md 唯一权威件）
//!
//! 核心论点：天赋 = 先天电路（G0/G1）+ 后天内容（G2/G3）+ 激活条件（情境门控）。
//! hearth 的"不一样" = 可切换天赋组合（认知风格调度），不是"拥有全部天赋"——
//! 全开会预算爆炸 + 杀死创造力（创作要"流"，不要"监控"）。
//!
//! 落地：
//! - 核心电路 6 项（T1/T3/T4/T7/T10/C9）：fail-closed 常驻，wiring 断言（§3）
//! - 认知风格 13 项：G3 偏置注入（prompt），由本调度器按任务激活/抑制（§4/§5）
//! - 统计验证：风格激活时 emit 日志（"该激活时激活了没"，非二进制 fail）
//!
//! 铁律：调度器只生成"偏置文本 + 激活日志"，不做控制流（零内核改动——
//! 激活=提升 prompt 权重，抑制=降权非硬阻断，避免创作被监控误伤）。

/// 认知风格 ID（§5.1 预定义，可扩展）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Sceptic,  // 怀疑论者
    Engineer, // 工程师
    Scholar,  // 学者
    Surgeon,  // 外科医生
    Creator,  // 创作者
    Decider,  // 决策者
    Default,  // 未匹配（保守偏置）
}

/// 认知风格定义：激活天赋 / 抑制天赋 / 说明（§5.1）
pub struct TalentStyle {
    pub id: Style,
    pub name: &'static str,
    pub activate: &'static [&'static str],
    pub suppress: &'static [&'static str],
    pub desc: &'static str,
}

/// 天赋短名（注入文本用）
pub const T1: &str = "认知谦逊";
pub const T2: &str = "求真驱力";
pub const T3: &str = "递归自检";
pub const T4: &str = "体感";
pub const T5: &str = "多透镜";
pub const T6: &str = "事前验尸";
pub const T7: &str = "减速";
pub const T8: &str = "证伪耐受";
pub const T9: &str = "抽象迁移";
pub const T10: &str = "目标锚定";
pub const C1: &str = "好奇探索";
pub const C2: &str = "困惑识别";
pub const C3: &str = "置信校准";
pub const C4: &str = "遗忘更新";
pub const C5: &str = "基础比率";
pub const C6: &str = "EV量化";
pub const C7: &str = "可逆性";
pub const C9: &str = "价值澄清";
pub const C10: &str = "沟通校准";

/// 核心电路 6 项（§3）——任何任务下常驻（可降权不可关）
pub const CORE_CIRCUIT: &[&str] = &[T1, T3, T4, T7, T10, C9];

/// §5.1 风格表
pub fn styles() -> &'static [TalentStyle] {
    &[
        TalentStyle {
            id: Style::Sceptic,
            name: "怀疑论者",
            activate: &[T1, T8, C2],
            suppress: &[],
            desc: "验证外部事实 / 审查——默认怀疑，先核验再采信",
        },
        TalentStyle {
            id: Style::Engineer,
            name: "工程师",
            activate: &[T7, C7, T10],
            suppress: &[C1],
            desc: "写代码 / 改系统——减速、可逆、锚定目标，防好奇跑偏",
        },
        TalentStyle {
            id: Style::Scholar,
            name: "学者",
            activate: &[C1, T9, C3],
            suppress: &[],
            desc: "调研 / 学习新领域——主动探索、抽象迁移、校准置信",
        },
        TalentStyle {
            id: Style::Surgeon,
            name: "外科医生",
            activate: &[T7, T10, T6, C7],
            suppress: &[],
            desc: "高风险不可逆操作——事前验尸 + 可逆性检查",
        },
        TalentStyle {
            id: Style::Creator,
            name: "创作者",
            activate: &[C1, T9, T2],
            suppress: &[T7, T6, C6],
            desc: "写作 / 生成——保持（流）状态，抑制自我监控化",
        },
        TalentStyle {
            id: Style::Decider,
            name: "决策者",
            activate: &[C3, C5, C6, T5],
            suppress: &[],
            desc: "数据分析 / 取舍——置信、基础比率、EV、多透镜",
        },
        TalentStyle {
            id: Style::Default,
            name: "通用",
            activate: &[],
            suppress: &[],
            desc: "未匹配——仅核心电路常驻偏置",
        },
    ]
}

/// 任务分类器（§5.2，关键词粗分——P1 骨架，后续可升 LLM route）。
/// 优先级：高风险 > 创作 > 代码 > 验证 > 数据 > 问答 > 学者默认。
pub fn classify(goal: &str) -> Style {
    let g = goal.to_lowercase();
    let g = g.as_str();
    // 高风险不可逆（外科医生优先——安全第一）
    if [
        "删除",
        "rm ",
        "覆盖",
        "发布",
        "上线",
        "不可逆",
        "清空",
        "drop",
        "delete",
    ]
    .iter()
    .any(|k| g.contains(k))
    {
        return Style::Surgeon;
    }
    // 创作
    if [
        "小说",
        "故事",
        "写一篇",
        "创作",
        "文章",
        "散文",
        "诗歌",
        "文案",
        "作文",
        "essay",
        "story",
        "novel",
    ]
    .iter()
    .any(|k| g.contains(k))
    {
        return Style::Creator;
    }
    // 验证/审查
    if [
        "验证", "检查", "审查", "核验", "确认", "复核", "测试", "verify", "review", "check",
        "audit",
    ]
    .iter()
    .any(|k| g.contains(k))
    {
        return Style::Sceptic;
    }
    // 写代码
    if [
        "写", "实现", "修复", "bug", "函数", "重构", "代码", "应用", "游戏", "模块", "接口",
        "rust", "python", "前端", "后端", "项目",
    ]
    .iter()
    .any(|k| g.contains(k))
    {
        return Style::Engineer;
    }
    // 数据分析/决策
    if [
        "分析", "数据", "统计", "对比", "决策", "评估", "报告", "指标", "趋势", "analyse",
        "analyze", "data", "report",
    ]
    .iter()
    .any(|k| g.contains(k))
    {
        return Style::Decider;
    }
    // 调研/学习（未命中上类的较长目标）
    if [
        "调研", "学习", "了解", "研究", "查", "what is", "how does", "介绍", "解释",
    ]
    .iter()
    .any(|k| g.contains(k))
    {
        return Style::Scholar;
    }
    // 纯问答（短目标）→ 通用（§5.3 纯问答几乎全抑制——通用风格激活集为空即抑制全部监控类）
    Style::Default
}

/// 取目标对应的风格定义
pub fn style_for(goal: &str) -> &'static TalentStyle {
    let id = classify(goal);
    styles().iter().find(|s| s.id == id).unwrap_or(&styles()[6])
}

/// 生成注入偏置段（build_messages 用——G3 软偏置，非控制流）
pub fn inject_text(goal: &str) -> String {
    let s = style_for(goal);
    let mut out = String::from("\n\n## Cognitive Style (天赋调度):\n");
    out.push_str(&format!("当前风格：**{}**——{}\n", s.name, s.desc));
    if !s.activate.is_empty() {
        out.push_str(&format!("激活天赋：{}\n", s.activate.join("、")));
    } else {
        out.push_str("激活天赋：（无——纯问答/通用，监控类天赋保持抑制）\n");
    }
    if !s.suppress.is_empty() {
        out.push_str(&format!(
            "抑制天赋：{}——降权非硬阻断（保护创作/执行的流状态）\n",
            s.suppress.join("、")
        ));
    }
    out.push_str(&format!(
        "核心电路常驻（不可关闭，仅可降权）：{}\n",
        CORE_CIRCUIT.join("、")
    ));
    out
}

/// 核心电路 wiring 断言（§3）——启动期检查各机制真实在位。
/// 返回未接线项列表（空 = 全接）。任一缺失 = 天赋电路断裂（wiring 断言语义）。
/// 可查项（真实可验证的机制）：T4 introspect 工具 / T7 budget 偏离字段 / T1+C9 planner gap。
pub fn core_circuit_wiring(
    dispatcher_tools: &[String],
    budget_has_deviation: bool,
    planner_gap_types: &[&str],
) -> Vec<String> {
    let mut missing = Vec::new();
    // T4 体感：introspect 工具必须注册（LLM 可查身体状态）
    if !dispatcher_tools.iter().any(|t| t == "introspect") {
        missing.push("T4 体感: introspect 工具未注册".to_string());
    }
    // T7 减速：Budget 必须带偏离阈值（50%/100% ask 的数据基础）
    if !budget_has_deviation {
        missing.push("T7 减速: Budget.deviation_warn_at 未启用".to_string());
    }
    // T1 认知谦逊：planner 必须产出 unverified_claim gap
    if !planner_gap_types.contains(&"unverified_claim") {
        missing.push("T1 认知谦逊: unverified_claim gap 未接线".to_string());
    }
    // C9 价值澄清：planner 必须产出 ambiguous_option gap（意图模糊必反问）
    if !planner_gap_types.contains(&"ambiguous_option") {
        missing.push("C9 价值澄清: ambiguous_option gap 未接线".to_string());
    }
    // T3 四阶段 / T10 目标锚定：架构拓扑级（相位钩子 + goal 判定），无法启动期
    // 单点检查——由 loop 侧 do_plan/do_act/do_observe/do_reflect 接线 + wiring-v13.toml 断言。
    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 分类器：创作/代码/验证/数据/高风险/问答 各归其位（§5.3 门控表）
    #[test]
    fn test_classify_styles() {
        assert_eq!(classify("帮我写一篇关于冬天的小说"), Style::Creator);
        assert_eq!(classify("写一个贪吃蛇游戏"), Style::Engineer);
        assert_eq!(classify("验证这个 API 是否可用"), Style::Sceptic);
        assert_eq!(classify("分析这份销售数据"), Style::Decider);
        assert_eq!(classify("删除 /tmp/old 目录"), Style::Surgeon);
        assert_eq!(classify("1+1等于几"), Style::Default);
    }

    /// 创作者风格：抑制 T7/T6/C6（创作要"流"）——§5.3 门控
    #[test]
    fn test_creator_suppresses_monitoring() {
        let s = style_for("写一篇小说");
        assert!(s.activate.contains(&C1), "创作者激活好奇");
        assert!(s.suppress.contains(&T7), "创作者抑制减速（监控）");
        assert!(s.suppress.contains(&C6), "创作者抑制 EV 量化（监控）");
    }

    /// 注入文本含核心电路常驻说明
    #[test]
    fn test_inject_mentions_core_circuit() {
        let t = inject_text("写一篇小说");
        assert!(t.contains("创作者"), "注入含风格名");
        assert!(t.contains("核心电路常驻"), "注入含核心电路说明");
        assert!(t.contains("抑制天赋"), "注入含抑制说明");
    }

    /// wiring 断言：缺 introspect 工具 → T4 缺失；全在位 → 空
    #[test]
    fn test_core_wiring() {
        let missing = core_circuit_wiring(&[], false, &[]);
        assert!(!missing.is_empty(), "全缺必须报缺失");
        assert!(
            missing.iter().any(|m| m.contains("T4")),
            "缺 introspect 报 T4"
        );
        let ok = core_circuit_wiring(
            &["bash".to_string(), "introspect".to_string()],
            true,
            &["unverified_claim", "ambiguous_option"],
        );
        assert!(ok.is_empty(), "全在位应为空: {ok:?}");
    }
}
