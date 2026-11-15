//! Keyword lines and the blocks of a text file: the keyword, its parameter lines and the arity.

/// Checks if a trimmed line is an OMSI keyword enclosed in `[...]`.
///
/// Returns the inner keyword name without brackets if valid.
#[must_use]
pub fn parse_keyword(line: &str) -> Option<&str> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
        Some(&line[1..line.len() - 1])
    } else {
        None
    }
}

/// Represents an extracted OMSI configuration block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmsiBlock {
    /// The keyword string without square brackets (e.g. `"currency"`, `"coin"`).
    pub keyword: String,
    /// 1-based line number where the keyword occurred in the source text.
    pub line_number: usize,
    /// Parameter lines consumed by this keyword.
    pub parameters: Vec<String>,
}

/// Scans OMSI text into structured [`OmsiBlock`] instances.
///
/// Lines preceding the first keyword and lines beginning with comment
/// indicators outside parameter blocks are ignored. Every non-blank line after a keyword up to the
/// next keyword is one of its parameters; see [`scan_blocks_with`] for keywords whose parameter
/// count is known.
#[must_use]
pub fn scan_blocks(text: &str) -> Vec<OmsiBlock> {
    scan_blocks_with(text, |_| None)
}

/// [`scan_blocks`] for formats in which some keywords take a fixed number of parameter *lines*.
///
/// `arity(keyword)` returns that number (lower-case keyword). For such a keyword exactly that many
/// lines after it belong to it, **a blank line counting as an empty parameter**; further lines
/// up to the next keyword are free text between blocks (author notes without a comment marker) and are
/// dropped instead of becoming parameters. A keyword with `Some(0)` takes none. Comment lines (`//`,
/// `#`) are never parameters. Keywords with `None` behave as in [`scan_blocks`].
#[must_use]
pub fn scan_blocks_with(text: &str, arity: impl Fn(&str) -> Option<usize>) -> Vec<OmsiBlock> {
    let mut blocks = Vec::new();
    let mut current_block: Option<OmsiBlock> = None;
    // Parameter lines the current keyword still takes literally (`None`: free-form).
    let mut literal: Option<usize> = None;

    for (line_idx, raw_line) in text.lines().enumerate() {
        let line_number = line_idx + 1;
        let line = raw_line.trim_end_matches(['\r', '\n']).trim();

        // Check for keyword beginning line
        if !line.is_empty() && raw_line.starts_with('[') && raw_line.trim_end().ends_with(']') {
            if let Some(kw) = parse_keyword(raw_line.trim_end()) {
                if let Some(prev) = current_block.take() {
                    blocks.push(prev);
                }
                let keyword = kw.to_ascii_lowercase();
                literal = arity(&keyword);
                current_block = Some(OmsiBlock {
                    keyword,
                    line_number,
                    parameters: Vec::new(),
                });
                continue;
            }
        }

        let Some(block) = current_block.as_mut() else {
            continue;
        };
        if line.is_empty() {
            // A blank line is an empty parameter only where the keyword takes lines literally.
            if let Some(left) = literal.as_mut().filter(|l| **l > 0) {
                block.parameters.push(String::new());
                *left -= 1;
            }
            continue;
        }
        // Lines starting with obvious comment markers are never parameters.
        if line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        match literal.as_mut() {
            Some(0) => {} // free text between blocks
            Some(left) => {
                block.parameters.push(line.to_string());
                *left -= 1;
            }
            None => block.parameters.push(line.to_string()),
        }
    }

    if let Some(last) = current_block {
        blocks.push(last);
    }

    blocks
}
