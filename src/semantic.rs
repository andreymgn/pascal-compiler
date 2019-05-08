use crate::ast::*;

use std::collections::HashSet;

use lazy_static::lazy_static;

#[derive(Debug)]
pub struct Scope {
    labels: Vec<usize>,
    consts: Vec<Const>,
    vars: Vec<Variable>,
    types: Vec<TypeDef>,
    procs: Vec<Proc>,
}

#[derive(Debug)]
pub enum Const {
    Integer { name: String, value: u32 },
    Real { name: String, value: f32 },
    Identifier { name: String, value: String },
    String { name: String, value: String },
    Nil,
}

#[derive(Debug)]
pub struct Variable {
    name: String,
    typ: Type,
}

#[derive(Debug)]
pub enum Proc {
    Declaration {
        name: String,
        params: Vec<FormalParameter>,
        ret: Option<String>,
    },
    Definition {
        name: String,
        params: Vec<FormalParameter>,
        ret: Option<String>,
        scope: Scope,
    },
}

pub fn get_symbols(p: Program) -> Scope {
    convert_block(p.block)
}

fn convert_block(b: Block) -> Scope {
    let labels = b.label_decls;
    let consts = get_consts(b.constant_defs);
    let vars = get_vars(b.variable_decls);
    let types = b.type_defs;
    let procs = get_procs(b.proc_decls);
    Scope {
        labels: labels,
        consts: consts,
        vars: vars,
        types: types,
        procs: procs,
    }
}

fn get_consts(cs: Vec<ConstDef>) -> Vec<Const> {
    let mut res = vec![];
    for c in cs.into_iter() {
        res.push(constexpr_to_const(c.id, c.expr).unwrap());
    }
    res
}

fn constexpr_to_const(name: String, c: ConstExpression) -> Result<Const, String> {
    if let ConstExpression::Simple(ConstSimpleExpression::Term(ConstTerm::Factor(
        ConstFactor::Exponentiation(ConstExponentiation::Primary(p)),
    ))) = c
    {
        match p {
            ConstPrimary::Identifier(v) => {
                return Ok(Const::Identifier {
                    name: name,
                    value: v,
                })
            }
            ConstPrimary::Paren(x) => return constexpr_to_const(name, *x),
            ConstPrimary::UnsignedConstant(x) => match x {
                UnsignedConstant::Number(n) => match n {
                    Number::Integer(v) => {
                        return Ok(Const::Integer {
                            name: name,
                            value: v,
                        })
                    }
                    Number::Real(v) => {
                        return Ok(Const::Real {
                            name: name,
                            value: v,
                        })
                    }
                },
                UnsignedConstant::String(v) => {
                    return Ok(Const::String {
                        name: name,
                        value: v,
                    })
                }
                UnsignedConstant::Nil => return Ok(Const::Nil),
            },
            ConstPrimary::Not(_) => return Err("I don't match `Not`".to_string()),
        }
    }
    Err("Didn't match".to_string())
}

fn get_vars(vs: Vec<VariableDecl>) -> Vec<Variable> {
    let mut res = vec![];
    for v in vs.iter() {
        for id in v.ids.iter() {
            res.push(Variable {
                name: id.to_string(),
                typ: v.typ.clone(),
            })
        }
    }
    res
}

fn get_procs(ps: Vec<ProcedureOrFuncDecl>) -> Vec<Proc> {
    let mut res = vec![];
    for p in ps.into_iter() {
        res.push(match p {
            ProcedureOrFuncDecl::Procedure(v) => convert_procedure(v),
            ProcedureOrFuncDecl::Function(v) => convert_function(v),
        });
    }
    res
}

fn convert_procedure(p: ProcedureDecl) -> Proc {
    match p {
        ProcedureDecl::Directive {
            head,
            is_forward: _,
        } => Proc::Declaration {
            name: head.name.to_string(),
            params: head.params,
            ret: None,
        },
        ProcedureDecl::Block { head, block } => Proc::Definition {
            name: head.name.to_string(),
            params: head.params,
            ret: None,
            scope: convert_block(block),
        },
    }
}

fn convert_function(p: FunctionDecl) -> Proc {
    match p {
        FunctionDecl::Identification { name, block } => Proc::Definition {
            name: name.to_string(),
            params: vec![],
            ret: None,
            scope: convert_block(block),
        },
        FunctionDecl::Block { head, block } => Proc::Definition {
            name: head.name.to_string(),
            params: head.params,
            ret: Some(head.result),
            scope: convert_block(block),
        },
        FunctionDecl::Directive {
            head,
            is_forward: _,
        } => Proc::Declaration {
            name: head.name.to_string(),
            params: head.params,
            ret: Some(head.result),
        },
    }
}

pub fn check_variable_types(s: Scope) -> Vec<String> {
    let mut errs = vec![];
    for v in s.vars {
        errs.append(&mut check_type(v.typ));
    }
    for p in s.procs {
        match p {
            Proc::Definition {
                name: _,
                params: _,
                ret,
                scope,
            } => {
                if let Some(s) = ret {
                    if !DEFINED_TYPES.contains(&s[..]) {
                        errs.push(format!("Unknown type {}", s));
                    }
                }
                errs.append(&mut check_variable_types(scope));
            }
            Proc::Declaration {
                name: _,
                params: _,
                ret,
            } => {
                if let Some(s) = ret {
                    if !DEFINED_TYPES.contains(&s[..]) {
                        errs.push(format!("Unknown type {}", s));
                    }
                }
            }
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
