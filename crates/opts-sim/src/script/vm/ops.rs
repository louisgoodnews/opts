//! pure arithmetic of the OSC operators: truth values, binary and unary operators.

use super::*;

pub(super) fn truth(b: bool) -> f32 {
    f32::from(u8::from(b))
}

pub(super) fn binary(op: BinOp, a: f32, b: f32) -> f32 {
    match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => {
            if b == 0.0 {
                0.0
            } else {
                a / b
            }
        }
        BinOp::Mod => {
            if b == 0.0 {
                0.0
            } else {
                a % b
            }
        }
        BinOp::Min => a.min(b),
        BinOp::Max => a.max(b),
        BinOp::Gt => truth(a > b),
        BinOp::Lt => truth(a < b),
        BinOp::Eq => truth(a == b),
        BinOp::Ge => truth(a >= b),
        BinOp::Le => truth(a <= b),
        BinOp::And => truth(a != 0.0 && b != 0.0),
        BinOp::Or => truth(a != 0.0 || b != 0.0),
    }
}

pub(super) fn unary(op: UnOp, x: f32) -> f32 {
    match op {
        UnOp::Not => truth(x == 0.0),
        UnOp::Abs => x.abs(),
        UnOp::Sqr => x * x,
        UnOp::Sqrt => x.max(0.0).sqrt(),
        UnOp::Exp => x.exp(),
        UnOp::Trunc => x.trunc(),
        UnOp::Neg => -x,
        UnOp::Sin => x.sin(),
        UnOp::Cos => x.cos(),
        UnOp::Tan => x.tan(),
        UnOp::Asin => x.clamp(-1.0, 1.0).asin(),
        UnOp::Acos => x.clamp(-1.0, 1.0).acos(),
        UnOp::Atan => x.atan(),
        UnOp::Sgn => {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            }
        }
    }
}
