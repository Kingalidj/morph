use std::{cmp, fmt, ops, ops::Range, str::FromStr};

use logos::Logos;
use paste::paste;

use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

pub type NumType = Decimal;

fn decimal<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Option<Decimal> {
    Decimal::from_str(lex.slice()).ok()
}

macro_rules! num {
    ($e: expr) => {
        dec!($e)
    };
}

pub(crate) use num;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f]+")]
#[logos(subpattern unicode_ident = r"\p{XID_Start}\p{XID_Continue}*")]
#[logos(subpattern ascii_ident = r"[_a-zA-Z][_0-9a-zA-Z]*")]
pub enum Token<'a> {
    #[token("*")]
    Mul,
    #[token("/")]
    Div,
    #[token("+")]
    Add,
    #[token("-")]
    Sub,
    #[token("^")]
    Pow,
    #[token("=")]
    Assign,
    #[token("+=")]
    AddAssign,
    #[token("-=")]
    SubAssign,
    #[token("*=")]
    MulAssign,
    #[token("/=")]
    DivAssign,
    #[token("^=")]
    PowAssign,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LCurly,
    #[token("}")]
    RCurly,

    #[regex("def")]
    Def,
    #[regex("if")]
    If,
    #[regex("else")]
    Else,

    #[regex("(?&unicode_ident)", |lex| lex.slice())]
    Unit(&'a str),

    // #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
    #[regex(r"(([0-9]+)(\.[0-9]+))", decimal)]
    #[regex("[0-9]+", decimal)]
    Num(NumType),

    #[regex(r";|\n")]
    NL,

    LexErr(&'a str),
}

impl<'a> Token<'a> {
    pub const NUM: Self = Self::Num(num!(0));
    pub const UNIT: Self = Self::Unit("...");
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Token::*;

        let res = match self {
            Mul => "*",
            Div => "/",
            Add => "+",
            Sub => "-",
            Pow => "^",
            Assign => "=",
            AddAssign => "+=",
            SubAssign => "-=",
            MulAssign => "*=",
            DivAssign => "/=",
            PowAssign => "^=",
            LParen => "(",
            RParen => ")",
            LCurly => "{",
            RCurly => "}",
            Def => "def",
            If => "if",
            Else => "else",
            Unit(_) => "UNIT",
            Num(_) => "NUM",
            NL => r"(\n or ;)",
            LexErr(msg) => return write!(f, "Lexer Error: {msg}"),
        };

        write!(f, "{}", res)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum NodeType<'a> {
    Def(&'a str),

    Add(Box<Node<'a>>, Box<Node<'a>>),
    Sub(Box<Node<'a>>, Box<Node<'a>>),
    Mul(Box<Node<'a>>, Box<Node<'a>>),
    Div(Box<Node<'a>>, Box<Node<'a>>),
    Unit(&'a str),
    Num(NumType),

    Assign(&'a str, Box<Node<'a>>),
    AddAssign(&'a str, Box<Node<'a>>),
    SubAssign(&'a str, Box<Node<'a>>),
    MulAssign(&'a str, Box<Node<'a>>),
    DivAssign(&'a str, Box<Node<'a>>),

    Scope(Vec<Node<'a>>),

    Err,
}

pub fn merge_ranges(r1: &Range<usize>, r2: &Range<usize>) -> Range<usize> {
    let mut smaller = r1;
    let mut bigger = r2;
    if r2.start < r1.start {
        smaller = r2;
        bigger = r1;
    }

    smaller.start..bigger.end
}

#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub typ: NodeType<'a>,
    pub range: Range<usize>,
}

impl<'a> Node<'a> {
    pub fn new<I: Into<Range<usize>>>(typ: NodeType<'a>, range: I) -> Self {
        Self {
            typ,
            range: range.into(),
        }
    }

    pub fn err<I: Into<Range<usize>>>(range: I) -> Self {
        Self {
            typ: NodeType::Err,
            range: range.into(),
        }
    }

    #[allow(dead_code)]
    pub fn assign(&mut self, other: Node<'a>) {
        if let NodeType::Unit(name) = self.typ {
            let range = merge_ranges(&self.range, &other.range);
            let typ = NodeType::Assign(name, other.into());
            *self = Node { typ, range };
        } else {
            panic!("You can only Assign to the Node::Unit enum");
        }
    }
}

impl<'a> From<Node<'a>> for NodeType<'a> {
    fn from(n: Node<'a>) -> Self {
        n.typ
    }
}

impl<'a> fmt::Display for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use NodeType::*;

        match &self.typ {
            Def(name) => write!(f, "(def {})", name),
            Add(left, right) => write!(f, "({} + {})", left, right),
            Sub(left, right) => write!(f, "({} - {})", left, right),
            Mul(left, right) => write!(f, "({} * {})", left, right),
            Div(left, right) => write!(f, "({} / {})", left, right),
            Unit(unit) => write!(f, "{}", unit),
            Num(num_type) => write!(f, "{}", num_type),
            Assign(name, val) => write!(f, "({} = {})", name, val),
            AddAssign(name, val) => write!(f, "({} += {})", name, val),
            SubAssign(name, val) => write!(f, "({} -= {})", name, val),
            MulAssign(name, val) => write!(f, "({} *= {})", name, val),
            DivAssign(name, val) => write!(f, "({} /= {})", name, val),
            Scope(nodes) => {
                writeln!(f, "{{")?;
                for n in nodes {
                    writeln!(f, "{}", n)?;
                }
                write!(f, "}}")
            }
            Err => write!(f, "Error"),
        }
    }
}

impl<'a> std::cmp::PartialEq for Node<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.typ == other.typ
    }
}

