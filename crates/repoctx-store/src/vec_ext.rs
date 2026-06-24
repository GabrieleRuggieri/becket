//! Registers the sqlite-vec extension with rusqlite (once per process).

use std::sync::OnceLock;

static VEC_REGISTERED: OnceLock<()> = OnceLock::new();

/// Ensures sqlite-vec is registered before opening SQLite connections.
pub fn ensure_sqlite_vec() {
    VEC_REGISTERED.get_or_init(|| unsafe {
        let init: rusqlite::auto_extension::RawAutoExtension =
            std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const () as usize);
        rusqlite::auto_extension::register_auto_extension(init)
            .expect("failed to register sqlite-vec extension");
    });
}
