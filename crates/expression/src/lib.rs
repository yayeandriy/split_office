//! Expression engine for Split Office.
//!
//! # Phase 2 — Transformation Engine (Spec §expression)
//!
//! Provides parsing, validation, and evaluation of algebraic expressions
//! over dataset columns. Used by:
//! - Derived columns (`Profit = Revenue - Cost`)
//! - Filter predicates (`country == "US"`)
//! - Aggregation expressions (`sum(revenue)`)
//!
//! # Architecture
//!
//! ```text
//! String → Tokenize → Parse → AST → Validate → Evaluate
//! ```
//!
//! # Expression grammar (BNF-like)
//!
//! ```text
//! expr     → term (('+' | '-') term)*
//! term     → factor (('*' | '/') factor)*
//! factor   → unary | '(' expr ')' | literal | column_ref | function_call
//! unary    → '-' factor
//! literal  → INTEGER | FLOAT | STRING | BOOLEAN
//! column_ref → IDENTIFIER ('.' IDENTIFIER)*
//! function_call → IDENTIFIER '(' expr (',' expr)* ')'
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

// ── AST ──────────────────────────────────────────────────────────────────────

/// A parsed expression tree.
///
/// Spec §expression: expression trees that can be validated and evaluated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Integer literal: `42`
    Int(i64),
    /// Float literal: `3.14`
    Float(f64),
    /// String literal: `"hello"`
    String(String),
    /// Boolean literal: `true` / `false`
    Bool(bool),
    /// Null literal: `null`
    Null,
    /// Column reference: `revenue`, `price`
    Column(String),
    /// Binary arithmetic: `a + b`, `x / y`
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Unary operation: `-x`, `!flag`
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    /// Function call: `sum(revenue)`, `upper(name)`
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Neq => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Le => write!(f, "<="),
            BinOp::Gt => write!(f, ">"),
            BinOp::Ge => write!(f, ">="),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
        }
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Int(v) => write!(f, "{}", v),
            Expr::Float(v) => write!(f, "{}", v),
            Expr::String(v) => write!(f, "\"{}\"", v),
            Expr::Bool(v) => write!(f, "{}", v),
            Expr::Null => write!(f, "null"),
            Expr::Column(name) => write!(f, "{}", name),
            Expr::Binary { op, left, right } => write!(f, "({} {} {})", left, op, right),
            Expr::Unary { op, expr } => write!(f, "({}{})", op, expr),
            Expr::Call { name, args } => {
                let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "{}({})", name, args_str.join(", "))
            }
        }
    }
}

// ── Parsing ──────────────────────────────────────────────────────────────────

/// Parse an expression string into an AST.
///
/// Returns `Err` with a human-readable message on syntax errors.
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expr()?;
    if !parser.is_done() {
        return Err(ParseError::UnexpectedToken {
            pos: parser.pos(),
            expected: "end of expression".into(),
            found: parser.current().map(|t| t.to_string()).unwrap_or_default(),
        });
    }
    Ok(expr)
}

