//! Interaction-contract tests for the specialty widgets (markdown, chat,
//! code, flowchart), driven headless through egui_kittest.

use egui_kittest::Harness;
use forge_egui::prelude::*;

fn themed_harness<'a>(app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    let mut harness = Harness::new_ui(app);
    Theme::dark().apply(&harness.ctx);
    harness.run();
    harness
}

#[cfg(feature = "markdown")]
mod markdown {
    use super::*;
    use egui_kittest::kittest::Queryable;

    const DOC: &str = "# Title\n\nBody with [ok link](https://forge.dev) and \
[evil link](javascript:alert(1)) plus `inline code`.\n\n```rust\nfn main() {}\n```";

    fn open_urls(harness: &Harness) -> Vec<String> {
        harness
            .output()
            .platform_output
            .commands
            .iter()
            .filter_map(|c| match c {
                egui::OutputCommand::OpenUrl(open) => Some(open.url.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn renders_headings_code_and_links() {
        let mut harness = themed_harness(|ui| {
            let _ = Markdown::new(DOC).show(ui);
        });
        harness.run();
        // Heading text is present.
        let _ = harness.get_by_label("Title");
        // Inline code renders (padded chip).
        let _ = harness.get_by_label(" inline code ");
        // The safe link is a real hyperlink: clicking it opens the URL.
        harness.get_by_label("ok link").click();
        harness.step();
        assert_eq!(open_urls(&harness), vec!["https://forge.dev".to_owned()]);
    }

    #[test]
    fn javascript_scheme_is_sanitized_to_plain_text() {
        let mut harness = themed_harness(|ui| {
            let _ = Markdown::new(DOC).show(ui);
        });
        harness.run();
        // The evil link's text still shows, but clicking it opens nothing.
        harness.get_by_label("evil link").click();
        harness.step();
        assert!(
            open_urls(&harness).is_empty(),
            "javascript: link must never open"
        );
    }
}

#[cfg(feature = "chat")]
mod chat {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use std::cell::RefCell;

    #[test]
    fn prompt_button_click_returns_answer() {
        let data = ChatPromptData::new(
            "Deploy now?",
            PromptControl::Buttons(vec!["Yes".into(), "No".into(), "Later".into()]),
        );
        let state = RefCell::new(ChatPromptState::default());
        let answer = RefCell::new(None);
        let mut harness = themed_harness(|ui| {
            let mut s = state.borrow_mut();
            if let Some(a) = ChatPrompt::new(&data).show(ui, &mut s) {
                *answer.borrow_mut() = Some(a);
            }
        });
        harness.get_by_label("No").click();
        harness.run();
        drop(harness);
        assert_eq!(*answer.borrow(), Some(PromptAnswer::Button(1)));
    }

    #[test]
    fn prompt_radio_needs_submit() {
        let data = ChatPromptData::new(
            "Pick one",
            PromptControl::Radio(vec!["Alpha".into(), "Beta".into()]),
        );
        let state = RefCell::new(ChatPromptState::default());
        let answer = RefCell::new(None);
        let mut harness = themed_harness(|ui| {
            let mut s = state.borrow_mut();
            if let Some(a) = ChatPrompt::new(&data).show(ui, &mut s) {
                *answer.borrow_mut() = Some(a);
            }
        });
        harness.get_by_label("Beta").click();
        harness.run();
        assert_eq!(state.borrow().choice, Some(1));
        assert_eq!(*answer.borrow(), None, "no answer before submit");
        harness.get_by_label("Submit").click();
        harness.run();
        drop(harness);
        assert_eq!(*answer.borrow(), Some(PromptAnswer::Radio(1)));
    }

    #[test]
    fn composer_enter_submits() {
        let text = RefCell::new(String::new());
        let submitted = RefCell::new(false);
        let mut harness = themed_harness(|ui| {
            let mut draft = text.borrow_mut();
            let response = Composer::new(&mut draft).show(ui);
            if response.submitted() {
                *submitted.borrow_mut() = true;
            }
        });
        let node = harness.get_by_role(egui::accesskit::Role::MultilineTextInput);
        node.focus();
        node.type_text("hello there");
        harness.run();
        assert_eq!(*text.borrow(), "hello there");
        assert!(!*submitted.borrow());
        harness.key_press(egui::Key::Enter);
        harness.run();
        drop(harness);
        assert!(*submitted.borrow(), "Enter should submit the draft");
        // Enter must not have inserted a newline into the draft.
        assert_eq!(*text.borrow(), "hello there");
    }

    #[test]
    fn chat_view_renders_all_item_kinds() {
        let items = vec![
            ChatItem::user("Hi **there**"),
            ChatItem::assistant("All good."),
            ChatItem::tool_with("read_file", ToolStatus::Ok, "src/main.rs"),
            ChatItem::Divider("today".into()),
            ChatItem::Typing,
        ];
        let state = RefCell::new(ChatViewState::default());
        // The typing indicator animates (requests repaints every frame), so
        // step a fixed number of frames instead of `run()`.
        let mut harness = Harness::new_ui(|ui| {
            let mut s = state.borrow_mut();
            let _ = ChatView::new(&items).max_height(400.0).show(ui, &mut s);
        });
        Theme::dark().apply(&harness.ctx);
        for _ in 0..4 {
            harness.step();
        }
        let _ = harness.get_by_label("All good.");
        let _ = harness.get_by_label("read_file");
        assert!(state.borrow().stick, "short transcript stays stuck");
    }
}

#[cfg(feature = "code")]
mod code {
    use super::*;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn code_view_and_diff_render() {
        let annotations = [CodeAnnotation::new(2, Severity::Warning, "unused variable")];
        let mut harness = themed_harness(move |ui| {
            let _ = CodeView::new("fn main() {\n    let x = 1;\n}", "rs")
                .annotations(&annotations)
                .show(ui);
            let _ = DiffView::new("a\nb", "a\nc").show(ui);
        });
        harness.run();
        // Highlighted line labels exist (whole-line labels).
        let _ = harness.get_by_label_contains("let x = 1;");
    }
}

#[test]
fn flowchart_renders_without_panic() {
    let nodes = vec![
        FlowNode::new("a", "Start"),
        FlowNode::new("b", "Middle"),
        FlowNode::new("c", "End").tone(forge_egui::widgets::Tone::Success),
    ];
    let edges = vec![
        FlowEdge::new("a", "b").label("go"),
        FlowEdge::new("b", "c").broken(true),
    ];
    let mut harness = themed_harness(move |ui| {
        let _ = Flowchart::new(&nodes, &edges).show(ui);
    });
    harness.run();
}
