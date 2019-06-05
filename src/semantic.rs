use crate::ast::*;

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use lazy_static::lazy_static;

#[derive(Debug)]
pub struct Scope {
    labels: Vec<usize>,
    consts: HashSet<Const>,
    vars: HashSet<Variable>,
    types: Vec<TypeDef>,
    procs: HashSet<Proc>,
}

#[derive(Debug)]
pub struct Const {
    name: String,
    variant: ConstType,
}

impl Hash for Const {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Const {
    fn eq(&self, other: &Const) -> bool {
        self.name == other.name
    }
}

impl Eq for Const {}

#[derive(Debug)]
pub enum ConstType {
    Integer(usize),
    Real(f32),
    Identifier(String),
    String(String),
    Nil,
}

#[derive(Debug)]
pub struct Variable {
    name: String,
    typ: Type,
}

impl Hash for Variable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Variable {
    fn eq(&self, other: &Variable) -> bool {
        self.name == other.name
    }
}

impl Eq for Variable {}

#[derive(Debug)]
pub struct Proc {
    name: String,
    params: Vec<FormalParameter>,
    ret: Option<String>,
    scope: Option<Scope>,
}

impl Hash for Proc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.params.hash(state);
        self.ret.hash(state);
        match self.scope {
            Some(_) => 1.hash(state),
            None => 0.hash(state),
        };
    }
}

impl PartialEq for Proc {
    fn eq(&self, other: &Proc) -> bool {
        let same_scope = (self.scope.is_some() && other.scope.is_some())
            || (self.scope.is_none() && other.scope.is_none());
        self.name == other.name
            && self.params == other.params
            && self.ret == other.ret
            && same_scope
    }
}

impl Eq for Proc {}

pub fn get_symbols(p: Program) -> Result<Scope, Vec<String>> {
    convert_block(p.block)
}

fn convert_block(b: Block) -> Result<Scope, Vec<String>> {
    let labels = b.label_decls;
    let consts = get_consts(b.constant_defs)?;
    let vars = get_vars(b.variable_decls)?;
    let types = b.type_defs;
    let procs = get_procs(b.proc_decls)?;
    Ok(Scope {
        labels,
        consts,
        vars,
        types,
        procs,
    })
}

fn get_consts(cs: Vec<ConstDef>) -> Result<HashSet<Const>, Vec<String>> {
    let mut res = HashSet::new();
    let mut errs = vec![];
    for c in cs.into_iter() {
        if !res.insert(constexpr_to_const(&c.id, c.expr).unwrap()) {
            errs.push(format!("Constant with name {} already defined", c.id));
        }
    }
    if errs.len() == 0 {
        return Ok(res);
    }
    Err(errs)
}

fn constexpr_to_const(name: &str, c: ConstExpression) -> Result<Const, String> {
    if let ConstExpression::Simple(ConstSimpleExpression::Term(ConstTerm::Factor(
        ConstFactor::Exponentiation(ConstExponentiation::Primary(p)),
    ))) = c
    {
        match p {
            ConstPrimary::Identifier(v) => {
                return Ok(Const {
                    name: name.to_string(),
                    variant: ConstType::Identifier(v),
                })
            }
            ConstPrimary::Paren(x) => return constexpr_to_const(name, *x),
            ConstPrimary::UnsignedConstant(x) => match x {
                UnsignedConstant::Number(n) => match n {
                    Number::Integer(v) => {
                        return Ok(Const {
                            name: name.to_string(),
                            variant: ConstType::Integer(v),
                        })
                    }
                    Number::Real(v) => {
                        return Ok(Const {
                            name: name.to_string(),
                            variant: ConstType::Real(v),
                        })
                    }
                },
                UnsignedConstant::String(v) => {
                    return Ok(Const {
                        name: name.to_string(),
                        variant: ConstType::String(v),
                    })
                }
                UnsignedConstant::Nil => {
                    return Ok(Const {
                        name: name.to_string(),
                        variant: ConstType::Nil,
                    })
                }
            },
            ConstPrimary::Not(_) => return Err("I don't match `Not`".to_string()),
        }
    }
    Err("Didn't match".to_string())
}

fn get_vars(vs: Vec<VariableDecl>) -> Result<HashSet<Variable>, Vec<String>> {
    let mut res = HashSet::new();
    let mut errs = vec![];
    for v in &vs {
        for id in v.ids.iter() {
            if !res.insert(Variable {
                name: id.to_string(),
                typ: v.typ.clone(),
            }) {
                errs.push(format!("Variable with name {} already defined", id));
            }
        }
    }
    if errs.len() == 0 {
        return Ok(res);
    }
    Err(errs)
}

