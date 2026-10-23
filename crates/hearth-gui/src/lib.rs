//! hearth-gui：终端 IDE 的原生 GUI（M2-G）。
//!
//! **架构**：纯 Rust、单进程、无 WebView / 无 Node 工具链（eframe/egui 0.29）。
//! 复用 `hearth-tui` 的**事件→状态模型**（`TuiState` 单一实现），GUI 只换渲染层。
//! provider / dispatcher / 会话落盘仍由 `codex-cli` 构造后注入（避免反向依赖环）。
//!
//! 异步桥：App 持一个独立 tokio Runtime；agent 在 `rt.spawn` 里跑，事件经
//! unbounded channel 进入 UI 帧循环；轮次结束经另一条 channel 把 `AgentLoop`
//! 本体交还（历史不丢）。**不使用 `block_on`**（帧回调运行在宿主 runtime 上下文里）。

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use agent_core::{AgentLoop, Event, Goal};
use eframe::egui;
use hearth_tui::{LineKind, PersistFn, TuiState};
use tool_runtime::ToolDispatcher;

/// 一键证据导出（由 CLI 注入 `hearth session export` 同源实现）。
pub type ExportFn = Arc<dyn Fn(&str) -> anyhow::Result<std::path::PathBuf> + Send + Sync>;

pub struct GuiOptions {
    pub session_id: String,
    pub budget: u64,
    pub initial_goal: Option<String>,
    pub persist: Option<PersistFn>,
    pub export: Option<ExportFn>,
}

pub fn run(agent: AgentLoop, opts: GuiOptions) -> anyhow::Result<()> {
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([880.0, 560.0])
            .with_title("Hearth"),
        ..Default::default()
    };
    eframe::run_native(
        "Hearth",
        native,
        Box::new(move |cc| Ok(Box::new(HearthApp::new(cc, agent, opts)))),
    )
    .map_err(|e| anyhow::anyhow!("GUI 启动失败: {e}"))
}

struct HearthApp {
    rt: tokio::runtime::Runtime,
    evt_rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
    done_tx: tokio::sync::mpsc::UnboundedSender<anyhow::Result<(agent_core::RunReport, AgentLoop)>>,
    done_rx:
        tokio::sync::mpsc::UnboundedReceiver<anyhow::Result<(agent_core::RunReport, AgentLoop)>>,
    agent: Option<AgentLoop>,
    state: TuiState,
    dispatcher: Arc<ToolDispatcher>,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
    opts: GuiOptions,
    running: bool,
    pending_initial: Option<String>,
    last_export: Option<String>,
    err: Option<String>,
    /// 首帧把焦点给输入框（打开即可打字，无需先点一下）。
    focus_composer_once: bool,
}

