//! Thin wrappers around `zag` for the LLM-driven steps. Each function asks
//! zag for a JSON-schema-validated response and parses it into native types.
//! Callers should treat AI failures as non-fatal — the bootstrap path always
//! has a deterministic fallback.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::json;

use crate::manifest::{Kind, Language, License, ProjectManifest};

const SYSTEM: &str = "You are a project bootstrapping assistant for the oss-spec CLI. \
You return ONLY structured JSON matching the requested schema. Never include prose.";

/// Parse a freeform user prompt (e.g. "create a python cli for finding stock buys")
/// into a partially-filled ProjectManifest.
pub async fn interpret_prompt(prompt: &str) -> Result<ProjectManifest> {
    let schema = json!({
        "type": "object",
        "required": ["name", "description", "language", "kind", "license"],
        "properties": {
            "name":        { "type": "string", "description": "Short kebab-case project name." },
            "description": { "type": "string", "description": "One concise sentence." },
            "language":    { "type": "string", "enum": ["rust","python","node","go","generic"] },
            "kind":        { "type": "string", "enum": ["lib","cli","service"] },
            "license":     { "type": "string", "enum": ["MIT","Apache-2.0","MPL-2.0"] },
            "why_bullets": {
                "type": "array",
                "items": { "type": "string" },
                "description": "3-5 concrete value propositions for the README 'Why?' section.",
                "maxItems": 5
            }
        }
    });

    let user =
        format!("Interpret the following project request and emit the JSON manifest:\n\n{prompt}");

    let raw = run_zag(&user, schema).await?;

    #[derive(Deserialize)]
    struct Wire {
        name: String,
        description: String,
        language: String,
        kind: String,
        license: String,
        #[serde(default)]
        why_bullets: Vec<String>,
    }

    let wire: Wire = serde_json::from_str(&raw)
        .with_context(|| format!("zag returned non-conforming JSON: {raw}"))?;

    let mut m = ProjectManifest::skeleton(&wire.name, &wire.description);
    m.language = Language::parse(&wire.language).unwrap_or(Language::Generic);
    m.kind = Kind::parse(&wire.kind).unwrap_or(Kind::Cli);
    m.license = License::parse(&wire.license).unwrap_or(License::Mit);
    m.why_bullets = wire.why_bullets;
    Ok(m)
}

/// Generate 3–5 README "Why?" bullet points for a project.
pub async fn draft_readme_why(description: &str, name: &str) -> Result<Vec<String>> {
    let schema = json!({
        "type": "object",
        "required": ["bullets"],
        "properties": {
            "bullets": {
                "type": "array",
                "items": { "type": "string" },
                "minItems": 3,
                "maxItems": 5
            }
        }
    });
    let user = format!(
        "Project name: {name}\nDescription: {description}\n\nDraft 3-5 short, concrete \
'Why?' bullets for the README. Each bullet should describe a tangible benefit, not a feature."
    );
    let raw = run_zag(&user, schema).await?;

    #[derive(Deserialize)]
    struct Wire {
        bullets: Vec<String>,
    }
    let wire: Wire = serde_json::from_str(&raw)
        .with_context(|| format!("zag returned non-conforming JSON: {raw}"))?;
    Ok(wire.bullets)
}

async fn run_zag(prompt: &str, schema: serde_json::Value) -> Result<String> {
    use zag::builder::AgentBuilder;

    let output = AgentBuilder::new()
        .system_prompt(SYSTEM)
        .auto_approve(true)
        .json_schema(schema)
        .exec(prompt)
        .await
        .context("zag agent execution failed")?;

    output
        .result
        .ok_or_else(|| anyhow!("zag returned no result text"))
}
