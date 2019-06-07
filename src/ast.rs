use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct Meta {
    pub start: (usize, usize),
    pub end: (usize, usize),
}

pub fn meta(start: (usize, usize), end: (usize, usize)) -> Meta {
    Meta { start, end }
}

impl fmt::Display for Meta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(({}, {}), ({}, {}))",
            self.start.0, self.start.1, self.end.0, self.end.1
        )
    }
}

#[derive(Debug)]
pub struct Program {
    pub meta: Meta,
    pub name: Identifier,
    pub block: Block,
}

pub type Identifier = String;

#[derive(Debug)]
pub struct Block {
    pub meta: Meta,
    pub label_decls: Vec<usize>,
    pub constant_defs: Vec<ConstDef>,
    pub type_defs: Vec<TypeDef>,
    pub variable_decls: Vec<VariableDecl>,
    pub proc_decls: Vec<ProcedureOrFuncDecl>,
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub struct ConstDef {
    pub meta: Meta,
    pub id: Identifier,
    pub expr: ConstExpression,
}

#[derive(Debug, Clone)]
pub enum ConstExpression {
    RelExpr {
        meta: Meta,
        lhs: ConstSimpleExpression,
        op: RelOp,
        rhs: ConstSimpleExpression,
    },
    Simple {
        meta: Meta,
        value: ConstSimpleExpression,
    },
}

#[derive(Debug, Clone)]
pub enum ConstSimpleExpression {
    AddExpr {
        meta: Meta,
        lhs: Box<ConstSimpleExpression>,
        op: AddOp,
        rhs: ConstTerm,
    },
    Term {
        meta: Meta,
        value: ConstTerm,
    },
}

#[derive(Debug, Clone)]
pub enum ConstTerm {
    MulExpr {
        meta: Meta,
        lhs: Box<ConstTerm>,
        op: MulOp,
        rhs: ConstFactor,
    },
    Factor {
        meta: Meta,
        value: ConstFactor,
    },
}

#[derive(Debug, Clone)]
pub enum ConstFactor {
    Factor {
        meta: Meta,
        is_negative: bool,
        expr: Box<ConstFactor>,
    },
    Exponentiation {
        meta: Meta,
        value: ConstExponentiation,
    },
}

#[derive(Debug, Clone)]
pub enum ConstExponentiation {
    Exponent {
        meta: Meta,
        base: ConstPrimary,
        pow: Box<ConstExponentiation>,
    },
    Primary {
        meta: Meta,
        value: ConstPrimary,
    },
}

#[derive(Debug, Clone)]
pub enum ConstPrimary {
    Identifier {
        meta: Meta,
        value: Identifier,
    },
    Paren {
        meta: Meta,
        value: Box<ConstExpression>,
    },
    UnsignedConstant {
        meta: Meta,
        value: UnsignedConstant,
    },
    Not {
        meta: Meta,
        value: Box<ConstPrimary>,
    },
}

#[derive(Debug, Clone)]
pub enum Constant {
    NonString {
        meta: Meta,
        value: NonString,
    },
    SignedConstant {
        meta: Meta,
        is_negative: bool,
        constant: Box<NonString>,
    },
    String {
        meta: Meta,
        value: String,
    },
}

#[derive(Debug, Clone)]
pub enum NonString {
    Integer { meta: Meta, value: usize },
    Real { meta: Meta, value: f32 },
    Identifier { meta: Meta, value: Identifier },
}

#[derive(Debug, Clone)]
pub enum UnsignedConstant {
    Number { meta: Meta, value: Number },
    String { meta: Meta, value: String },
    Nil { meta: Meta },
}

#[derive(Debug, Clone)]
pub enum Number {
    Integer { meta: Meta, value: usize },
    Real { meta: Meta, value: f32 },
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub meta: Meta,
    pub id: Identifier,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub enum Type {
    Identifier { meta: Meta, value: Identifier },
    NewType { meta: Meta, value: Box<NewType> },
}

#[derive(Debug, Clone)]
pub enum NewType {
    OrdinalType {
        meta: Meta,
        value: NewOrdinalType,
    },
    StructuredType {
        meta: Meta,
        value: Box<StructuredType>,
    },
    PointerType {
        meta: Meta,
        value: Identifier,
    },
}

#[derive(Debug, Clone)]
pub enum NewOrdinalType {
    EnumeratedType {
        meta: Meta,
        value: Vec<Identifier>,
    },
    SubrangeType {
        meta: Meta,
        low: Constant,
        high: Constant,
    },
}

#[derive(Debug, Clone)]
pub enum StructuredType {
    Array {
        meta: Meta,
        index_list: Vec<OrdinalType>,
        typ: Box<Type>,
    },
    Record {
        meta: Meta,
        record_section: Vec<RecordSection>,
        variant_section: Option<RecordVariantPart>,
    },
    Set {
        meta: Meta,
        value: OrdinalType,
    },
    File {
        meta: Meta,
        value: Box<Type>,
    },
}

#[derive(Debug, Clone)]
pub enum OrdinalType {
    NewOrdinalType { meta: Meta, value: NewOrdinalType },
    Identifier { meta: Meta, value: Identifier },
}

#[derive(Debug, Clone)]
pub struct RecordSection {
    pub meta: Meta,
    pub ids: Vec<Identifier>,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub struct RecordVariantPart {
    pub meta: Meta,
    pub selector: VariantSelector,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone)]
pub struct VariantSelector {
    pub meta: Meta,
    pub field: Option<Identifier>,
    pub tag: Identifier,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub meta: Meta,
    pub case_consts: Vec<CaseConstant>,
    pub record_section: Vec<RecordSection>,
    pub variant_section: Option<RecordVariantPart>,
}

#[derive(Debug, Clone)]
pub struct CaseConstant {
    pub meta: Meta,
    pub low: Constant,
    pub high: Option<Constant>,
}

#[derive(Debug)]
pub struct VariableDecl {
    pub meta: Meta,
    pub ids: Vec<Identifier>,
    pub typ: Type,
}

#[derive(Debug)]
pub enum ProcedureOrFuncDecl {
    Procedure { meta: Meta, value: ProcedureDecl },
    Function { meta: Meta, value: FunctionDecl },
}

#[derive(Debug)]
pub enum ProcedureDecl {
    Directive {
        meta: Meta,
        head: ProcedureHeading,
        is_forward: bool,
    },
    Block {
        meta: Meta,
        head: ProcedureHeading,
        block: Block,
    },
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ProcedureHeading {
    pub meta: Meta,
    pub name: Identifier,
    pub params: Vec<FormalParameter>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum FormalParameter {
    Value {
        meta: Meta,
        lhs: Vec<Identifier>,
        rhs: Identifier,
    },
    Variable {
        meta: Meta,
        lhs: Vec<Identifier>,
        rhs: Identifier,
    },
    Procedure {
        meta: Meta,
        value: ProcedureHeading,
    },
    Function {
        meta: Meta,
        value: FunctionHeading,
    },
}

#[derive(Debug)]
pub enum FunctionDecl {
    Identification {
        meta: Meta,
        name: Identifier,
        block: Block,
    },
    Block {
        meta: Meta,
        head: FunctionHeading,
        block: Block,
    },
    Directive {
        meta: Meta,
        head: FunctionHeading,
        is_forward: bool,
    },
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FunctionHeading {
    pub meta: Meta,
    pub name: Identifier,
    pub params: Vec<FormalParameter>,
    pub result: Identifier,
}

#[derive(Debug)]
pub enum Statement {
    Open {
        meta: Meta,
        label: Option<usize>,
        s: OpenStatement,
    },
    Closed {
        meta: Meta,
        label: Option<usize>,
        s: ClosedStatement,
    },
}

#[derive(Debug)]
pub enum ClosedStatement {
    Assignment {
        meta: Meta,
        value: Assignment,
    },
    Procedure {
        meta: Meta,
        id: Identifier,
        params: Vec<Parameter>,
    },
    Goto {
        meta: Meta,
        value: usize,
    },
    Compound {
        meta: Meta,
        value: Vec<Statement>,
    },
    Case {
        meta: Meta,
        index: Expression,
        cases: Vec<CaseElement>,
        otherwise: Option<Box<Statement>>,
    },
    Repeat {
        meta: Meta,
        s: Vec<Statement>,
        until: Expression,
    },
    With {
        meta: Meta,
        vars: Vec<VariableAccess>,
        s: Box<Statement>,
    },
    If {
        meta: Meta,
        predicate: Expression,
        then: Box<Statement>,
        els: Box<Statement>,
    },
    While {
        meta: Meta,
        predicate: Expression,
        s: Box<Statement>,
    },
    For {
        meta: Meta,
        var: Identifier,
        init: Expression,
        inc: bool,
        fin: Expression,
        s: Box<Statement>,
    },
    Empty {
        meta: Meta,
    },
}

#[derive(Debug)]
pub enum OpenStatement {
    With {
        meta: Meta,
        vars: Vec<VariableAccess>,
        s: Box<Statement>,
    },
    If {
        meta: Meta,
        value: OpenIf,
    },
    While {
        meta: Meta,
        predicate: Expression,
        s: Box<Statement>,
    },
    For {
        meta: Meta,
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
        meta: Meta,
        predicate: Expression,
        then: Box<Statement>,
    },
    WithElse {
        meta: Meta,
        predicate: Expression,
        then: Box<Statement>,
        els: Box<Statement>,
    },
}

#[derive(Debug)]
pub struct Assignment {
    pub meta: Meta,
    pub var: VariableAccess,
    pub rhs: Expression,
}

#[derive(Debug)]
pub enum VariableAccess {
    Identifier {
        meta: Meta,
        value: Identifier,
    },
    Indexed {
        meta: Meta,
        var: Box<VariableAccess>,
        index: Box<Expression>,
    },
    Field {
        meta: Meta,
        lhs: Box<VariableAccess>,
        rhs: Identifier,
    },
    Deref {
        meta: Meta,
        value: Box<VariableAccess>,
    },
}

pub type Parameter = Expression;

#[derive(Debug)]
pub struct CaseElement {
    pub meta: Meta,
    pub cases: Vec<CaseConstant>,
    pub s: Statement,
}

#[derive(Debug)]
pub enum Expression {
    Simple {
        meta: Meta,
        value: SimpleExpression,
    },
    RelExpr {
        meta: Meta,
        lhs: Box<SimpleExpression>,
        op: RelOp,
        rhs: Box<SimpleExpression>,
    },
}

#[derive(Debug)]
pub enum SimpleExpression {
    Term {
        meta: Meta,
        value: Term,
    },
    AddExpr {
        meta: Meta,
        lhs: Box<SimpleExpression>,
        op: AddOp,
        rhs: Term,
    },
}

#[derive(Debug)]
pub enum Term {
    Factor {
        meta: Meta,
        value: Factor,
    },
    MulExpr {
        meta: Meta,
        lhs: Box<Term>,
        op: MulOp,
        rhs: Factor,
    },
}

#[derive(Debug)]
pub enum Factor {
    Factor {
        meta: Meta,
        is_negative: bool,
        expr: Box<Factor>,
    },
    Exponentiation {
        meta: Meta,
        value: Exponentiation,
    },
}

#[derive(Debug)]
pub enum Exponentiation {
    Primary {
        meta: Meta,
        value: Primary,
    },
    Exponent {
        meta: Meta,
        base: Primary,
        pow: Box<Exponentiation>,
    },
}

#[derive(Debug)]
pub enum Primary {
    VariableAccess {
        meta: Meta,
        value: VariableAccess,
    },
    UnsignedConstant {
        meta: Meta,
        value: UnsignedConstant,
    },
    FunctionCall {
        meta: Meta,
        name: Identifier,
        params: Vec<Parameter>,
    },
    Set {
        meta: Meta,
        value: Vec<Vec<Expression>>,
    },
    Paren {
        meta: Meta,
        value: Box<Expression>,
    },
    Not {
        meta: Meta,
        value: Box<Primary>,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum AddOp {
    Add,
    Sub,
    Or,
}

#[derive(Debug, Copy, Clone)]
pub enum MulOp {
    Mul,
    FDiv,
    IDiv,
    Mod,
    And,
}

#[derive(Debug, Copy, Clone)]
pub enum RelOp {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    In,
}
