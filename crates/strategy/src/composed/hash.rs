//! Content-hash canonicalization for composed strategy configs (T506).
//!
//! The hash must be **byte-stable across runs** — the same TOML produces the
//! same 32-byte SHA-256 regardless of Rust version, platform, or execution
//! order.  Rules:
//! - Indicator nodes are visited in depth-first, deterministic order derived
//!   from the AST structure (not TOML-parse order, which is already embedded
//!   in the AST).
//! - BTreeMap params are sorted by key (guaranteed by BTreeMap iteration order).
//! - Fixed separators between fields.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use smol_str::SmolStr;

use super::ast::*;

/// Compute the SHA-256 content hash of a strategy config.
///
/// The hash covers: strategy id, the rule AST, and the params map.
/// It does NOT cover symbol, stage, or size so that those fields can change
/// without invalidating the signal logic hash.
pub fn compute_config_hash(
    id: &SmolStr,
    rule: &RuleAst,
    params: &BTreeMap<SmolStr, Decimal>,
) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // 1. Strategy id.
    hasher.update(b"id:");
    hasher.update(id.as_bytes());
    hasher.update(b"\n");

    // 2. Rule AST — depth-first serialization.
    hash_rule(&mut hasher, rule);

    // 3. Params — BTreeMap iteration is always lexicographic by key.
    hasher.update(b"params:");
    for (k, v) in params {
        hasher.update(k.as_bytes());
        hasher.update(b"=");
        hasher.update(v.to_string().as_bytes());
        hasher.update(b";");
    }
    hasher.update(b"\n");

    hasher.finalize().into()
}

fn hash_rule(h: &mut Sha256, rule: &RuleAst) {
    match rule {
        RuleAst::And(a, b) => {
            h.update(b"AND(");
            hash_rule(h, a);
            h.update(b",");
            hash_rule(h, b);
            h.update(b")");
        }
        RuleAst::Or(a, b) => {
            h.update(b"OR(");
            hash_rule(h, a);
            h.update(b",");
            hash_rule(h, b);
            h.update(b")");
        }
        RuleAst::Not(inner) => {
            h.update(b"NOT(");
            hash_rule(h, inner);
            h.update(b")");
        }
        RuleAst::Cmp { op, lhs, rhs } => {
            h.update(b"CMP(");
            h.update(cmp_op_str(op).as_bytes());
            h.update(b",");
            hash_expr(h, lhs);
            h.update(b",");
            hash_expr(h, rhs);
            h.update(b")");
        }
        RuleAst::CrossAbove { a, b } => {
            h.update(b"CROSS_ABOVE(");
            hash_expr(h, a);
            h.update(b",");
            hash_expr(h, b);
            h.update(b")");
        }
        RuleAst::CrossBelow { a, b } => {
            h.update(b"CROSS_BELOW(");
            hash_expr(h, a);
            h.update(b",");
            hash_expr(h, b);
            h.update(b")");
        }
        RuleAst::MacdCross { fast, slow, signal, direction } => {
            h.update(b"MACD_CROSS(");
            h.update(fast.to_string().as_bytes());
            h.update(b",");
            h.update(slow.to_string().as_bytes());
            h.update(b",");
            h.update(signal.to_string().as_bytes());
            h.update(b",");
            h.update(match direction {
                CrossDir::Above => b"above".as_ref(),
                CrossDir::Below => b"below".as_ref(),
            });
            h.update(b")");
        }
        RuleAst::BollingerLowerTouch { period, mult } => {
            h.update(b"BB_LOWER_TOUCH(");
            h.update(period.to_string().as_bytes());
            h.update(b",");
            h.update(mult.to_string().as_bytes());
            h.update(b")");
        }
    }
}

fn hash_expr(h: &mut Sha256, expr: &Expr) {
    match expr {
        Expr::Indicator(call) => {
            h.update(b"IND(");
            h.update(call.name.as_bytes());
            for arg in &call.args {
                h.update(b",");
                hash_expr(h, arg);
            }
            h.update(b")");
        }
        Expr::BarField(field) => {
            h.update(b"FIELD(");
            h.update(field.as_bytes());
            h.update(b")");
        }
        Expr::Param(name) => {
            h.update(b"PARAM(");
            h.update(name.as_bytes());
            h.update(b")");
        }
        Expr::Literal(d) => {
            h.update(b"LIT(");
            h.update(d.to_string().as_bytes());
            h.update(b")");
        }
        Expr::Binary { op, lhs, rhs } => {
            h.update(b"BIN(");
            h.update(arith_op_str(op).as_bytes());
            h.update(b",");
            hash_expr(h, lhs);
            h.update(b",");
            hash_expr(h, rhs);
            h.update(b")");
        }
        Expr::BoolRule(inner) => {
            h.update(b"BOOLRULE(");
            hash_rule(h, inner);
            h.update(b")");
        }
    }
}

fn cmp_op_str(op: &CmpOp) -> &'static str {
    match op {
        CmpOp::Lt => "lt",
        CmpOp::Le => "le",
        CmpOp::Eq => "eq",
        CmpOp::Ge => "ge",
        CmpOp::Gt => "gt",
        CmpOp::Ne => "ne",
    }
}

fn arith_op_str(op: &ArithOp) -> &'static str {
    match op {
        ArithOp::Add => "add",
        ArithOp::Sub => "sub",
        ArithOp::Mul => "mul",
        ArithOp::Div => "div",
    }
}
