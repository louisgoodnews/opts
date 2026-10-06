# Documentation Standards

This chapter outlines the documentation conventions, formatting standards, and encoding requirements for OPTS.

---

## 🔤 Character Encoding

All documentation files (`*.md`) in the OPTS repository must strictly be encoded in **UTF-8 without BOM** using Unix line endings (`LF`).

### Preventing Mojibake
Under no circumstances should Markdown files be opened, edited, or saved using legacy single-byte encodings (such as Windows-1252/CP1252, CP866, or CP1251). Doing so corrupts multi-byte UTF-8 sequences (such as em dashes `—` and emojis `🟢`, `✅`, `❓`) into mojibake gibberish (`тАФ`, `ЁЯЯв`, `â€`).

### Recommended EditorConfig
Contributors should ensure their editors enforce UTF-8 across all documentation files:

```ini
[*.md]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true
```

### Pre-Commit / CI Encoding Check
To verify that all Markdown files are properly encoded in UTF-8:

```bash
find docs/ -name "*.md" -exec sh -c 'file "$1" | grep -q "UTF-8" || echo "Not UTF-8: $1"' _ {} \;
```

---

## 📝 Markdown Structure & Style

- **Headings:** Use ATX-style headings (`#`, `##`, `###`). Avoid Setext-style underlines.
- **Heading Hierarchy:** Each file must have exactly one top-level `#` title heading.
- **Emojis:** Top-level section headings in specification documents and architectural guides should use standard status and topic emojis (`📖 Overview`, `🔤 General Structure`, `📚 Field Reference`, `🧪 Examples`, `❓ Open Questions`, `📝 Provenance`, `🔗 Related Documents`).
- **Status Badges:** Format specification and milestone documents should include metadata tables indicating status (`🟢 Specified`, `🟡 Draft`, `🔴 Blocked`).
- **Provenance Attestation:** Every format specification in `docs/src/formats/` must include a clean-room attestation in compliance with [ADR-0002](../adr/0002-clean-room-policy.md).