// ── Tokenizer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Int(i64),
    Float(f64),
    String(String),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    EqEq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    LParen,
    RParen,
    Comma,
    True,
    False,
    Null,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Int(v) => write!(f, "{}", v),
            Token::Float(v) => write!(f, "{}", v),
            Token::String(v) => write!(f, "\"{}\"", v),
            Token::Ident(v) => write!(f, "{}", v),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::EqEq => write!(f, "=="),
            Token::Neq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Le => write!(f, "<="),
            Token::Gt => write!(f, ">"),
            Token::Ge => write!(f, ">="),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Not => write!(f, "!"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Null => write!(f, "null"),
        }
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Skip whitespace.
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // String literal.
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
                s.push(chars[i]);
                i += 1;
            }
            if i >= chars.len() {
                return Err(ParseError::UnterminatedString { pos: i });
            }
            i += 1; // skip closing quote
            tokens.push(Token::String(s));
            continue;
        }

        // Number literal.
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            let start = i;
            let mut is_float = false;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                if chars[i] == '.' {
                    if is_float {
                        return Err(ParseError::InvalidNumber { pos: i });
                    }
                    is_float = true;
                }
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            if is_float {
                tokens.push(Token::Float(num_str.parse().map_err(|_| ParseError::InvalidNumber { pos: start })?));
            } else {
                tokens.push(Token::Int(num_str.parse().map_err(|_| ParseError::InvalidNumber { pos: start })?));
            }
            continue;
        }

        // Multi-char operators.
        if i + 1 < chars.len() {
            let two = format!("{}{}", c, chars[i + 1]);
            match two.as_str() {
                "==" => { tokens.push(Token::EqEq); i += 2; continue; }
                "!=" => { tokens.push(Token::Neq); i += 2; continue; }
                "<=" => { tokens.push(Token::Le); i += 2; continue; }
                ">=" => { tokens.push(Token::Ge); i += 2; continue; }
                "&&" => { tokens.push(Token::And); i += 2; continue; }
                "||" => { tokens.push(Token::Or); i += 2; continue; }
                _ => {}
            }
        }

        // Single-char tokens and identifiers.
        match c {
            '+' => { tokens.push(Token::Plus); i += 1; }
            '-' => { tokens.push(Token::Minus); i += 1; }
            '*' => { tokens.push(Token::Star); i += 1; }
            '/' => { tokens.push(Token::Slash); i += 1; }
            '<' => { tokens.push(Token::Lt); i += 1; }
            '>' => { tokens.push(Token::Gt); i += 1; }
            '!' => { tokens.push(Token::Not); i += 1; }
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            ',' => { tokens.push(Token::Comma); i += 1; }
            _ if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                match ident.as_str() {
                    "true" => tokens.push(Token::True),
                    "false" => tokens.push(Token::False),
                    "null" => tokens.push(Token::Null),
                    _ => tokens.push(Token::Ident(ident)),
                }
            }
            _ => return Err(ParseError::UnexpectedChar { pos: i, ch: c }),
        }
    }
    Ok(tokens)
}

// ── Parser ───────────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn is_done(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: &str) -> Result<Token, ParseError> {
        match self.current() {
            Some(t) => {
                let t = t.clone();
                self.pos += 1;
                Ok(t)
            }
            None => Err(ParseError::UnexpectedToken {
                pos: self.pos,
                expected: expected.into(),
                found: "EOF".into(),
            }),
        }
    }

    /// expr → term (('+' | '-') term)*
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_term()?;

        while let Some(op) = self.current() {
            match op {
                Token::Plus => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Add, left: Box::new(left), right: Box::new(right) };
                }
                Token::Minus => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Sub, left: Box::new(left), right: Box::new(right) };
                }
                Token::EqEq => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Eq, left: Box::new(left), right: Box::new(right) };
                }
                Token::Neq => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Neq, left: Box::new(left), right: Box::new(right) };
                }
                Token::Lt => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Lt, left: Box::new(left), right: Box::new(right) };
                }
                Token::Le => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Le, left: Box::new(left), right: Box::new(right) };
                }
                Token::Gt => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Gt, left: Box::new(left), right: Box::new(right) };
                }
                Token::Ge => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Ge, left: Box::new(left), right: Box::new(right) };
                }
                Token::And => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::And, left: Box::new(left), right: Box::new(right) };
                }
                Token::Or => {
                    self.pos += 1;
                    let right = self.parse_term()?;
                    left = Expr::Binary { op: BinOp::Or, left: Box::new(left), right: Box::new(right) };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// term → factor (('*' | '/') factor)*
    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_factor()?;

        while let Some(op) = self.current() {
            match op {
                Token::Star => {
                    self.pos += 1;
                    let right = self.parse_factor()?;
                    left = Expr::Binary { op: BinOp::Mul, left: Box::new(left), right: Box::new(right) };
                }
                Token::Slash => {
                    self.pos += 1;
                    let right = self.parse_factor()?;
                    left = Expr::Binary { op: BinOp::Div, left: Box::new(left), right: Box::new(right) };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// factor → unary | '(' expr ')' | literal | column_ref | function_call
    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        match self.current().cloned() {
            Some(Token::Minus) => {
                self.pos += 1;
                let expr = self.parse_factor()?;
                Ok(Expr::Unary { op: UnaryOp::Neg, expr: Box::new(expr) })
            }
            Some(Token::Not) => {
                self.pos += 1;
                let expr = self.parse_factor()?;
                Ok(Expr::Unary { op: UnaryOp::Not, expr: Box::new(expr) })
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let expr = self.parse_expr()?;
                match self.current() {
                    Some(Token::RParen) => { self.pos += 1; Ok(expr) }
                    _ => Err(ParseError::ExpectedCloseParen { pos: self.pos }),
                }
            }
            Some(Token::Int(v)) => { self.pos += 1; Ok(Expr::Int(v)) }
            Some(Token::Float(v)) => { self.pos += 1; Ok(Expr::Float(v)) }
            Some(Token::String(v)) => { self.pos += 1; Ok(Expr::String(v)) }
            Some(Token::True) => { self.pos += 1; Ok(Expr::Bool(true)) }
            Some(Token::False) => { self.pos += 1; Ok(Expr::Bool(false)) }
            Some(Token::Null) => { self.pos += 1; Ok(Expr::Null) }
            Some(Token::Ident(name)) => {
                self.pos += 1;
                // Function call?
                if matches!(self.current(), Some(Token::LParen)) {
                    self.pos += 1; // skip '('
                    let mut args = Vec::new();
                    if !matches!(self.current(), Some(Token::RParen)) {
                        args.push(self.parse_expr()?);
                        while matches!(self.current(), Some(Token::Comma)) {
                            self.pos += 1; // skip ','
                            args.push(self.parse_expr()?);
                        }
                    }
                    match self.current() {
                        Some(Token::RParen) => { self.pos += 1; }
                        _ => return Err(ParseError::ExpectedCloseParen { pos: self.pos }),
                    }
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Column(name))
                }
            }
            _ => Err(ParseError::UnexpectedToken {
                pos: self.pos,
                expected: "expression".into(),
                found: self.current().map(|t| t.to_string()).unwrap_or("EOF".into()),
            }),
        }
    }
}

// ── Validation ────────────────────────────────────────────────────────────────

/// Information about a dataset column needed for type-checking.
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    /// DuckDB/Polars type name: "Int64", "Float64", "Utf8", etc.
    pub data_type: String,
}

