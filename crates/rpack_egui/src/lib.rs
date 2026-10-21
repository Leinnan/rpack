#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod fonts;
mod helpers;
mod json_preview;
mod notifications;
mod preview;
mod view_settings;
pub use app::Application;
pub use app::ICON_DATA;
