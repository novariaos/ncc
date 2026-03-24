#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Void,
    Int,
    Char,
    Pointer(Box<CType>),
    Array(Box<CType>, u32),
    Struct(String),
}

#[derive(Debug)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub globals: Vec<GlobalDecl>,
    pub functions: Vec<FuncDef>,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, CType)>,
}

#[derive(Debug)]
pub struct GlobalDecl {
    pub name: String,
    pub ty: CType,
    pub init: Option<Expr>,
}

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub return_ty: CType,
    pub params: Vec<Param>,
    pub is_variadic: bool,
    pub body: Option<Block>,
    pub is_static: bool,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: CType,
}

pub type Block = Vec<Stmt>;

#[derive(Debug)]
pub enum Stmt {
    Local {
        name: String,
        ty: CType,
        init: Option<Expr>,
    },
    Expr(Expr),
    Return(Option<Expr>),
    If {
        cond: Expr,
        then_body: Block,
        else_body: Option<Block>,
    },
    While {
        cond: Expr,
        body: Block,
    },
    DoWhile {
        body: Block,
        cond: Expr,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        step: Option<Expr>,
        body: Block,
    },
    Block(Block),
    Switch {
        expr: Expr,
        cases: Vec<(i32, Block)>,
        default: Option<Block>,
    },
    Break,
}

#[derive(Debug)]
pub enum Expr {
    IntLit(i32),
    StrLit(String),
    CharLit(i32),
    Var(String),
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        func: String,
        args: Vec<Expr>,
    },
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        expr: Box<Expr>,
        name: String,
    },
    ArrowField {
        expr: Box<Expr>,
        name: String,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    CompoundAssign {
        op: BinOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    PostIncDec {
        op: IncDec,
        expr: Box<Expr>,
    },
    PreIncDec {
        op: IncDec,
        expr: Box<Expr>,
    },
    SizeofType(CType),
    SizeofExpr(Box<Expr>),
    Cast {
        ty: CType,
        expr: Box<Expr>,
    },
    AddrOf(Box<Expr>),
    Deref(Box<Expr>),
    InitList(Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    LogicalAnd,
    LogicalOr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IncDec {
    Inc,
    Dec,
}
