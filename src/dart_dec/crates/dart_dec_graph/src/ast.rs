use serde::Serialize;

/// Dart AST node for decompiled code
#[derive(Debug, Clone, Serialize)]
pub enum AstNode {
    Block(Vec<AstNode>),
    If {
        condition: AstExpr,
        then_body: Box<AstNode>,
        else_body: Option<Box<AstNode>>,
    },
    While {
        condition: AstExpr,
        body: Box<AstNode>,
    },
    DoWhile {
        body: Box<AstNode>,
        condition: AstExpr,
    },
    For {
        init: Box<AstNode>,
        condition: AstExpr,
        update: Box<AstNode>,
        body: Box<AstNode>,
    },
    Switch {
        subject: AstExpr,
        cases: Vec<(AstExpr, AstNode)>,
        default: Option<Box<AstNode>>,
    },
    TryCatch {
        try_body: Box<AstNode>,
        catches: Vec<CatchClause>,
        finally_body: Option<Box<AstNode>>,
    },
    Return(Option<AstExpr>),
    Throw(AstExpr),
    ExprStatement(AstExpr),
    VarDecl {
        name: String,
        dart_type: Option<String>,
        value: Option<AstExpr>,
        is_final: bool,
        is_late: bool,
    },
    Await(AstExpr),
    Yield(AstExpr),
    YieldStar(AstExpr),
    Break,
    Continue,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatchClause {
    pub exception_type: Option<String>,
    pub exception_var: String,
    pub stack_trace_var: Option<String>,
    pub body: AstNode,
}

/// Dart AST expression
#[derive(Debug, Clone, Serialize)]
pub enum AstExpr {
    Literal(DartLiteral),
    Variable(String),
    FieldAccess(Box<AstExpr>, String),
    MethodCall {
        receiver: Option<Box<AstExpr>>,
        method: String,
        args: Vec<AstExpr>,
        type_args: Vec<String>,
    },
    BinaryOp(Box<AstExpr>, BinOp, Box<AstExpr>),
    UnaryOp(UnaryOp, Box<AstExpr>),
    Conditional(Box<AstExpr>, Box<AstExpr>, Box<AstExpr>),
    IsCheck(Box<AstExpr>, String),
    AsCheck(Box<AstExpr>, String),
    NullAwareAccess(Box<AstExpr>, String),
    NullAssert(Box<AstExpr>),
    ListLiteral(Vec<AstExpr>),
    MapLiteral(Vec<(AstExpr, AstExpr)>),
    RecordLiteral(Vec<AstExpr>, Vec<(String, AstExpr)>),
    Lambda {
        params: Vec<String>,
        body: Box<AstNode>,
    },
    Cascade(Box<AstExpr>, Vec<AstExpr>),
    StringInterpolation(Vec<StringPart>),
    IndexAccess(Box<AstExpr>, Box<AstExpr>),
    ConstructorCall {
        class_name: String,
        constructor_name: Option<String>,
        args: Vec<AstExpr>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum DartLiteral {
    Int(i64),
    Double(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Equal, NotEqual, LessThan, GreaterThan, LessOrEqual, GreaterOrEqual,
    LogicalAnd, LogicalOr,
    BitwiseAnd, BitwiseOr, BitwiseXor,
    ShiftLeft, ShiftRight,
    NullCoalesce,
    Assign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UnaryOp {
    Neg, Not, BitwiseNot, PreIncrement, PostIncrement, PreDecrement, PostDecrement,
}

#[derive(Debug, Clone, Serialize)]
pub enum StringPart {
    Literal(String),
    Interpolation(AstExpr),
}

/// Generate Dart code from an AST node
pub fn generate_dart_code(node: &AstNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match node {
        AstNode::Block(stmts) => {
            stmts.iter().map(|s| generate_dart_code(s, indent)).collect::<Vec<_>>().join("\n")
        }
        AstNode::If { condition, then_body, else_body } => {
            let mut s = format!("{}if ({}) {{\n", pad, expr_to_dart(condition));
            s += &generate_dart_code(then_body, indent + 1);
            s += &format!("\n{}}}", pad);
            if let Some(else_b) = else_body {
                s += &format!(" else {{\n{}\n{}}}", generate_dart_code(else_b, indent + 1), pad);
            }
            s
        }
        AstNode::While { condition, body } => {
            format!("{}while ({}) {{\n{}\n{}}}", pad, expr_to_dart(condition), generate_dart_code(body, indent + 1), pad)
        }
        AstNode::DoWhile { body, condition } => {
            format!("{}do {{\n{}\n{}}} while ({});", pad, generate_dart_code(body, indent + 1), pad, expr_to_dart(condition))
        }
        AstNode::For { init, condition, update, body } => {
            format!("{}for ({}; {}; {}) {{\n{}\n{}}}", pad, generate_dart_code(init, 0).trim(), expr_to_dart(condition), generate_dart_code(update, 0).trim(), generate_dart_code(body, indent + 1), pad)
        }
        AstNode::Switch { subject, cases, default } => {
            let mut s = format!("{}switch ({}) {{\n", pad, expr_to_dart(subject));
            for (pattern, body) in cases {
                s += &format!("{}  case {}:\n{}\n{}    break;\n", pad, expr_to_dart(pattern), generate_dart_code(body, indent + 2), pad);
            }
            if let Some(def) = default {
                s += &format!("{}  default:\n{}\n", pad, generate_dart_code(def, indent + 2));
            }
            s += &format!("{}}}", pad);
            s
        }
        AstNode::TryCatch { try_body, catches, finally_body } => {
            let mut s = format!("{}try {{\n{}\n{}}}", pad, generate_dart_code(try_body, indent + 1), pad);
            for catch in catches {
                let exc_type = catch.exception_type.as_deref().unwrap_or("dynamic");
                s += &format!(" catch ({} {}) {{\n{}\n{}}}", exc_type, catch.exception_var, generate_dart_code(&catch.body, indent + 1), pad);
            }
            if let Some(fin) = finally_body {
                s += &format!(" finally {{\n{}\n{}}}", generate_dart_code(fin, indent + 1), pad);
            }
            s
        }
        AstNode::Return(Some(expr)) => format!("{}return {};", pad, expr_to_dart(expr)),
        AstNode::Return(None) => format!("{}return;", pad),
        AstNode::Throw(expr) => format!("{}throw {};", pad, expr_to_dart(expr)),
        AstNode::ExprStatement(expr) => format!("{}{};", pad, expr_to_dart(expr)),
        AstNode::VarDecl { name, dart_type, value, is_final, is_late } => {
            let prefix = if *is_late { "late " } else { "" };
            let kind = if *is_final { "final" } else { "var" };
            let type_str = dart_type.as_deref().unwrap_or("");
            let val_str = value.as_ref().map(|v| format!(" = {}", expr_to_dart(v))).unwrap_or_default();
            if type_str.is_empty() {
                format!("{}{}{} {}{};", pad, prefix, kind, name, val_str)
            } else {
                format!("{}{}{} {} {}{};", pad, prefix, type_str, kind, name, val_str)
            }
        }
        AstNode::Await(expr) => format!("{}await {};", pad, expr_to_dart(expr)),
        AstNode::Yield(expr) => format!("{}yield {};", pad, expr_to_dart(expr)),
        AstNode::YieldStar(expr) => format!("{}yield* {};", pad, expr_to_dart(expr)),
        AstNode::Break => format!("{}break;", pad),
        AstNode::Continue => format!("{}continue;", pad),
    }
}

fn expr_to_dart(expr: &AstExpr) -> String {
    match expr {
        AstExpr::Literal(lit) => match lit {
            DartLiteral::Int(v) => v.to_string(),
            DartLiteral::Double(v) => format!("{}", v),
            DartLiteral::String(s) => format!("'{}'", s.replace('\'', "\\'")),
            DartLiteral::Bool(b) => b.to_string(),
            DartLiteral::Null => "null".to_string(),
        },
        AstExpr::Variable(name) => name.clone(),
        AstExpr::FieldAccess(recv, field) => format!("{}.{}", expr_to_dart(recv), field),
        AstExpr::MethodCall { receiver, method, args, type_args } => {
            let recv_str = receiver.as_ref().map(|r| format!("{}.", expr_to_dart(r))).unwrap_or_default();
            let type_args_str = if type_args.is_empty() { String::new() } else { format!("<{}>", type_args.join(", ")) };
            let args_str = args.iter().map(expr_to_dart).collect::<Vec<_>>().join(", ");
            format!("{}{}{}({})", recv_str, method, type_args_str, args_str)
        }
        AstExpr::BinaryOp(lhs, op, rhs) => {
            let op_str = match op {
                BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%",
                BinOp::Equal => "==", BinOp::NotEqual => "!=",
                BinOp::LessThan => "<", BinOp::GreaterThan => ">",
                BinOp::LessOrEqual => "<=", BinOp::GreaterOrEqual => ">=",
                BinOp::LogicalAnd => "&&", BinOp::LogicalOr => "||",
                BinOp::BitwiseAnd => "&", BinOp::BitwiseOr => "|", BinOp::BitwiseXor => "^",
                BinOp::ShiftLeft => "<<", BinOp::ShiftRight => ">>",
                BinOp::NullCoalesce => "??",
                BinOp::Assign => "=",
            };
            format!("{} {} {}", expr_to_dart(lhs), op_str, expr_to_dart(rhs))
        }
        AstExpr::UnaryOp(op, expr) => {
            let op_str = match op {
                UnaryOp::Neg => "-", UnaryOp::Not => "!", UnaryOp::BitwiseNot => "~",
                UnaryOp::PreIncrement => "++", UnaryOp::PostIncrement => "++",
                UnaryOp::PreDecrement => "--", UnaryOp::PostDecrement => "--",
            };
            format!("{}{}", op_str, expr_to_dart(expr))
        }
        AstExpr::Conditional(cond, then_e, else_e) => {
            format!("{} ? {} : {}", expr_to_dart(cond), expr_to_dart(then_e), expr_to_dart(else_e))
        }
        AstExpr::IsCheck(expr, type_name) => format!("{} is {}", expr_to_dart(expr), type_name),
        AstExpr::AsCheck(expr, type_name) => format!("{} as {}", expr_to_dart(expr), type_name),
        AstExpr::NullAwareAccess(expr, field) => format!("{}?.{}", expr_to_dart(expr), field),
        AstExpr::NullAssert(expr) => format!("{}!", expr_to_dart(expr)),
        AstExpr::ListLiteral(items) => format!("[{}]", items.iter().map(expr_to_dart).collect::<Vec<_>>().join(", ")),
        AstExpr::MapLiteral(entries) => format!("{{{}}}", entries.iter().map(|(k, v)| format!("{}: {}", expr_to_dart(k), expr_to_dart(v))).collect::<Vec<_>>().join(", ")),
        AstExpr::RecordLiteral(pos, named) => {
            let mut parts = pos.iter().map(expr_to_dart).collect::<Vec<_>>();
            parts.extend(named.iter().map(|(k, v)| format!("{}: {}", k, expr_to_dart(v))));
            format!("({})", parts.join(", "))
        }
        AstExpr::Lambda { params, body } => format!("({}) => {}", params.join(", "), generate_dart_code(body, 0)),
        AstExpr::Cascade(obj, calls) => {
            let mut s = expr_to_dart(obj);
            for c in calls { s += &format!("\n  ..{}", expr_to_dart(c)); }
            s
        }
        AstExpr::StringInterpolation(parts) => {
            let mut s = String::from("'");
            for part in parts {
                match part {
                    StringPart::Literal(lit) => s += lit,
                    StringPart::Interpolation(e) => s += &format!("${{{}}}", expr_to_dart(e)),
                }
            }
            s += "'";
            s
        }
        AstExpr::IndexAccess(obj, idx) => format!("{}[{}]", expr_to_dart(obj), expr_to_dart(idx)),
        AstExpr::ConstructorCall { class_name, constructor_name, args } => {
            let ctor = constructor_name.as_ref().map(|n| format!(".{}", n)).unwrap_or_default();
            let args_str = args.iter().map(expr_to_dart).collect::<Vec<_>>().join(", ");
            format!("{}{}({})", class_name, ctor, args_str)
        }
    }
}
