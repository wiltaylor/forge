#[cfg(feature = "chat")]
mod chat;
#[cfg(feature = "code")]
mod code;
mod flowchart;
#[cfg(feature = "markdown")]
mod markdown;
#[cfg(feature = "term")]
mod terminal;

#[cfg(feature = "chat")]
pub use chat::{
    ChatItem, ChatPrompt, ChatPromptState, ChatView, ChatViewState, Composer, ComposerState,
    Role, ToolStatus,
};
#[cfg(feature = "code")]
pub use code::{CodeView, CodeViewState, DiffView};
pub use flowchart::{FlowEdge, FlowNode, Flowchart};
#[cfg(feature = "markdown")]
pub use markdown::{markdown_lines, Markdown};
#[cfg(feature = "term")]
pub use terminal::{Terminal, TerminalState};

#[cfg(feature = "term")]
pub use portable_pty::CommandBuilder;
