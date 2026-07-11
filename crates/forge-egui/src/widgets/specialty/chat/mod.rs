//! Chat kit (cargo feature `chat`, implies `markdown`): transcript view with
//! message bubbles and tool-call boxes, a composer, interactive prompts, and
//! link cards — the egui mirror of `@forge/chat` and forge-tui's chat kit.

mod composer;
mod link_card;
mod prompt;
mod view;

pub use composer::Composer;
pub use link_card::LinkCard;
pub use prompt::{ChatPrompt, ChatPromptData, ChatPromptState, PromptAnswer, PromptControl};
pub use view::{ChatView, ChatViewState};

/// Who authored a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

/// Tool-call lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Ok,
    Error,
}

/// One transcript entry.
#[derive(Clone, Debug)]
pub enum ChatItem {
    Message {
        role: Role,
        name: Option<String>,
        time: Option<String>,
        /// Body markdown, rendered through [`Markdown`](crate::widgets::Markdown).
        markdown: String,
    },
    ToolCall {
        title: String,
        status: ToolStatus,
        /// Collapsible mono detail (tool args/result).
        body: Option<String>,
    },
    /// A centered labelled rule ("Today", "New messages", …).
    Divider(String),
    /// Animated typing indicator.
    Typing,
}

impl ChatItem {
    pub fn user(markdown: impl Into<String>) -> ChatItem {
        ChatItem::Message {
            role: Role::User,
            name: None,
            time: None,
            markdown: markdown.into(),
        }
    }

    pub fn assistant(markdown: impl Into<String>) -> ChatItem {
        ChatItem::Message {
            role: Role::Assistant,
            name: None,
            time: None,
            markdown: markdown.into(),
        }
    }

    pub fn system(markdown: impl Into<String>) -> ChatItem {
        ChatItem::Message {
            role: Role::System,
            name: None,
            time: None,
            markdown: markdown.into(),
        }
    }

    pub fn tool(title: impl Into<String>, status: ToolStatus) -> ChatItem {
        ChatItem::ToolCall {
            title: title.into(),
            status,
            body: None,
        }
    }

    pub fn tool_with(
        title: impl Into<String>,
        status: ToolStatus,
        body: impl Into<String>,
    ) -> ChatItem {
        ChatItem::ToolCall {
            title: title.into(),
            status,
            body: Some(body.into()),
        }
    }
}
