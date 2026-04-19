//! Rule AST — the typed tree produced by the DSL parser.
//!
//! These types are immutable after construction; evaluation happens via
//! `node.rs` during `on_bar`.

use rust_decimal::Decimal;
use smol_str::SmolStr;

/// A comparison operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
    Ne,
}

/// An arithmetic operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Cross direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossDir {
    Above,
    Below,
}

/// Named indicator with its positional arguments.
///
/// Most indicator args are numeric literals, but `min`/`max`/`avg` accept
/// a bar field as the first argument (e.g. `min(low, 20)`).
#[derive(Debug, Clone, PartialEq)]
pub struct IndicatorCall {
    pub name: SmolStr,
    pub args: Vec<Expr>,
}

/// An expression node (Decimal-valued).
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A named indicator call, e.g. `rsi(14)`.
    Indicator(IndicatorCall),
    /// A bar-native field reference: `close`, `open`, `high`, `low`, `volume`, `trade_count`.
    BarField(SmolStr),
    /// A named parameter from `[params]`.
    Param(SmolStr),
    /// A numeric literal.
    Literal(Decimal),
    /// Binary arithmetic.
    Binary {
        op: ArithOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// A parenthesized rule used in expression position (boolean context).
    /// Evaluates to 1.0 if the rule is true, 0.0 if false.
    BoolRule(Box<RuleAst>),
}

/// A rule node (Boolean-valued).
#[derive(Debug, Clone, PartialEq)]
pub enum RuleAst {
    /// Logical AND.
    And(Box<RuleAst>, Box<RuleAst>),
    /// Logical OR.
    Or(Box<RuleAst>, Box<RuleAst>),
    /// Logical NOT.
    Not(Box<RuleAst>),
    /// Comparison: `lhs CMP_OP rhs`.
    Cmp {
        op: CmpOp,
        lhs: Expr,
        rhs: Expr,
    },
    /// Cross-above sugar: `cross_above(a, b)`.
    CrossAbove { a: Expr, b: Expr },
    /// Cross-below sugar: `cross_below(a, b)`.
    CrossBelow { a: Expr, b: Expr },
    /// MACD cross sugar: `macd_cross(fast, slow, signal)`.
    MacdCross {
        fast: u32,
        slow: u32,
        signal: u32,
        direction: CrossDir,
    },
    /// Bollinger lower-touch sugar: `bollinger_lower_touch(period, mult)`.
    BollingerLowerTouch { period: u32, mult: Decimal },
}
