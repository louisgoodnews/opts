//! header lines (`{...}` words that open a block) and their bookkeeping.

use super::*;

impl Compiler {
    pub(super) fn header(&mut self, file: &str, tok: &Token, open: &mut Option<Open>) {
        let inner = tok.text[1..tok.text.len() - 1].to_lowercase();
        match inner.as_str() {
            "if" | "else" | "endif" => {
                if let Some(block) = open.as_mut() {
                    self.control(file, tok.line, &inner, block);
                } else {
                    self.warn(
                        file,
                        tok.line,
                        format!("{{{inner}}} outside any block ignored"),
                    );
                }
                return;
            }
            "end" => {
                match open.take() {
                    Some(block) => self.close(file, tok.line, block),
                    None => self.warn(file, tok.line, "{end} without a block".into()),
                }
                return;
            }
            _ => {}
        }
        if let Some(block) = open.take() {
            self.warn(
                file,
                tok.line,
                format!("block `{}` has no {{end}}", block.name),
            );
            self.close(file, tok.line, block);
        }
        let (kind, name) = match inner.split_once(':') {
            None if inner == "init" => (Kind::Init, "init".to_owned()),
            None if inner == "frame" => (Kind::Frame, "frame".to_owned()),
            None if inner == "frame_ai" => (Kind::FrameAi, "frame_ai".to_owned()),
            Some(("macro", n)) => (
                Kind::Macro(self.program.names.macros.intern(n.trim())),
                format!("macro:{}", n.trim()),
            ),
            Some(("trigger", n)) => (
                Kind::Trigger(self.program.names.events.intern(n.trim())),
                format!("trigger:{}", n.trim()),
            ),
            _ => {
                self.warn(
                    file,
                    tok.line,
                    format!("unknown block header `{}`", tok.text),
                );
                return;
            }
        };
        *open = Some(Open {
            name,
            kind,
            ops: Vec::new(),
            ifs: Vec::new(),
            failed: false,
        });
    }
}
