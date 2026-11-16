//! Pack metadata: the struct and the code rules.

use super::*;

/// The identity of a pack (`pack.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackMeta {
    /// Language tag, also the folder name (`fr`, `pt-BR`).
    pub code: String,
    /// English name of the language.
    pub name: String,
    /// Name of the language in itself.
    pub native_name: String,
    /// Writing direction.
    pub direction: Direction,
    /// Version of the pack.
    pub version: String,
    /// Authors (names or handles); AI-assisted packs say so here.
    pub authors: Vec<String>,
    /// Licence of the text; expected [`EXPECTED_LICENSE`].
    pub license: String,
    /// The English revision ([`Reference::revision`]) the strings were made against; may be empty.
    pub base_version: String,
}

impl PackMeta {
    /// `pack.json` text (two-space indent, stable field order, trailing newline).
    #[must_use]
    pub fn to_json(&self) -> String {
        let q = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into());
        let authors: Vec<String> = self.authors.iter().map(|a| q(a)).collect();
        format!(
            "{{\n  \"code\": {},\n  \"name\": {},\n  \"native_name\": {},\n  \"direction\": {},\n  \"version\": {},\n  \"authors\": [{}],\n  \"license\": {},\n  \"base_version\": {}\n}}\n",
            q(&self.code),
            q(&self.name),
            q(&self.native_name),
            q(self.direction.name()),
            q(&self.version),
            authors.join(", "),
            q(&self.license),
            q(&self.base_version),
        )
    }
}

/// `true` for a well-formed language tag: a 2 or 3 letter lower-case language, then optional
/// 2 to 8 character alphanumeric subtags (`fr`, `pt-BR`, `zh-Hans`), at most 20 characters.
#[must_use]
pub fn valid_code(code: &str) -> bool {
    if code.len() > 20 {
        return false;
    }
    let mut parts = code.split('-');
    let lang = parts.next().unwrap_or("");
    (2..=3).contains(&lang.len())
        && lang.chars().all(|c| c.is_ascii_lowercase())
        && parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}
