//! control words (`if`, `else`, `endif`) and closing the blocks of a script.

use super::*;

impl Compiler {
    pub(super) fn control(&mut self, file: &str, line: usize, word: &str, block: &mut Open) {
        let here = block.ops.len();
        match word {
            "if" => {
                block.ops.push(Op::JumpIfZero(0));
                block.ifs.push(IfFrame {
                    jump_if_zero: here,
                    else_jump: None,
                    extra_else: Vec::new(),
                    line,
                });
            }
            "else" => match block.ifs.last_mut() {
                Some(f) if f.else_jump.is_none() => {
                    f.else_jump = Some(here);
                    let jz = f.jump_if_zero;
                    block.ops.push(Op::Jump(0));
                    block.ops[jz] = Op::JumpIfZero(here as u32 + 1);
                }
                Some(f) => {
                    // A second `{else}` (`{if} A {else} B {else} C {endif}`): OMSI's interpreter
                    // runs B, skips to `{endif}` at the second `{else}` and never runs C.
                    f.extra_else.push(here);
                    block.ops.push(Op::Jump(0));
                    self.warn(
                        file,
                        line,
                        "second {else} of one {if}: its branch never runs".into(),
                    );
                }
                None => self.warn(file, line, "{else} without a matching {if} ignored".into()),
            },
            _ => match block.ifs.pop() {
                Some(f) => {
                    for j in &f.extra_else {
                        block.ops[*j] = Op::Jump(here as u32);
                    }
                    match f.else_jump {
                        Some(j) => block.ops[j] = Op::Jump(here as u32),
                        None => block.ops[f.jump_if_zero] = Op::JumpIfZero(here as u32),
                    }
                }
                None => self.warn(file, line, "{endif} without a matching {if} ignored".into()),
            },
        }
    }

    pub(super) fn close(&mut self, file: &str, line: usize, mut block: Open) {
        // OMSI tolerates a block that ends inside open conditionals (60+ vehicle scripts do):
        // the open branches simply end with the block.
        while let Some(f) = block.ifs.pop() {
            self.warn(
                file,
                f.line,
                format!(
                    "{{if}} without {{endif}} in `{}` closed at {{end}}",
                    block.name
                ),
            );
            let end = block.ops.len() as u32;
            for j in &f.extra_else {
                block.ops[*j] = Op::Jump(end);
            }
            match f.else_jump {
                Some(j) => block.ops[j] = Op::Jump(end),
                None => block.ops[f.jump_if_zero] = Op::JumpIfZero(end),
            }
        }
        if block.failed {
            self.error(file, line, format!("block `{}` disabled", block.name));
            return;
        }
        let index = self.program.blocks.len();
        self.program.blocks.push(Block {
            name: block.name,
            file: file.to_owned(),
            ops: block.ops,
        });
        match block.kind {
            Kind::Init => self.program.init.push(index),
            Kind::Frame => self.program.frame.push(index),
            Kind::FrameAi => self.program.frame_ai.push(index),
            Kind::Macro(id) => {
                self.program.macros.insert(id, index);
            }
            Kind::Trigger(id) => self.program.triggers.entry(id).or_default().push(index),
        }
    }
}
