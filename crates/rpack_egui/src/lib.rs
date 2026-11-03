#![warn(clippy::all, rust_2018_idioms)]

macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
        puffin::profile_scope!($name);
    };
}

mod app;
mod commands;
#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
mod debug_capture;
mod events;
mod menu;
mod platform;
mod settings;
mod state;
#[cfg(test)]
mod tests;
mod ui;

pub use app::Application;

pub const ICON_DATA: &[u8] = include_bytes!("../static/base_icon.png");
