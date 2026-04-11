//! ProjectManifest — the structured intent the bootstrap engine renders against.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    Rust,
    Python,
    Node,
    Go,
    Generic,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::Node => "node",
            Language::Go => "go",
            Language::Generic => "generic",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "rust" | "rs" => Some(Self::Rust),
            "python" | "py" => Some(Self::Python),
            "node" | "javascript" | "js" | "typescript" | "ts" => Some(Self::Node),
            "go" | "golang" => Some(Self::Go),
            "generic" | "other" => Some(Self::Generic),
            _ => None,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Lib,
    Cli,
    Service,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Lib => "lib",
            Kind::Cli => "cli",
            Kind::Service => "service",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "lib" | "library" => Some(Self::Lib),
            "cli" | "tool" | "binary" | "bin" => Some(Self::Cli),
            "service" | "server" | "daemon" => Some(Self::Service),
            _ => None,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum License {
    Mit,
    Apache20,
    Mpl20,
}

impl Serialize for License {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.spdx())
    }
}

impl<'de> Deserialize<'de> for License {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        License::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown license {s}")))
    }
}

impl License {
    pub fn spdx(self) -> &'static str {
        match self {
            License::Mit => "MIT",
            License::Apache20 => "Apache-2.0",
            License::Mpl20 => "MPL-2.0",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "mit" => Some(Self::Mit),
            "apache-2.0" | "apache2" | "apache" => Some(Self::Apache20),
            "mpl-2.0" | "mpl2" | "mpl" => Some(Self::Mpl20),
            _ => None,
        }
    }

    pub fn template_filename(self) -> &'static str {
        match self {
            License::Mit => "LICENSE-MIT.tmpl",
            License::Apache20 => "LICENSE-Apache-2.0.tmpl",
            License::Mpl20 => "LICENSE-MPL-2.0.tmpl",
        }
    }
}

impl fmt::Display for License {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spdx())
    }
}

/// All the inputs the bootstrap engine needs to materialize a new repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub name: String,
    pub description: String,
    pub language: Language,
    pub kind: Kind,
    pub license: License,
    pub author_name: String,
    pub author_email: String,
    pub github_owner: String,
    pub year: i32,
    /// Optional bullet-list "Why?" lines for the README. May be empty.
    #[serde(default)]
    pub why_bullets: Vec<String>,
}

impl ProjectManifest {
    /// Build a sensible default manifest from minimal inputs (used by --no-ai path
    /// and as the seed before AI fills in extras).
    pub fn skeleton(name: impl Into<String>, description: impl Into<String>) -> Self {
        let now = chrono_year();
        Self {
            name: name.into(),
            description: description.into(),
            language: Language::Rust,
            kind: Kind::Cli,
            license: License::Mit,
            author_name: String::from("Your Name"),
            author_email: String::from("you@example.com"),
            github_owner: String::from("your-github"),
            year: now,
            why_bullets: vec![],
        }
    }

    /// True if this project ships a CLI binary (kind=cli or kind=service).
    pub fn ships_cli(&self) -> bool {
        matches!(self.kind, Kind::Cli | Kind::Service)
    }
}

/// Cheap year lookup without pulling in `chrono`. Falls back to 2026 if the
/// system clock is unreadable (vanishingly unlikely).
fn chrono_year() -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(1_775_000_000);
    // Days since epoch / 365.2425, then 1970 base.
    let days = secs / 86_400;
    1970 + ((days as f64) / 365.2425) as i32
}