impl HearthApp {
    fn new(cc: &eframe::CreationContext<'_>, mut agent: AgentLoop, opts: GuiOptions) -> Self {
        install_cjk_font(&cc.egui_ctx);
        apply_text_scale(&cc.egui_ctx);
        let dispatcher = agent.scheduler_arc();
        let interrupt = agent.interrupt_handle();
        let notify = agent.interrupt_notify();
        let (evt_tx, evt_rx) = tokio::sync::mpsc::unbounded_channel();
        agent.set_event_sender(evt_tx.clone());
        let (done_tx, done_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut state = TuiState::new(&opts.session_id);
        state.push_line(
            LineKind::System,
            "Hearth GUI · 输入任务后点“发送”（Enter 亦可）· 审批在弹窗内确认".into(),
        );
        Self {
            rt: tokio::runtime::Runtime::new().expect("创建 tokio runtime"),
            evt_rx,
            done_tx,
            done_rx,
            agent: Some(agent),
            state,
            dispatcher,
            interrupt,
            notify,
            pending_initial: opts.initial_goal.clone(),
            last_export: None,
            err: None,
            opts,
            running: false,
            focus_composer_once: true,
        }
    }

    fn submit(&mut self, text: String) {
        let text = text.trim().to_string();
        if text.is_empty() || self.running {
            return;
        }
        let Some(agent) = self.agent.take() else {
            self.err = Some("agent 不可用（上一轮异常）——请重启应用".into());
            return;
        };
        self.state.push_line(LineKind::User, format!("› {text}"));
        self.state.phase = "planning".into();
        self.state.steps = 0;
        self.running = true;
        self.err = None;
        let budget = self.opts.budget;
        let done_tx = self.done_tx.clone();
        self.rt.spawn(async move {
            let goal = Goal::with_budget(
                text,
                agent_types::Budget {
                    max_steps: budget,
                    ..Default::default()
                },
            );
            let r = agent.run_take(goal).await;
            let _ = done_tx.send(r);
        });
    }

    fn interrupt(&mut self) {
        if !self.running {
            return;
        }
        self.interrupt.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        self.state
            .push_line(LineKind::System, "⏸ 已请求中断（等待本步收尾…）".into());
    }

    fn resolve(&self, id: &str, approved: bool) {
        let d = self.dispatcher.clone();
        let sid = self.opts.session_id.clone();
        let id = id.to_string();
        self.rt.spawn(async move {
            let _ = d
                .resolve_interaction(
                    &sid,
                    &id,
                    approved,
                    serde_json::json!({ "answer": if approved { "approve" } else { "deny" } }),
                )
                .await;
        });
    }

    fn drain(&mut self) {
        while let Ok(evt) = self.evt_rx.try_recv() {
            self.state.ingest(&evt);
        }
        if let Ok(r) = self.done_rx.try_recv() {
            self.running = false;
            // 未决审批随本轮结束即失效（内核 60s 等待超时按拒绝处理）——
            // 不清会留下一个已无法生效的审批条（实测病灶）。
            if self.state.pending_approval.take().is_some() {
                self.state.push_line(
                    LineKind::System,
                    "⚠ 本轮已结束：未处理的审批按超时拒绝（未执行）".into(),
                );
            }
            match r {
                Ok((report, back)) => {
                    self.state.steps = report.steps;
                    if let Some(p) = &self.opts.persist {
                        p(&back);
                    }
                    self.agent = Some(back);
                }
                Err(e) => {
                    // agent 随失败一起丢失——如实提示，不假装还能继续。
                    self.err = Some(format!("run 失败：{e:#}"));
                    self.state
                        .push_line(LineKind::System, format!("⚠ run 失败：{e:#}"));
                }
            }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Hearth").strong());
            ui.add_space(8.0);
            ui.label(format!("session {}", self.state.session_short));
            ui.separator();
            ui.label(format!("phase {}", self.state.phase));
            ui.label(format!("{} 步", self.state.steps));
            if self.running {
                ui.colored_label(egui::Color32::YELLOW, "运行中");
            } else {
                ui.colored_label(egui::Color32::GREEN, "就绪");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(self.running, egui::Button::new("中断"))
                    .clicked()
                {
                    self.interrupt();
                }
                if ui
                    .add_enabled(!self.running, egui::Button::new("导出证据"))
                    .clicked()
                {
                    if let Some(export) = &self.opts.export {
                        match export(&self.opts.session_id) {
                            Ok(p) => {
                                self.last_export = Some(format!("证据已导出 → {}", p.display()))
                            }
                            Err(e) => self.last_export = Some(format!("导出失败：{e:#}")),
                        }
                    }
                }
            });
        });
    }

    fn transcript(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for line in &self.state.lines {
                    let color = line_color(&line.kind);
                    ui.add(
                        egui::Label::new(egui::RichText::new(&line.text).monospace().color(color))
                            .wrap(),
                    );
                }
            });
    }

    /// 审批条：固定在输入框上方（此前用 floating Window——内容溢出窗口边框后
    /// 按钮落在裁剪区外，点击无效，只能等 60s 超时被拒；实测病灶）。
    fn approval_bar(&mut self, ui: &mut egui::Ui) {
        let Some(pa) = self.state.pending_approval.clone() else {
            return;
        };
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("⚠ 需要审批")
                    .strong()
                    .color(egui::Color32::from_rgb(230, 120, 230)),
            );
            ui.label(egui::RichText::new(format!("要执行：{}", pa.action)).strong());
        });
        if !pa.tools.is_empty() {
            for t in &pa.tools {
                ui.add(egui::Label::new(egui::RichText::new(t).monospace().small()).wrap());
            }
        }
        ui.horizontal(|ui| {
            if ui
                .button(egui::RichText::new("批准").color(egui::Color32::GREEN))
                .clicked()
            {
                self.resolve(&pa.id, true);
                self.state.pending_approval = None;
                self.state.push_line(LineKind::System, "✔ 已批准".into());
            }
            if ui
                .button(egui::RichText::new("拒绝").color(egui::Color32::RED))
                .clicked()
            {
                self.resolve(&pa.id, false);
                self.state.pending_approval = None;
                self.state.push_line(LineKind::System, "✘ 已拒绝".into());
            }
            ui.label(
                egui::RichText::new("破坏性命令 / workspace 外写入；60 秒未处理按拒绝")
                    .small()
                    .color(egui::Color32::GRAY),
            );
        });
        ui.separator();
    }

    fn composer(&mut self, ui: &mut egui::Ui) {
        self.approval_bar(ui);
        ui.horizontal(|ui| {
            let enabled = !self.running;
            let mut input = self.state.input.clone();
            let resp = ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(&mut input)
                    .hint_text("描述你的任务，例如：修复 src/lib.rs 的 fibonacci off-by-one，并跑 cargo test")
                    .desired_width(f32::INFINITY),
            );
            self.state.input = input;
            if self.focus_composer_once {
                resp.request_focus();
                self.focus_composer_once = false;
            }
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let clicked = ui
                .add_enabled(enabled, egui::Button::new("发送"))
                .clicked();
            if (enter || clicked) && !self.running {
                let text = std::mem::take(&mut self.state.input);
                self.submit(text);
                resp.request_focus();
            }
        });
        if let Some(msg) = &self.last_export {
            ui.label(egui::RichText::new(msg).small().color(egui::Color32::GRAY));
        }
        if let Some(err) = &self.err {
            ui.colored_label(egui::Color32::RED, err);
        }
    }
}