fn get_procs(ps: Vec<ProcedureOrFuncDecl>) -> Result<HashSet<Proc>, Vec<String>> {
    let mut res = HashSet::new();
    let mut errs = vec![];
    for p in ps.into_iter() {
        let item = match p {
            ProcedureOrFuncDecl::Procedure(v) => convert_procedure(v),
            ProcedureOrFuncDecl::Function(v) => convert_function(v),
        };
        match item {
            Ok(proc) => {
                if !res.insert(proc) {
                    errs.push("Redeclaration of function".to_string());
                }
            }
            Err(mut es) => errs.append(es.as_mut()),
        }
    }
    if errs.len() == 0 {
        Ok(res)
    } else {
        Err(errs)
    }
}

fn convert_procedure(p: ProcedureDecl) -> Result<Proc, Vec<String>> {
    match p {
        ProcedureDecl::Directive {
            head,
            is_forward: _,
        } => Ok(Proc {
            name: head.name.to_string(),
            params: head.params,
            ret: None,
            scope: None,
        }),
        ProcedureDecl::Block { head, block } => {
            let scope = convert_block(block)?;
            Ok(Proc {
                name: head.name.to_string(),
                params: head.params,
                ret: None,
                scope: Some(scope),
            })
        }
    }
}

fn convert_function(p: FunctionDecl) -> Result<Proc, Vec<String>> {
    match p {
        FunctionDecl::Identification { name, block } => {
            let scope = convert_block(block)?;
            Ok(Proc {
                name: name.to_string(),
                params: vec![],
                ret: None,
                scope: Some(scope),
            })
        }
        FunctionDecl::Block { head, block } => {
            let scope = convert_block(block)?;
            Ok(Proc {
                name: head.name.to_string(),
                params: head.params,
                ret: Some(head.result),
                scope: Some(scope),
            })
        }
        FunctionDecl::Directive {
            head,
            is_forward: _,
        } => Ok(Proc {
            name: head.name.to_string(),
            params: head.params,
            ret: Some(head.result),
            scope: None,
        }),
    }
}

pub fn check_variable_types(s: Scope) -> Vec<String> {
    let mut errs = vec![];
    for v in s.vars {
        errs.append(&mut check_type(v.typ));
    }
    for p in s.procs {
        if let Some(s) = p.ret {
            if !DEFINED_TYPES.contains(&s[..]) {
                errs.push(format!("Function {} returns unknown type {}", p.name, s));
            }
        }
        if let Some(s) = p.scope {
            errs.append(check_variable_types(s).as_mut());
        }
    }
    errs
}

lazy_static! {
    static ref DEFINED_TYPES: HashSet<&'static str> =
        ["integer", "smallint", "longint", "real", "boolean", "string", "char", "byte",]
            .iter()
            .cloned()
            .collect();
}

fn check_type(t: Type) -> Vec<String> {
    let mut errs = vec![];
    match t {
        Type::Identifier(s) => {
            if !DEFINED_TYPES.contains(&s[..]) {
                errs.push(format!("Unknown type {}", s))
            }
        }
        Type::NewType(t) => match *t {
            NewType::OrdinalType(t) => {
                errs.append(&mut check_newordinal(t));
            }

            NewType::PointerType(s) => {
                if !DEFINED_TYPES.contains(&s[..]) {
                    errs.push(format!("Unknown pointer type {}", s))
                }
            }
            NewType::StructuredType(t) => errs.append(&mut check_structured(*t)),
        },
    }
    errs
}

fn check_newordinal(t: NewOrdinalType) -> Vec<String> {
    let mut errs = vec![];
    match t {
        NewOrdinalType::SubrangeType { low, high } => {
            if let Err(s) = check_constant(low) {
                errs.push(s);
            }
            if let Err(s) = check_constant(high) {
                errs.push(s);
            }
            errs
        }
        _ => errs,
    }
}

fn check_constant(c: Constant) -> Result<(), String> {
    match c {
        Constant::String(s) => Err(format!("Unexpected string '{}' in constant type", s)),
        _ => Ok(()),
    }
}

fn check_structured(s: StructuredType) -> Vec<String> {
    let mut errs = vec![];
    match s {
        StructuredType::Array { index_list, typ } => {
            for idx in index_list {
                errs.append(&mut check_ordinal(idx));
            }
            errs.append(&mut check_type(*typ));
        }
        StructuredType::Set(t) => errs.append(&mut check_ordinal(t)),
        StructuredType::File(t) => errs.append(&mut check_type(*t)),
        StructuredType::Record {
            record_section: _,
            variant_section: _,
        } => (),
    }
    errs
}

fn check_ordinal(t: OrdinalType) -> Vec<String> {
    let mut errs = vec![];
    match t {
        OrdinalType::Identifier(s) => {
            if !DEFINED_TYPES.contains(&s[..]) {
                errs.push(format!("Unknown type {}", s))
            }
        }
        OrdinalType::NewOrdinalType(t) => {
            errs.append(&mut check_newordinal(t));
        }
    }
    errs
}
