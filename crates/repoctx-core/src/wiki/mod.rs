//! Grounded knowledge wiki — graph-compiled markdown pages.

pub mod compiler;
pub mod fingerprint;
pub mod lint;
pub mod store;
pub mod util;

pub use compiler::WikiCompiler;
pub use lint::{claim_calls_valid, WikiLinter};
pub use store::{find_page_for_symbol, load_page_from_path, WikiStore};
pub use util::{
    replace_prose_slot, sanitize_for_context, wiki_adds_context, PROSE_PLACEHOLDER, PROSE_SLOT,
};
