use crate::parser::{ArgDecl, BinOp, Block, Expr, Func, Rel, SourceFile, Statement, Type, UnaryOp};

const SHIFT_WIDTH: usize = 4;

impl SourceFile {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();

        let mut iter = self.funcs.iter();

        if let Some(func) = iter.next() {
            out.push_str(&func.pretty_print());
        }

        for func in iter {
            out.push('\n');
            out.push_str(&func.pretty_print());
        }

        out
    }
}

impl Func {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!(
            "{} {}({})\n",
            self.ret_type.pretty_print(),
            self.name,
            self.args
                .iter()
                .map(|arg| arg.pretty_print())
                .collect::<Vec<_>>()
                .join(", ")
        ));

        out.push_str(&self.blk.pretty_print(0));
        out.push('\n');

        out
    }
}

impl Type {
    pub fn pretty_print(&self) -> String {
        match self {
            Self::Void => "void".to_string(),
            Self::Int => "int".to_string(),
            Self::Bool => "bool".to_string(),
            Self::UserDef(name) => name.clone(),
        }
    }
}

impl ArgDecl {
    pub fn pretty_print(&self) -> String {
        format!("{} {}", self.r#type.pretty_print(), self.name)
    }
}

impl Statement {
    pub fn pretty_print(&self, indent: usize) -> String {
        let space = SHIFT_WIDTH * indent;

        match self {
            Self::Expr(expr) => format!("{:space$}{};", "", expr.pretty_print()),
            Self::Continue => format!("{:space$}continue;", ""),
            Self::Break => format!("{:space$}break;", ""),
            Self::Return(expr) => format!("{:space$}return {};", "", expr.pretty_print()),
            Self::Block(blk) => format!("{:space$}{}", "", blk.pretty_print(indent)),
            Self::While { cond, stmt } => {
                format!(
                    "{:space$}while ({})\n{}\n",
                    "",
                    cond.pretty_print(),
                    stmt.pretty_print(indent + 1)
                )
            }
            Self::DoWhile { stmt, cond } => format!(
                "{:space$}do {} while ({});",
                "",
                stmt.pretty_print(indent + 1),
                cond.pretty_print()
            ),
            Self::If {
                cond,
                stmt,
                else_stmt,
            } => match else_stmt {
                Some(else_stmt) => format!(
                    "{:space$}if ({})\n{} else {}\n",
                    "",
                    cond.pretty_print(),
                    stmt.pretty_print(indent + 1),
                    else_stmt.pretty_print(indent + 1)
                ),
                None => format!(
                    "{:space$}if ({})\n{}\n",
                    "",
                    cond.pretty_print(),
                    stmt.pretty_print(indent + 1)
                ),
            },
        }
    }
}

impl Expr {
    pub fn pretty_print(&self) -> String {
        match self {
            Self::Assign { src, dst } => format!("{} = {}", src.pretty_print(), dst.pretty_print()),
            Self::Rel(rel, lhs, rhs) => {
                format!(
                    "({}) {} ({})",
                    lhs.pretty_print(),
                    rel.pretty_print(),
                    rhs.pretty_print()
                )
            }
            Self::UnaryOp(op, expr) => format!("{}({})", op.pretty_print(), expr.pretty_print()),
            Self::Ternary {
                cond,
                if_yes,
                if_no,
            } => format!(
                "({}) ? ({}) : ({})",
                cond.pretty_print(),
                if_yes.pretty_print(),
                if_no.pretty_print()
            ),
            Self::BinOp(op, lhs, rhs) => format!(
                "({}) {} ({})",
                lhs.pretty_print(),
                op.pretty_print(),
                rhs.pretty_print()
            ),
            Self::Nullptr => "nullptr".to_string(),
            Self::Str(s) => {
                let mut buf = String::new();

                for ch in s.chars() {
                    match ch {
                        '\n' => buf.push_str("\\n"),
                        '\r' => buf.push_str("\\r"),
                        '\0' => buf.push_str("\\0"),
                        '\\' => buf.push_str("\\\\"),
                        '"' => buf.push_str("\\\""),
                        _ => buf.push(ch),
                    }
                }

                format!("\"{}\"", buf)
            }
            Self::Iden(name) => name.clone(),
            Self::Int(n) => n.to_string(),
            Self::Float(f) => f.to_string(),
            Self::FuncCall(func, args) => format!(
                "({})({})",
                func.pretty_print(),
                args.iter()
                    .map(Expr::pretty_print)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::ArrayAccess(arr, idx) => {
                format!("({})[{}]", arr.pretty_print(), idx.pretty_print())
            }
            Self::Bool(b) => b.to_string(),
        }
    }
}

impl BinOp {
    pub fn pretty_print(&self) -> String {
        match self {
            Self::Add => "+".to_string(),
            Self::Sub => "-".to_string(),
            Self::Mul => "*".to_string(),
            Self::Div => "/".to_string(),
            Self::Mod => "%".to_string(),
            Self::Comma => ",".to_string(),
            Self::BoolAnd => "&".to_string(),
            Self::BoolOr => "|".to_string(),
        }
    }
}

impl UnaryOp {
    pub fn pretty_print(&self) -> String {
        match self {
            Self::Plus => "+".to_string(),
            Self::Minus => "-".to_string(),
            Self::BoolNot => "~".to_string(),
            Self::Addr => "&".to_string(),
            Self::Deref => "*".to_string(),
            Self::Sizeof => "sizeof ".to_string(),
        }
    }
}

impl Rel {
    pub fn pretty_print(&self) -> String {
        match self {
            Self::Eq => "==".to_string(),
            Self::Neq => "!=".to_string(),
            Self::Gt => ">".to_string(),
            Self::Geq => ">=".to_string(),
            Self::Lt => "<".to_string(),
            Self::Leq => "<=".to_string(),
        }
    }
}

impl Block {
    pub fn pretty_print(&self, indent: usize) -> String {
        let space = SHIFT_WIDTH * indent;
        let mut out = String::from("{\n");

        for stmt in &self.stmts {
            out.push_str(&stmt.pretty_print(indent + 1));
            out.push('\n');
        }

        out.push_str(&format!("{:space$}}}", ""));
        out
    }
}