macro_rules! impl_node_op {
    (binop: $op: ident) => {
        impl<'a> std::ops::$op<Node<'a>> for Node<'a> {
            type Output = Node<'a>;

            paste! {
                fn [<$op:snake>](self, rhs: Node<'a>) -> Self::Output {
                    let range = merge_ranges(&self.range, &rhs.range);
                    let typ = NodeType::$op(self.into(), rhs.into());
                    Node {typ, range}
                }
            }
        }
    };

    (assign: $op: ident) => {
        impl<'a> std::ops::$op<Node<'a>> for Node<'a> {
            paste! {
                fn [<$op:snake>](&mut self, other: Self) {
                    // Self::Output::$op(self.into(), rhs.into())
                    if let NodeType::Unit(name) = self.typ {
                        let range = merge_ranges(&self.range, &other.range);
                        let typ = NodeType::$op(name, other.into());
                        *self = Node {typ, range}
                    } else {
                        panic!("You can only {} to the Node::Unit enum", stringify!($op))
                    }
                }
            }
        }
    };
}

impl_node_op!(binop: Mul);
impl_node_op!(binop: Div);
impl_node_op!(binop: Add);
impl_node_op!(binop: Sub);

impl_node_op!(assign: AddAssign);
impl_node_op!(assign: SubAssign);
impl_node_op!(assign: MulAssign);
impl_node_op!(assign: DivAssign);

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct UnitAtom<'a> {
    name: &'a str,
    exp: Decimal,
}

impl<'a> UnitAtom<'a> {
    pub fn base(name: &'a str) -> Self {
        Self { name, exp: num!(1) }
    }
}

impl<'a> fmt::Display for UnitAtom<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.exp.is_integer() && self.exp == num!(1) {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}^{}", self.name, self.exp)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Unit<'a>(Vec<UnitAtom<'a>>);

impl<'a> Unit<'a> {
    pub fn none() -> Self {
        Unit(vec![])
    }
}

impl<'a> From<UnitAtom<'a>> for Unit<'a> {
    fn from(value: UnitAtom<'a>) -> Self {
        Unit(vec![value])
    }
}

impl<'a> fmt::Display for Unit<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }

        write!(f, "[")?;

        if let Some((last, rest)) = self.0.split_last() {
            for u in rest {
                write!(f, "{} ", u)?;
            }

            write!(f, "{}]", last)?;
        }

        Ok(())
    }
}

impl<'a> ops::Mul for Unit<'a> {
    type Output = Unit<'a>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut merged: Vec<_> = Vec::new();

        for u in self.0 {
            merged.push(u);
        }

        for u in rhs.0 {
            merged.push(u);
        }

        merged.sort_by(|a, b| a.name.cmp(b.name));

        let mut current: Option<UnitAtom> = None;

        let mut result: Vec<_> = Vec::new();

        for u in merged {
            if let Some(mut c) = current {
                if c.name == u.name {
                    c.exp += u.exp;
                    current = Some(c);
                } else {
                    result.push(c);
                    current = Some(u);
                }
            } else {
                current = Some(u)
            }
        }

        if let Some(c) = current {
            result.push(c);
        }

        result.retain(|unit| !unit.exp.is_zero());
        result.sort_by(|a, _| match a.exp.is_sign_negative() {
            true => cmp::Ordering::Greater,
            false => cmp::Ordering::Less,
        });

        Unit(result)
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl<'a> ops::Div for Unit<'a> {
    type Output = Unit<'a>;

    fn div(self, rhs: Self) -> Self::Output {
        let mut res = rhs.clone();
        res.0.iter_mut().for_each(|u| u.exp *= num!(-1));

        self * res
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Quantity<'a> {
    value: Decimal,
    unit: Unit<'a>,
}

impl<'a> Quantity<'a> {
    pub fn new<I: Into<Decimal>>(value: I, unit: Unit<'a>) -> Self {
        Self {
            value: value.into(),
            unit,
        }
    }

    pub fn num<I: Into<Decimal>>(value: I) -> Self {
        Self {
            value: value.into(),
            unit: Unit::none(),
        }
    }
}

impl<'a> From<UnitAtom<'a>> for Quantity<'a> {
    fn from(unit: UnitAtom<'a>) -> Self {
        Quantity {
            value: num!(1),
            unit: unit.into(),
        }
    }
}

impl<'a> From<Unit<'a>> for Quantity<'a> {
    fn from(unit: Unit<'a>) -> Self {
        Quantity {
            value: num!(1),
            unit,
        }
    }
}

impl<'a> fmt::Display for Quantity<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.unit)
    }
}

impl<'a> ops::Add for Quantity<'a> {
    type Output = Option<Quantity<'a>>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut res = self.clone();

        if self.unit == rhs.unit {
            res.value += rhs.value;
            Some(res)
        } else {
            None
        }
    }
}

impl<'a> ops::Sub for Quantity<'a> {
    type Output = Option<Quantity<'a>>;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut res = rhs.clone();
        res.value *= num!(-1);
        self + res
    }
}

impl<'a> ops::Mul for Quantity<'a> {
    type Output = Option<Quantity<'a>>;

    fn mul(self, rhs: Self) -> Self::Output {
        let res = self.unit * rhs.unit;
        Some(Quantity::new(self.value * rhs.value, res))
    }
}

impl<'a> ops::Div for Quantity<'a> {
    type Output = Option<Quantity<'a>>;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.value.is_zero() {
            None
        } else {
            let res = self.unit / rhs.unit;
            Some(Quantity::new(self.value / rhs.value, res))
        }
    }
}