/// Validate an expression against a known set of columns.
///
/// Checks:
/// - All column references exist in the schema
/// - Function names are recognized
/// - Argument counts are correct
pub fn validate(expr: &Expr, columns: &[ColumnInfo]) -> Result<ExprType, ValidationError> {
    match expr {
        Expr::Column(name) => {
            columns.iter().find(|c| c.name == *name)
                .map(|c| ExprType::from_str(&c.data_type))
                .ok_or_else(|| ValidationError::UnknownColumn(name.clone()))
        }
        Expr::Int(_) => Ok(ExprType::Int),
        Expr::Float(_) => Ok(ExprType::Float),
        Expr::String(_) => Ok(ExprType::String),
        Expr::Bool(_) => Ok(ExprType::Bool),
        Expr::Null => Ok(ExprType::Null),
        Expr::Binary { op, left, right } => {
            let lt = validate(left, columns)?;
            let rt = validate(right, columns)?;
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    match (&lt, &rt) {
                        (ExprType::Int, ExprType::Int) => Ok(ExprType::Int),
                        _ => Ok(ExprType::Float), // numeric promotion
                    }
                }
                BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    Ok(ExprType::Bool)
                }
                BinOp::And | BinOp::Or => {
                    if lt != ExprType::Bool || rt != ExprType::Bool {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Bool".into(),
                            found: format!("{:?} and {:?}", lt, rt),
                        });
                    }
                    Ok(ExprType::Bool)
                }
            }
        }
        Expr::Unary { op, expr } => {
            let t = validate(expr, columns)?;
            match op {
                UnaryOp::Neg => match t {
                    ExprType::Int | ExprType::Float => Ok(t),
                    _ => Err(ValidationError::TypeMismatch {
                        expected: "numeric".into(),
                        found: format!("{:?}", t),
                    }),
                },
                UnaryOp::Not => {
                    if t != ExprType::Bool {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Bool".into(),
                            found: format!("{:?}", t),
                        });
                    }
                    Ok(ExprType::Bool)
                }
            }
        }
        Expr::Call { name, args } => {
            match name.as_str() {
                "sum" | "avg" | "min" | "max" => {
                    if args.len() != 1 {
                        return Err(ValidationError::WrongArgCount {
                            function: name.clone(),
                            expected: 1,
                            got: args.len(),
                        });
                    }
                    let t = validate(&args[0], columns)?;
                    match t {
                        ExprType::Int | ExprType::Float => Ok(t),
                        _ => Err(ValidationError::TypeMismatch {
                            expected: "numeric".into(),
                            found: format!("{:?}", t),
                        }),
                    }
                }
                "count" => Ok(ExprType::Int),
                "upper" | "lower" => {
                    if args.len() != 1 {
                        return Err(ValidationError::WrongArgCount {
                            function: name.clone(),
                            expected: 1,
                            got: args.len(),
                        });
                    }
                    Ok(ExprType::String)
                }
                "round" => {
                    if args.is_empty() || args.len() > 2 {
                        return Err(ValidationError::WrongArgCount {
                            function: name.clone(),
                            expected: 1,
                            got: args.len(),
                        });
                    }
                    Ok(ExprType::Float)
                }
                _ => Err(ValidationError::UnknownFunction(name.clone())),
            }
        }
    }
}

