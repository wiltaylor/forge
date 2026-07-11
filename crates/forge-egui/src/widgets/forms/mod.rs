//! Form controls. Value-bound widgets borrow the app's data
//! (`Input::new(&mut text)`, `Checkbox::new(&mut bool, ..)`); pickers with
//! real internal state pair with an explicit `FooState` struct owned by the
//! app (`SelectState`, `ComboboxState`, `ListBoxState`) — plain data,
//! headless-testable.

mod checkbox;
mod combobox;
mod field;
mod form;
mod input;
mod list_box;
mod radio_group;
mod select;
mod slider;
mod textarea;
mod toggle;
mod toggle_group;

pub use checkbox::Checkbox;
pub use combobox::{Combobox, ComboboxState};
pub use form::FormState;
pub use input::Input;
pub use list_box::{ListBox, ListBoxState};
pub use radio_group::RadioGroup;
pub use select::{Select, SelectState};
pub use slider::Slider;
pub use textarea::Textarea;
pub use toggle::Toggle;
pub use toggle_group::ToggleGroup;
