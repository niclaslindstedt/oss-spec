//! minijinja environment + helpers for rendering manifest data into templates.

use anyhow::{Context, Result};
use minijinja::{Environment, Value};

use crate::manifest::ProjectManifest;

/// Build a fresh minijinja environment. We don't cache it because each render
/// pass is short and we want clean error reporting.
pub fn env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_trim_blocks(false);
    env.set_lstrip_blocks(false);
    env
}

/// Render a single template string against the given manifest.
pub fn render_str(template_name: &str, source: &str, manifest: &ProjectManifest) -> Result<String> {
    let mut env = env();
    env.add_template_owned(template_name.to_string(), source.to_string())
        .with_context(|| format!("failed to compile template {template_name}"))?;
    let tmpl = env
        .get_template(template_name)
        .with_context(|| format!("template {template_name} missing after add"))?;
    let ctx = manifest_to_value(manifest);
    tmpl.render(ctx)
        .with_context(|| format!("failed to render template {template_name}"))
}

/// Convert a ProjectManifest into a minijinja Value (via serde_json).
fn manifest_to_value(manifest: &ProjectManifest) -> Value {
    let json = serde_json::to_value(manifest).expect("ProjectManifest serializes");
    Value::from_serialize(&json)
}
