use std::sync::atomic::{AtomicUsize, Ordering};

pub mod avatar;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod checkbox;
pub mod chip;
pub mod citations;
pub mod collapsible;
pub mod dialog;
pub mod icon_button;
pub mod input;
pub mod list_item;
pub mod menu;
pub mod popover;
pub mod select;
pub mod separator;
pub mod side_panel;
pub mod switch;
pub mod table;
pub mod tabs;
pub mod tag;
pub mod textarea;
pub mod theme;
pub mod toast;
pub mod tooltip;

pub use avatar::*;
pub use badge::*;
pub use breadcrumb::*;
pub use button::*;
pub use checkbox::*;
pub use chip::*;
pub use citations::*;
pub use collapsible::*;
pub use dialog::*;
pub use icon_button::*;
pub use input::*;
pub use list_item::*;
pub use menu::*;
pub use popover::*;
pub use select::*;
pub use separator::*;
pub use side_panel::*;
pub use switch::*;
pub use table::*;
pub use tabs::*;
pub use tag::*;
pub use textarea::*;
pub use theme::*;
pub use toast::*;
pub use tooltip::*;

static NEXT_DOM_ID: AtomicUsize = AtomicUsize::new(1);

pub(crate) fn next_dom_id(prefix: &str) -> String {
    format!("{prefix}-{}", NEXT_DOM_ID.fetch_add(1, Ordering::Relaxed))
}
