mod app;
#[allow(clippy::doc_markdown)]
mod sys;

pub use app::{get_settings, save_settings};
pub use sys::*;
