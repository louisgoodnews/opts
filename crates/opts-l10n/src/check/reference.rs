//! The English reference a translation is checked against.

use std::collections::{BTreeSet, HashMap};

use crate::fluent::{self, Table};

/// The English source every translation is checked against.
#[derive(Debug, Clone, Default)]
pub struct Reference {
    /// All English messages (comments and file order included).
    pub table: Table,
    /// Message key to the name of the source file it comes from.
    pub file_of: HashMap<String, String>,
    vars: HashMap<String, BTreeSet<String>>,
}

impl Reference {
    /// A reference from named `.ftl` sources `(file name, text)`; later files may not redefine a
    /// key of an earlier one (the later definition wins).
    #[must_use]
    pub fn from_sources(sources: &[(&str, &str)]) -> Self {
        let mut table = Table::default();
        let mut file_of = HashMap::new();
        for (name, text) in sources {
            let t = fluent::parse(text);
            for key in &t.order {
                file_of.insert(key.clone(), (*name).to_string());
                if !table.messages.contains_key(key) {
                    table.order.push(key.clone());
                }
            }
            table.messages.extend(t.messages);
            table.lines.extend(t.lines);
            table.comments.extend(t.comments);
            table.diagnostics.extend(t.diagnostics);
        }
        table.errors = table.diagnostics.iter().map(ToString::to_string).collect();
        Self::from_table(table, file_of)
    }

    /// A reference over an already merged table.
    #[must_use]
    pub fn from_table(table: Table, file_of: HashMap<String, String>) -> Self {
        let vars = table
            .messages
            .iter()
            .map(|(k, p)| (k.clone(), fluent::variables(p)))
            .collect();
        Self {
            table,
            file_of,
            vars,
        }
    }

    /// Number of messages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.table.messages.len()
    }

    /// `true` without messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.table.messages.is_empty()
    }

    /// The keys in file order.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.table.order
    }

    /// The variables and references of the English message `key`.
    #[must_use]
    pub fn variables_of(&self, key: &str) -> Option<&BTreeSet<String>> {
        self.vars.get(key)
    }

    /// A short hash of all English keys and texts: a pack records it as `base_version`, so a later
    /// check can tell that the source has moved on.
    #[must_use]
    pub fn revision(&self) -> String {
        let mut keys: Vec<&String> = self.table.messages.keys().collect();
        keys.sort();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for k in keys {
            let line = format!("{k}={}\n", fluent::to_source(&self.table.messages[k]));
            for b in line.bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        format!("{:08x}", h >> 32 ^ (h & 0xffff_ffff))
    }
}