/// The deduced type of an expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprType {
    Int,
    Float,
    String,
    Bool,
    Null,
    Unknown,
}

impl ExprType {
    fn from_str(s: &str) -> Self {
        match s {
            "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32" | "UInt64" => ExprType::Int,
            "Float32" | "Float64" => ExprType::Float,
            "Utf8" | "LargeUtf8" => ExprType::String,
            "Boolean" => ExprType::Bool,
            "Null" => ExprType::Null,
            _ => ExprType::Unknown,
        }
    }
}

// ── Column extraction ────────────────────────────────────────────────────────

/// Collect all column references in an expression.
pub fn columns_used(expr: &Expr) -> Vec<String> {
    let mut cols = Vec::new();
    collect_columns(expr, &mut cols);
    cols.sort();
    cols.dedup();
    cols
}

fn collect_columns(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Column(name) => out.push(name.clone()),
        Expr::Binary { left, right, .. } => {
            collect_columns(left, out);
            collect_columns(right, out);
        }
        Expr::Unary { expr, .. } => collect_columns(expr, out),
        Expr::Call { args, .. } => {
            for arg in args {
                collect_columns(arg, out);
            }
        }
        _ => {}
    }
}

// ── SQL generation ───────────────────────────────────────────────────────────

/// Render an expression as a SQL fragment for DuckDB.
pub fn to_sql(expr: &Expr) -> String {
    match expr {
        Expr::Int(v) => v.to_string(),
        Expr::Float(v) => v.to_string(),
        Expr::String(v) => format!("'{}'", v.replace('\'', "''")),
        Expr::Bool(v) => if *v { "TRUE".into() } else { "FALSE".into() },
        Expr::Null => "NULL".into(),
        Expr::Column(name) => format!("\"{}\"", name),
        Expr::Binary { op, left, right } => {
            let sql_op = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Eq => "=",
                BinOp::Neq => "!=",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::And => "AND",
                BinOp::Or => "OR",
            };
            format!("({} {} {})", to_sql(left), sql_op, to_sql(right))
        }
        Expr::Unary { op, expr } => {
            match op {
                UnaryOp::Neg => format!("(-{})", to_sql(expr)),
                UnaryOp::Not => format!("(NOT {})", to_sql(expr)),
            }
        }
        Expr::Call { name, args } => {
            let args_sql: Vec<String> = args.iter().map(to_sql).collect();
            format!("{}({})", name.to_uppercase(), args_sql.join(", "))
        }
    }
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ParseError {
    UnexpectedChar { pos: usize, ch: char },
    UnexpectedToken { pos: usize, expected: String, found: String },
    ExpectedCloseParen { pos: usize },
    UnterminatedString { pos: usize },
    InvalidNumber { pos: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedChar { pos, ch } => write!(f, "Unexpected character '{}' at position {}", ch, pos),
            ParseError::UnexpectedToken { pos, expected, found } => write!(f, "Expected {} at position {}, found {}", expected, pos, found),
            ParseError::ExpectedCloseParen { pos } => write!(f, "Expected ')' at position {}", pos),
            ParseError::UnterminatedString { pos } => write!(f, "Unterminated string starting at position {}", pos),
            ParseError::InvalidNumber { pos } => write!(f, "Invalid number at position {}", pos),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub enum ValidationError {
    UnknownColumn(String),
    UnknownFunction(String),
    WrongArgCount { function: String, expected: usize, got: usize },
    TypeMismatch { expected: String, found: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::UnknownColumn(c) => write!(f, "Unknown column: {}", c),
            ValidationError::UnknownFunction(func) => write!(f, "Unknown function: {}", func),
            ValidationError::WrongArgCount { function, expected, got } => write!(f, "Function {} expects {} arguments, got {}", function, expected, got),
            ValidationError::TypeMismatch { expected, found } => write!(f, "Type mismatch: expected {}, found {}", expected, found),
        }
    }
}

impl std::error::Error for ValidationError {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Parsing ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_literal() {
        assert_eq!(parse("42").unwrap(), Expr::Int(42));
        assert_eq!(parse("3.14").unwrap(), Expr::Float(3.14));
        assert_eq!(parse("\"hello\"").unwrap(), Expr::String("hello".into()));
        assert_eq!(parse("true").unwrap(), Expr::Bool(true));
        assert_eq!(parse("null").unwrap(), Expr::Null);
    }

    #[test]
    fn test_parse_column() {
        assert_eq!(parse("revenue").unwrap(), Expr::Column("revenue".into()));
    }

    #[test]
    fn test_parse_arithmetic() {
        let expr = parse("a + b * 2").unwrap();
        // a + (b * 2)
        assert!(matches!(expr, Expr::Binary { op: BinOp::Add, .. }));
        if let Expr::Binary { right, .. } = &expr {
            assert!(matches!(**right, Expr::Binary { op: BinOp::Mul, .. }));
        }
    }

    #[test]
    fn test_parse_comparison() {
        let expr = parse("revenue > 100").unwrap();
        assert!(matches!(expr, Expr::Binary { op: BinOp::Gt, .. }));
    }

    #[test]
    fn test_parse_function_call() {
        let expr = parse("sum(revenue)").unwrap();
        assert!(matches!(expr, Expr::Call { name, .. } if name == "sum"));
    }

    #[test]
    fn test_parse_nested_parens() {
        let expr = parse("(a + b) * c").unwrap();
        assert!(matches!(expr, Expr::Binary { op: BinOp::Mul, .. }));
    }

    #[test]
    fn test_parse_error() {
        assert!(parse("revenue @@@").is_err());
    }

    // ── Display (round-trip) ─────────────────────────────────────────────

    #[test]
    fn test_display_roundtrip() {
        let inputs = vec![
            "42",
            "3.14",
            "\"hello\"",
            "revenue",
            "(revenue + cost)",
            "(revenue * 2)",
        ];
        for input in inputs {
            let expr = parse(input).unwrap();
            let rendered = expr.to_string();
            let re_parsed = parse(&rendered).unwrap();
            assert_eq!(expr, re_parsed, "round-trip failed for: {}", input);
        }
    }

    // ── Validation ───────────────────────────────────────────────────────

    fn test_columns() -> Vec<ColumnInfo> {
        vec![
            ColumnInfo { name: "revenue".into(), data_type: "Float64".into() },
            ColumnInfo { name: "cost".into(), data_type: "Float64".into() },
            ColumnInfo { name: "name".into(), data_type: "Utf8".into() },
        ]
    }

    #[test]
    fn test_validate_arithmetic() {
        let expr = parse("revenue - cost").unwrap();
        let t = validate(&expr, &test_columns()).unwrap();
        assert_eq!(t, ExprType::Float);
    }

    #[test]
    fn test_validate_comparison() {
        let expr = parse("revenue > 100").unwrap();
        let t = validate(&expr, &test_columns()).unwrap();
        assert_eq!(t, ExprType::Bool);
    }

    #[test]
    fn test_validate_unknown_column() {
        let expr = parse("missing_column * 2").unwrap();
        assert!(validate(&expr, &test_columns()).is_err());
    }

    // ── Column extraction ────────────────────────────────────────────────

    #[test]
    fn test_columns_used() {
        let expr = parse("revenue - cost + 10").unwrap();
        let cols = columns_used(&expr);
        assert!(cols.contains(&"revenue".into()));
        assert!(cols.contains(&"cost".into()));
        assert_eq!(cols.len(), 2);
    }

    // ── SQL generation ───────────────────────────────────────────────────

    #[test]
    fn test_to_sql_column() {
        let expr = parse("revenue").unwrap();
        assert_eq!(to_sql(&expr), "\"revenue\"");
    }

    #[test]
    fn test_to_sql_arithmetic() {
        let expr = parse("revenue - cost").unwrap();
        let sql = to_sql(&expr);
        assert!(sql.contains("\"revenue\""));
        assert!(sql.contains("-"));
        assert!(sql.contains("\"cost\""));
    }

    #[test]
    fn test_to_sql_function() {
        let expr = parse("sum(revenue)").unwrap();
        let sql = to_sql(&expr);
        assert!(sql.contains("SUM"));
        assert!(sql.contains("\"revenue\""));
    }
}
