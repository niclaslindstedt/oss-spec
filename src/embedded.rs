//! Embedded templates. The entire `templates/` directory is compiled into the
//! binary at build time so `oss-spec` can be `cargo install`-ed and used
//! standalone with no runtime data files.

use include_dir::{Dir, include_dir};

pub static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");
