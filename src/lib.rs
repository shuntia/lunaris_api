#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
#![deny(clippy::style)]

pub mod plugin;
pub mod prelude;
pub mod protocol;
pub mod render;
pub mod request;
pub mod timeline;
pub mod ui;
pub mod util;

pub use egui;
pub use parking_lot;
