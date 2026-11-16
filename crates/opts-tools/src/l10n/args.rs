//! Command-line parsing for `opts-tools l10n`: positionals, value flags, boolean flags.

/// Parsed command line: positionals, flags with a value, boolean flags.
#[derive(Debug, Default)]
pub(super) struct Args {
    pub(super) positional: Vec<String>,
    pub(super) values: Vec<(String, String)>,
    pub(super) flags: Vec<String>,
}

impl Args {
    pub(super) fn parse(args: &[String], value_flags: &[&str], bool_flags: &[&str]) -> Result<Self, String> {
        let mut a = Self::default();
        let mut it = args.iter();
        while let Some(x) = it.next() {
            if let Some(name) = x.strip_prefix("--") {
                if value_flags.contains(&name) {
                    let v = it.next().ok_or_else(|| format!("--{name} needs a value"))?;
                    a.values.push((name.to_string(), v.clone()));
                } else if bool_flags.contains(&name) {
                    a.flags.push(name.to_string());
                } else {
                    return Err(format!("unknown option --{name}"));
                }
            } else {
                a.positional.push(x.clone());
            }
        }
        Ok(a)
    }

    pub(super) fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    pub(super) fn all(&self, name: &str) -> Vec<&str> {
        self.values
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    pub(super) fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }
}
