//! String-stack operators of the script language (`$…`).
//!
//! Stack effects are taken from usage in the OMSI corpus (`osc.md`): the length operators take
//! `string n`, `$IntToStrEnh` takes `number format`, `$*` takes `string count`.

use super::compiler::StrOp;

/// Both operand stacks of a script run.
#[derive(Debug, Default)]
pub struct Stacks {
    /// Float stack.
    pub numbers: Vec<f32>,
    /// String stack.
    pub strings: Vec<String>,
    /// The registers `s0`… / `l0`…: shared by a block and every macro it calls (macros
    /// take their arguments in registers, `(L.L.x) s1 … (M.L.druckausgleich)`).
    pub regs: [f32; super::compiler::REGISTERS],
}

/// Keeps the first `n` characters, padding with spaces on the right (`$SetLengthL`).
fn left(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();
    let len = out.chars().count();
    out.extend(std::iter::repeat_n(' ', n - len));
    out
}

/// Keeps the last `n` characters, padding with spaces on the left (`$SetLengthR`).
fn right(s: &str, n: usize) -> String {
    let len = s.chars().count();
    if len >= n {
        s.chars().skip(len - n).collect()
    } else {
        format!("{}{s}", " ".repeat(n - len))
    }
}

/// Centres `s` in `n` characters, cutting from both ends when it is longer (`$SetLengthC`).
fn centre(s: &str, n: usize) -> String {
    let len = s.chars().count();
    if len >= n {
        let skip = (len - n) / 2;
        return s.chars().skip(skip).take(n).collect();
    }
    let pad = n - len;
    let l = pad / 2;
    format!("{}{s}{}", " ".repeat(l), " ".repeat(pad - l))
}

/// `$IntToStrEnh`: the format is a pad character followed by the width (`" 3"`, `"02"`).
fn int_enh(value: f32, format: &str) -> String {
    let n = value.trunc() as i64;
    let mut chars = format.chars();
    let pad = chars.next().unwrap_or(' ');
    let width: usize = chars.as_str().trim().parse().unwrap_or(0);
    let digits = n.unsigned_abs().to_string();
    let sign = if n < 0 { "-" } else { "" };
    let fill = width.saturating_sub(digits.len() + sign.len());
    if pad == '0' {
        format!("{sign}{}{digits}", "0".repeat(fill))
    } else {
        format!("{}{sign}{digits}", pad.to_string().repeat(fill))
    }
}

fn truth(b: bool) -> f32 {
    f32::from(u8::from(b))
}

/// Executes `op` on `st`. Missing operands are substituted (`0` / `""`) and reported through
/// `underflow`.
pub fn exec(op: StrOp, st: &mut Stacks, messages: &mut Vec<String>, underflow: &mut bool) {
    let mut num = |st: &mut Stacks| {
        st.numbers.pop().unwrap_or_else(|| {
            *underflow = true;
            0.0
        })
    };
    let count = |v: f32| v.max(0.0) as usize;
    match op {
        StrOp::Dup => {
            let top = st.strings.last().cloned().unwrap_or_default();
            st.strings.push(top);
        }
        StrOp::Concat | StrOp::Eq | StrOp::Lt | StrOp::Gt | StrOp::Le | StrOp::Ge => {
            let b = st.strings.pop().unwrap_or_default();
            let a = st.strings.pop().unwrap_or_default();
            match op {
                StrOp::Concat => st.strings.push(a + &b),
                StrOp::Eq => st.numbers.push(truth(a == b)),
                StrOp::Lt => st.numbers.push(truth(a < b)),
                StrOp::Gt => st.numbers.push(truth(a > b)),
                StrOp::Le => st.numbers.push(truth(a <= b)),
                _ => st.numbers.push(truth(a >= b)),
            }
        }
        StrOp::Repeat => {
            let n = count(num(st));
            let s = st.strings.pop().unwrap_or_default();
            st.strings.push(s.repeat(n));
        }
        StrOp::Length => {
            let s = st.strings.pop().unwrap_or_default();
            st.numbers.push(s.chars().count() as f32);
        }
        StrOp::SetLengthL | StrOp::SetLengthC | StrOp::SetLengthR => {
            let n = count(num(st));
            let s = st.strings.pop().unwrap_or_default();
            st.strings.push(match op {
                StrOp::SetLengthL => left(&s, n),
                StrOp::SetLengthC => centre(&s, n),
                _ => right(&s, n),
            });
        }
        StrOp::CutBegin | StrOp::CutEnd => {
            let n = count(num(st));
            let s = st.strings.pop().unwrap_or_default();
            let len = s.chars().count();
            st.strings.push(if op == StrOp::CutBegin {
                s.chars().skip(n).collect()
            } else {
                s.chars().take(len.saturating_sub(n)).collect()
            });
        }
        StrOp::RemoveSpaces => {
            let s = st.strings.pop().unwrap_or_default();
            st.strings.push(s.replace(' ', ""));
        }
        StrOp::IntToStr => {
            let v = num(st);
            st.strings.push((v.trunc() as i64).to_string());
        }
        StrOp::IntToStrEnh => {
            let format = st.strings.pop().unwrap_or_default();
            let v = num(st);
            st.strings.push(int_enh(v, &format));
        }
        StrOp::StrToFloat => {
            let s = st.strings.pop().unwrap_or_default();
            st.numbers.push(s.trim().parse().unwrap_or(0.0));
        }
        StrOp::Msg => messages.push(st.strings.pop().unwrap_or_default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_length_keeps_first_or_last_characters() {
        assert_eq!(left("abcdef", 2), "ab");
        assert_eq!(left("ab", 4), "ab  ");
        assert_eq!(right("abcdef", 1), "f");
        assert_eq!(right("ab", 4), "  ab");
        assert_eq!(centre("ab", 6), "  ab  ");
        assert_eq!(centre("abcde", 3), "bcd");
    }

    #[test]
    fn int_to_str_enh_pads_with_the_format_character() {
        assert_eq!(int_enh(7.0, "02"), "07");
        assert_eq!(int_enh(7.0, " 3"), "  7");
        assert_eq!(int_enh(-7.9, "03"), "-07");
        assert_eq!(int_enh(12345.0, "02"), "12345");
    }
}
