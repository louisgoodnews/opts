//! the operator and word table of the OSC language.

use super::*;

pub(super) fn word_op(word: &str) -> Option<Op> {
    use BinOp as B;
    use StrOp as S;
    use UnOp as U;
    let bytes = word.as_bytes();
    // Registers `s0`…`s31` / `l0`…`l31` (the corpus uses up to `s9`; the documented eight are
    // not the limit of OMSI's scripts).
    if (bytes.len() == 2 || bytes.len() == 3)
        && (bytes[0] == b's' || bytes[0] == b'l')
        && bytes[1..].iter().all(u8::is_ascii_digit)
    {
        if let Ok(r) = word[1..].parse::<u8>() {
            if r < REGISTERS as u8 {
                return Some(if bytes[0] == b's' {
                    Op::StoreReg(r)
                } else {
                    Op::LoadReg(r)
                });
            }
        }
    }
    Some(match word {
        "+" => Op::Bin(B::Add),
        "-" => Op::Bin(B::Sub),
        "*" => Op::Bin(B::Mul),
        "/" => Op::Bin(B::Div),
        "%" => Op::Bin(B::Mod),
        "min" => Op::Bin(B::Min),
        "max" => Op::Bin(B::Max),
        ">" => Op::Bin(B::Gt),
        "<" => Op::Bin(B::Lt),
        "=" => Op::Bin(B::Eq),
        ">=" => Op::Bin(B::Ge),
        "<=" => Op::Bin(B::Le),
        "&&" => Op::Bin(B::And),
        "||" => Op::Bin(B::Or),
        "!" => Op::Un(U::Not),
        "abs" => Op::Un(U::Abs),
        "sqr" => Op::Un(U::Sqr),
        "sqrt" => Op::Un(U::Sqrt),
        "exp" => Op::Un(U::Exp),
        "trunc" => Op::Un(U::Trunc),
        "/-/" => Op::Un(U::Neg),
        "sin" => Op::Un(U::Sin),
        "sgn" => Op::Un(U::Sgn),
        "cos" => Op::Un(U::Cos),
        "tan" => Op::Un(U::Tan),
        "arcsin" => Op::Un(U::Asin),
        "arccos" => Op::Un(U::Acos),
        "arctan" => Op::Un(U::Atan),
        "d" => Op::Dup,
        "pi" => Op::PushPi,
        "random" => Op::Random,
        "$+" => Op::Str(S::Concat),
        "$=" => Op::Str(S::Eq),
        "$<" => Op::Str(S::Lt),
        "$>" => Op::Str(S::Gt),
        "$<=" => Op::Str(S::Le),
        // `$=>` is a frequent typo of `$>=` in add-on scripts (52 uses); read as `$>=`.
        "$>=" | "$=>" => Op::Str(S::Ge),
        "$d" => Op::Str(S::Dup),
        "$*" => Op::Str(S::Repeat),
        "$length" => Op::Str(S::Length),
        "$setlengthl" => Op::Str(S::SetLengthL),
        // `$SetLengthM` (mittig) appears in the BVG ATRON scripts: the centred variant.
        "$setlengthc" | "$setlengthm" => Op::Str(S::SetLengthC),
        "$setlengthr" => Op::Str(S::SetLengthR),
        "$removespaces" => Op::Str(S::RemoveSpaces),
        "$inttostr" => Op::Str(S::IntToStr),
        "$inttostrenh" => Op::Str(S::IntToStrEnh),
        "$strtofloat" => Op::Str(S::StrToFloat),
        "$cutbegin" => Op::Str(S::CutBegin),
        "$cutend" => Op::Str(S::CutEnd),
        "$msg" => Op::Str(S::Msg),
        _ => return None,
    })
}