impl eframe::App for HearthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain();
        if let Some(g) = self.pending_initial.take() {
            self.submit(g);
        }

        egui::TopBottomPanel::top("hearth-top").show(ctx, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(4.0);
        });
        egui::TopBottomPanel::bottom("hearth-composer").show(ctx, |ui| {
            ui.add_space(4.0);
            self.composer(ui);
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.transcript(ui);
        });

        if self.running || self.state.pending_approval.is_some() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }
}

fn line_color(kind: &LineKind) -> egui::Color32 {
    match kind {
        LineKind::User => egui::Color32::from_rgb(110, 200, 255),
        LineKind::Assistant => egui::Color32::from_rgb(235, 235, 235),
        LineKind::Tool => egui::Color32::from_rgb(230, 200, 90),
        LineKind::ToolOk => egui::Color32::from_rgb(110, 210, 120),
        LineKind::ToolErr => egui::Color32::from_rgb(240, 110, 110),
        LineKind::System => egui::Color32::from_rgb(150, 150, 150),
        LineKind::Approval => egui::Color32::from_rgb(230, 120, 230),
    }
}

/// 默认 egui 字号（12.5/14）在高分屏上过小——按桌面 IDE 的阅读密度放大，
/// 并拉开行距（中文行高比拉丁字体更吃空间）。
fn apply_text_scale(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, TextStyle};
    ctx.style_mut(|style| {
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(21.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(15.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(14.5, FontFamily::Monospace),
            ),
            (
                TextStyle::Small,
                FontId::new(12.5, FontFamily::Proportional),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    });
}

/// egui 默认字体不含 CJK——不装字体会把中文显示成方框。
/// Windows 取系统自带的 SimHei / YaHei；缺失则保持默认（不崩）。
fn install_cjk_font(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simsun.ttc",
    ];
    for p in candidates {
        let Ok(bytes) = std::fs::read(p) else {
            continue;
        };
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            list.push("cjk".to_owned());
        }
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            list.push("cjk".to_owned());
        }
        ctx.set_fonts(fonts);
        return;
    }
}
