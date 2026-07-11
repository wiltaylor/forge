//! Chat: transcript, tool calls, interactive prompt, and the composer
//! (feature `chat`).

use forge_egui::prelude::*;

pub struct ChatSectionState {
    pub items: Vec<ChatItem>,
    pub view: ChatViewState,
    pub composer: String,
    pub prompt: ChatPromptState,
    pub prompt_answered: Option<String>,
}

impl Default for ChatSectionState {
    fn default() -> ChatSectionState {
        ChatSectionState {
            items: vec![
                ChatItem::Divider("today".into()),
                ChatItem::Message {
                    role: Role::User,
                    name: Some("wil".into()),
                    time: Some("09:41".into()),
                    markdown: "Can you check why the **staging deploy** failed?".into(),
                },
                ChatItem::tool_with(
                    "read_file",
                    ToolStatus::Ok,
                    "deploy/staging.log — 412 lines",
                ),
                ChatItem::tool_with(
                    "run_command",
                    ToolStatus::Error,
                    "kubectl rollout status → error: deployment exceeded its progress deadline",
                ),
                ChatItem::Message {
                    role: Role::Assistant,
                    name: None,
                    time: Some("09:42".into()),
                    markdown: "The rollout timed out. Two findings:\n\n1. The image tag \
`v2.4.1` was never pushed\n2. Readiness probes fail on `/healthz`\n\nI can retag and \
redeploy — see the [runbook](https://forge.dev/runbooks/deploy) first."
                        .into(),
                },
                ChatItem::tool("watch_pipeline", ToolStatus::Running),
                ChatItem::Typing,
            ],
            view: ChatViewState::default(),
            composer: String::new(),
            prompt: ChatPromptState::default(),
            prompt_answered: None,
        }
    }
}

pub fn draw(ui: &mut egui::Ui, state: &mut ChatSectionState) {
    Card::new().title("Transcript").show(ui, |ui| {
        let _ = ChatView::new(&state.items)
            .max_height(340.0)
            .show(ui, &mut state.view);
        ui.add_space(8.0);
        let response = Composer::new(&mut state.composer).show(ui);
        if response.submitted() {
            let draft = std::mem::take(&mut state.composer);
            state.items.push(ChatItem::user(draft));
            state.view.stick = true;
        }
    });
    ui.add_space(12.0);

    Card::new().title("ChatPrompt").show(ui, |ui| {
        let data = ChatPromptData::new(
            "Retag v2.4.1 and redeploy staging?",
            PromptControl::Radio(vec![
                "Yes — redeploy now".into(),
                "Wait for the pipeline".into(),
                "Cancel the rollout".into(),
            ]),
        );
        if let Some(answer) = ChatPrompt::new(&data).show(ui, &mut state.prompt) {
            state.prompt_answered = Some(format!("{answer:?}"));
        }
        if let Some(answered) = &state.prompt_answered {
            ui.add_space(6.0);
            let _ = Badge::new(answered)
                .tone(forge_egui::widgets::Tone::Success)
                .show(ui);
        }
    });
    ui.add_space(12.0);

    Card::new().title("LinkCard").show(ui, |ui| {
        let _ = LinkCard::new("Forge design system", "https://forge.dev/docs")
            .description("Dark-default, dense, technical-tools aesthetic for consoles.")
            .show(ui);
    });
}
