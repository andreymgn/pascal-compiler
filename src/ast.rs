#[derive(Debug)]
pub struct Program {
    pub head: ProgramHeading,
    pub block: Block,
}

pub type ProgramHeading = Vec<Identifier>;

pub type Identifier = String;

#[derive(Debug)]
pub struct Block {
    pub label_decls: Vec<usize>,
    pub constant_defs: Vec<ConstDef>,
    pub type_defs: Vec<TypeDef>,
    pub variable_decls: Vec<VariableDecl>,
    pub proc_decls: Vec<ProcedureOrFuncDecl>,
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub struct ConstDef {
    pub id: Identifier,
    pub expr: ConstExpression,
}

#[derive(Debug)]
pub enum ConstExpression {
    RelExpr {
        lhs: ConstSimpleExpression,
        op: RelOp,
        rhs: ConstSimpleExpression,
    },
    Simple(ConstSimpleExpression),
}

#[derive(Debug)]
pub enum ConstSimpleExpression {
    AddExpr {
        lhs: Box<ConstSimpleExpression>,
        op: AddOp,
        rhs: ConstTerm,
    },
    Term(ConstTerm),
}

#[derive(Debug)]
pub enum ConstTerm {
    MulExpr {
        lhs: Box<ConstTerm>,
        op: MulOp,
        rhs: ConstFactor,
    },
    Factor(ConstFactor),
}

#[derive(Debug)]
pub enum ConstFactor {
    Factor {
        is_negative: bool,
        expr: Box<ConstFactor>,
    },
    Exponentiation(ConstExponentiation),
}

#[derive(Debug)]
pub enum ConstExponentiation {
    Exponent {
        base: ConstPrimary,
        pow: Box<ConstExponentiation>,
    },
    Primary(ConstPrimary),
}

#[derive(Debug)]
pub enum ConstPrimary {
    Identifier(Identifier),
    Paren(Box<ConstExpression>),
    UnsignedConstant(UnsignedConstant),
    Not(Box<ConstPrimary>),
}

#[derive(Debug, Clone)]
pub enum Constant {
    NonString(NonString),
    SignedConstant {
        is_negative: bool,
        constant: Box<NonString>,
    },
    String(String),
}

#[derive(Debug, Clone)]
pub enum NonString {
    Integer(usize),
    Real(f32),
    Identifier(Identifier),
}

#[derive(Debug)]
pub enum UnsignedConstant {
    Number(Number),
    String(String),
    Nil,
}

#[derive(Debug)]
pub enum Number {
    Integer(usize),
    Real(f32),
}

#[derive(Debug)]
pub struct TypeDef {
    pub id: Identifier,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub enum Type {
    Identifier(Identifier),
    NewType(Box<NewType>),
}

#[derive(Debug, Clone)]
pub enum NewType {
    OrdinalType(NewOrdinalType),
    StructuredType(Box<StructuredType>),
    PointerType(Identifier),
}

#[derive(Debug, Clone)]
pub enum NewOrdinalType {
    EnumeratedType(Vec<Identifier>),
    SubrangeType { low: Constant, high: Constant },
}

#[derive(Debug, Clone)]
pub enum StructuredType {
    Array {
        index_list: Vec<OrdinalType>,
        typ: Box<Type>,
    },
    Record {
        record_section: Vec<RecordSection>,
        variant_section: Option<RecordVariantPart>,
    },
    Set(OrdinalType),
    File(Box<Type>),
}

#[derive(Debug, Clone)]
pub enum OrdinalType {
    NewOrdinalType(NewOrdinalType),
    Identifier(Identifier),
}

#[derive(Debug, Clone)]
pub struct RecordSection {
    pub ids: Vec<Identifier>,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub struct RecordVariantPart {
    pub selector: VariantSelector,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone)]
pub struct VariantSelector {
    pub field: Option<Identifier>,
    pub tag: Identifier,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub case_consts: Vec<CaseConstant>,
    pub record_section: Vec<RecordSection>,
    pub variant_section: Option<RecordVariantPart>,
}

#[derive(Debug, Clone)]
pub struct CaseConstant {
    pub low: Constant,
    pub high: Option<Constant>,
}

#[derive(Debug)]
pub struct VariableDecl {
    pub ids: Vec<Identifier>,
    pub typ: Type,
}

#[derive(Debug)]
pub enum ProcedureOrFuncDecl {
    Procedure(ProcedureDecl),
    Function(FunctionDecl),
}

#[derive(Debug)]
pub enum ProcedureDecl {
    Directive {
        head: ProcedureHeading,
        is_forward: bool,
    },
    Block {
        head: ProcedureHeading,
        block: Block,
    },
}

#[derive(Debug)]
pub struct ProcedureHeading {
    pub name: Identifier,
    pub params: Vec<FormalParameter>,
}

#[derive(Debug)]
pub enum FormalParameter {
    Value {
        lhs: Vec<Identifier>,
        rhs: Identifier,
    },
    Variable {
        lhs: Vec<Identifier>,
        rhs: Identifier,
    },
    Procedure(ProcedureHeading),
    Function(FunctionHeading),
}

#[derive(Debug)]
pub enum FunctionDecl {
    Identification {
        name: Identifier,
        block: Block,
    },
    Block {
        head: FunctionHeading,
        block: Block,
    },
    Directive {
        head: FunctionHeading,
        is_forward: bool,
    },
}

#[derive(Debug)]
pub struct FunctionHeading {
    pub name: Identifier,
    pub params: Vec<FormalParameter>,
    pub result: Identifier,
}

#[derive(Debug)]
pub enum Statement {
    Open {
        label: Option<usize>,
        s: OpenStatement,
    },
    Closed {
        label: Option<usize>,
        s: ClosedStatement,
    },
}

#[derive(Debug)]
pub enum ClosedStatement {
    Assignment(Assignment),
    Procedure {
        id: Identifier,
        params: Vec<Parameter>,
    },
    Goto(usize),
    Compound(Vec<Statement>),
    Case {
        index: Expression,
        cases: Vec<CaseElement>,
        otherwise: Option<Box<Statement>>,
    },
    Repeat {
        s: Vec<Statement>,
        until: Expression,
    },
    With {
        vars: Vec<VariableAccess>,
        s: Box<Statement>,
    },
    If {
        predicate: Expression,
        then: Box<Statement>,
        els: Box<Statement>,
    },
    While {
        predicate: Expression,
        s: Box<Statement>,
    },
    For {
        var: Identifier,
        init: Expression,
        inc: bool,
        fin: Expression,
        s: Box<Statement>,
    },
    Empty,
}

#[derive(Debug)]
pub enum OpenStatement {
    With {
        vars: Vec<VariableAccess>,
        s: Box<Statement>,
    },
    If(OpenIf),
    While {
        predicate: Expression,
        s: Box<Statement>,
    },
    For {
        var: Identifier,
        init: Expression,
        inc: bool,
        fin: Expression,
        s: Box<Statement>,
    },
}

#[derive(Debug)]
pub enum OpenIf {
    WithoutElse {
        predicate: Expression,
        then: Box<Statement>,
    },
    WithElse {
        predicate: Expression,
        then: Box<Statement>,
        els: Box<Statement>,
    },
}

#[derive(Debug)]
pub struct Assignment {
    pub var: VariableAccess,
    pub rhs: Expression,
}

#[derive(Debug)]
pub enum VariableAccess {
    Identifier(Identifier),
    Indexed {
        var: Box<VariableAccess>,
        index: Vec<Expression>,
    },
    Field {
        lhs: Box<VariableAccess>,
        rhs: Identifier,
    },
    Deref(Box<VariableAccess>),
}

pub type Parameter = Expression;

#[derive(Debug)]
pub struct CaseElement {
    pub cases: Vec<CaseConstant>,
    pub s: Statement,
}

#[derive(Debug)]
pub enum Expression {
    Simple(SimpleExpression),
    RelExpr {
        lhs: Box<SimpleExpression>,
        op: RelOp,
        rhs: Box<SimpleExpression>,
    },
}

#[derive(Debug)]
pub enum SimpleExpression {
    Term(Term),
    AddExpr {
        lhs: Box<SimpleExpression>,
        op: AddOp,
        rhs: Term,
    },
}

#[derive(Debug)]
pub enum Term {
    Factor(Factor),
    MulExpr {
        lhs: Box<Term>,
        op: MulOp,
        rhs: Factor,
    },
}

#[derive(Debug)]
pub enum Factor {
    Factor {
        is_negative: bool,
        expr: Box<Factor>,
    },
    Exponentiation(Exponentiation),
}

#[derive(Debug)]
pub enum Exponentiation {
    Primary(Primary),
    Exponent {
        base: Primary,
        pow: Box<Exponentiation>,
    },
}

#[derive(Debug)]
pub enum Primary {
    VariableAccess(VariableAccess),
    UnsignedConstant(UnsignedConstant),
    FunctionCall {
        name: Identifier,
        params: Vec<Parameter>,
    },
    Set(Vec<Vec<Expression>>),
    Paren(Box<Expression>),
    Not(Box<Primary>),
}

#[derive(Debug)]
pub enum AddOp {
    Add,
    Sub,
    Or,
}

#[derive(Debug)]
pub enum MulOp {
    Mul,
    FDiv,
    IDiv,
    Mod,
    And,
}

#[derive(Debug)]
pub enum RelOp {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    In,
}
