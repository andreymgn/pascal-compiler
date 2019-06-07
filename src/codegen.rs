use crate::ast::*;
use crate::semantic;

extern crate libc;

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::ptr;

extern crate llvm_sys as llvm;

use self::llvm::core::*;
use self::llvm::prelude::*;

unsafe fn cur_bb_has_no_terminator(builder: LLVMBuilderRef) -> bool {
    LLVMIsATerminatorInst(LLVMGetLastInstruction(LLVMGetInsertBlock(builder))) == ptr::null_mut()
}

#[derive(Clone, Debug)]
struct VarInfo {
    typ: Type,
    llvm_typ: LLVMTypeRef,
    llvm_val: LLVMValueRef,
    is_const: bool,
}

impl VarInfo {
    fn new(typ: Type, llvm_typ: LLVMTypeRef, llvm_val: LLVMValueRef, is_const: bool) -> VarInfo {
        VarInfo {
            typ,
            llvm_typ,
            llvm_val,
            is_const,
        }
    }
}

pub struct Codegen {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    global_varmap: HashMap<String, VarInfo>,
    cur_func: Option<LLVMValueRef>,
}

impl Codegen {
    pub unsafe fn new(name: String) -> Codegen {
        llvm::execution_engine::LLVMLinkInInterpreter();
        llvm::target::LLVM_InitializeAllTargetMCs();
        llvm::target::LLVM_InitializeNativeTarget();
        llvm::target::LLVM_InitializeNativeAsmPrinter();
        llvm::target::LLVM_InitializeNativeAsmParser();

        let context = LLVMContextCreate();

        let c_mod_name = CString::new(name).unwrap();
        let module = LLVMModuleCreateWithNameInContext(c_mod_name.as_ptr(), context);
        let mut global_varmap = HashMap::new();

        let mut ee = 0 as llvm::execution_engine::LLVMExecutionEngineRef;
        let mut error = 0 as *mut i8;
        if llvm::execution_engine::LLVMCreateExecutionEngineForModule(&mut ee, module, &mut error)
            != 0
        {
            println!("err");
        }

        let printf_typ = LLVMFunctionType(
            LLVMInt32Type(),
            vec![LLVMPointerType(LLVMInt8Type(), 0)]
                .as_mut_slice()
                .as_mut_ptr(),
            1,
            1,
        );
        let printf = LLVMAddFunction(module, CString::new("printf").unwrap().as_ptr(), printf_typ);
        global_varmap.insert(
            "printf".to_string(),
            VarInfo::new(
                Type::Identifier {
                    meta: Default::default(),
                    value: "internal_fn".to_string(),
                },
                printf_typ,
                printf,
                true,
            ),
        );

        Codegen {
            context: context,
            module: module,
            builder: LLVMCreateBuilderInContext(context),
            global_varmap: global_varmap,
            cur_func: None,
        }
    }

    pub unsafe fn populate_global(&mut self, symbols: semantic::Scope) -> Result<(), String> {
        self.gen_global_constants(symbols.consts)?;
        self.gen_global_variables(symbols.vars)?;
        self.gen_proc_decls(symbols.procs)?;
        Ok(())
    }

    unsafe fn gen_global_constants(
        &mut self,
        consts: HashSet<semantic::Const>,
    ) -> Result<(), String> {
        for c in consts {
            self.gen_constant(c)?;
        }
        Ok(())
    }

    unsafe fn gen_constant(&mut self, constant: semantic::Const) -> Result<(), String> {
        use semantic::ConstType;
        let (llvm_typ, var, var_typ) = match constant.variant {
            ConstType::Integer(i) => {
                let llvm_typ = LLVMInt32Type();
                let gvar = LLVMAddGlobal(
                    self.module,
                    llvm_typ,
                    CString::new(constant.name.as_str()).unwrap().as_ptr(),
                );
                LLVMSetInitializer(gvar, LLVMConstInt(llvm_typ, i as u64, 0));
                (
                    llvm_typ,
                    gvar,
                    Type::Identifier {
                        meta: Default::default(),
                        value: "integer".to_string(),
                    },
                )
            }
            ConstType::Real(r) => {
                let llvm_typ = LLVMFloatType();
                let gvar = LLVMAddGlobal(
                    self.module,
                    llvm_typ,
                    CString::new(constant.name.as_str()).unwrap().as_ptr(),
                );
                LLVMSetInitializer(gvar, LLVMConstReal(llvm_typ, r as f64));
                (
                    llvm_typ,
                    gvar,
                    Type::Identifier {
                        meta: Default::default(),
                        value: "real".to_string(),
                    },
                )
            }
            ConstType::String(_) => {
                return Err("Strings aren't working".to_string());
                // let len = (s.len()) as u32;
                // let llvm_typ = LLVMArrayType(LLVMInt8Type(), len);
                // let cstr = CString::new(s.as_str()).unwrap();
                // let gvar = LLVMBuildGlobalStringPtr(
                //     self.builder,
                //     cstr.as_ptr(),
                //     CString::new(constant.name.as_str()).unwrap().as_ptr(),
                // );
                // (
                //     llvm_typ,
                //     gvar,
                //     Type::Identifier {
                //         meta: Default::default(),
                //         value: "const_str".to_string(),
                //     },
                // )
            }
            ConstType::Identifier(i) => match self.global_varmap.get(&i) {
                Some(var) => {
                    let llvm_typ = var.llvm_typ;
                    let var_value = var.llvm_val;
                    let gvar = LLVMAddGlobal(
                        self.module,
                        llvm_typ,
                        CString::new(constant.name.as_str()).unwrap().as_ptr(),
                    );
                    LLVMSetInitializer(gvar, var_value);
                    (llvm_typ, gvar, var.typ.clone())
                }
                None => {
                    return Err(format!(
                        "Constant {} refers to undefined constant {}",
                        constant.name, i
                    ))
                }
            },
            ConstType::Nil => return Err("Nil not supported".to_string()),
        };
        LLVMSetGlobalConstant(var, 1);
        self.global_varmap.insert(
            constant.name.to_string(),
            VarInfo::new(var_typ, llvm_typ, var, true),
        );
        Ok(())
    }

    unsafe fn gen_global_variables(
        &mut self,
        vars: HashSet<semantic::Variable>,
    ) -> Result<(), String> {
        for v in vars.into_iter() {
            self.gen_variable_decl(v)?;
        }
        Ok(())
    }

    unsafe fn gen_proc_decls(&mut self, procs: HashSet<semantic::Proc>) -> Result<(), String> {
        for p in procs.into_iter() {
            self.gen_proc_decl(p)?;
        }
        Ok(())
    }

    unsafe fn gen_proc_decl(&mut self, proc: semantic::Proc) -> Result<(), String> {
        let mut param_types = vec![];
        for p in &proc.params {
            match p {
                FormalParameter::Value { meta: _, lhs, rhs } => {
                    for _ in lhs {
                        param_types.push(self.identifier_to_llvmty(rhs));
                    }
                }
                _ => return Err(format!("Unsupported parameter type {:?}", p)),
            };
        }
        let ret_type = if let Some(t) = proc.ret {
            self.identifier_to_llvmty(&t)
        } else {
            LLVMVoidType()
        };
        let llvm_typ = LLVMFunctionType(
            ret_type,
            param_types.as_mut_slice().as_mut_ptr(),
            param_types.len() as u32,
            0,
        );
        let func = LLVMAddFunction(
            self.module,
            CString::new(proc.name.clone()).unwrap().as_ptr(),
            llvm_typ,
        );
        self.global_varmap.insert(
            proc.name.clone(),
            VarInfo::new(
                Type::Identifier {
                    meta: Default::default(),
                    value: "user_fn".to_string(),
                },
                llvm_typ,
                func,
                true,
            ),
        );
        Ok(())
    }

    pub unsafe fn write_llvm_bitcode_to_file(&mut self, filename: &str) {
        llvm::bit_writer::LLVMWriteBitcodeToFile(
            self.module,
            CString::new(filename).unwrap().as_ptr(),
        );
    }

    pub unsafe fn run(&mut self, p: Program) -> Result<(), String> {
        let res = self.gen_program(p);
        LLVMDumpModule(self.module);
        res
    }

    unsafe fn gen_program(&mut self, p: Program) -> Result<(), String> {
        let func = LLVMAddFunction(
            self.module,
            CString::new("main").unwrap().as_ptr(),
            LLVMFunctionType(LLVMVoidType(), vec![].as_mut_slice().as_mut_ptr(), 0, 0),
        );
        self.cur_func = Some(func);
        let bb_entry = LLVMAppendBasicBlock(func, CString::new("entry").unwrap().as_ptr());
        LLVMPositionBuilderAtEnd(self.builder, bb_entry);

        self.gen_statements(p.block.statements)?;

        let mut iter_bb = LLVMGetFirstBasicBlock(func);
        while iter_bb != ptr::null_mut() {
            if LLVMIsATerminatorInst(LLVMGetLastInstruction(iter_bb)) == ptr::null_mut() {
                let terminator_builder = LLVMCreateBuilderInContext(self.context);
                LLVMPositionBuilderAtEnd(terminator_builder, iter_bb);
                LLVMBuildRetVoid(terminator_builder);
            }
            iter_bb = LLVMGetNextBasicBlock(iter_bb);
        }

        Ok(())
    }

    unsafe fn gen_variable_decl(&mut self, var: semantic::Variable) -> Result<(), String> {
        match var.typ {
            Type::Identifier { meta, value: i } => {
                let llvm_typ = self.identifier_to_llvmty(&i);
                let gvar = LLVMAddGlobal(
                    self.module,
                    llvm_typ,
                    CString::new(var.name.as_str()).unwrap().as_ptr(),
                );
                self.global_varmap.insert(
                    var.name.to_string(),
                    VarInfo::new(
                        Type::Identifier {
                            meta: meta,
                            value: i.to_string(),
                        },
                        llvm_typ,
                        gvar,
                        false,
                    ),
                );
                LLVMSetLinkage(gvar, llvm::LLVMLinkage::LLVMCommonLinkage);
                LLVMSetInitializer(gvar, LLVMConstNull(self.identifier_to_llvmty(&i)));
            }
            Type::NewType { meta: _, value: nt } => match *nt {
                NewType::StructuredType { meta: _, value: tt } => match &*tt {
                    StructuredType::Array {
                        meta,
                        index_list,
                        typ,
                    } => {
                        let t = match *typ.clone() {
                            Type::Identifier { meta: _, value: t } => self.identifier_to_llvmty(&t),
                            _ => return Err(format!("Array of complex values at {}", meta)),
                        };
                        let size = match &index_list[0] {
                            OrdinalType::NewOrdinalType{meta:_,value: NewOrdinalType::SubrangeType {
                                meta,
                                low: _, /* consider low to be always 0 */
                                high,
                            }} => {
                                if let Constant::NonString{meta:_, value: NonString::Integer{meta:_, value: x}} = high {
                                    x
                                } else {
                                    return Err(format!("Array of non-integer size at {}", meta));
                                }
                            },
                            _ => return Err("Arrays with types other than integers as indices are not supported".to_string())
                        };
                        let llvm_typ = LLVMArrayType(t, *size as u32);
                        let gvar = LLVMAddGlobal(
                            self.module,
                            llvm_typ,
                            CString::new(var.name.as_str()).unwrap().as_ptr(),
                        );
                        let typ = Type::NewType {
                            meta: *meta,
                            value: Box::new(NewType::StructuredType {
                                meta: *meta,
                                value: Box::new(StructuredType::Array {
                                    meta: *meta,
                                    index_list: index_list.clone(),
                                    typ: typ.clone(),
                                }),
                            }),
                        };
                        self.global_varmap
                            .insert(var.name, VarInfo::new(typ, llvm_typ, gvar, false));
                        LLVMSetLinkage(gvar, llvm::LLVMLinkage::LLVMCommonLinkage);
                        LLVMSetInitializer(gvar, LLVMConstNull(LLVMArrayType(t, *size as u32)));
                    }
                    _ => return Err("Types harder than arrays are not supported".to_string()),
                },
                _ => return Err("Types harder than arrays are not supported".to_string()),
            },
        };
        Ok(())
    }

    unsafe fn identifier_to_llvmty(&mut self, id: &str) -> LLVMTypeRef {
        match id {
            "integer" => LLVMInt32Type(),
            "smallint" => LLVMInt16Type(),
            "longint" => LLVMInt64Type(),
            "real" => LLVMFloatType(),
            "boolean" => LLVMInt1Type(),
            "char" => LLVMInt8Type(),
            "byte" => LLVMInt8Type(),
            _ => panic!("Unknown type"),
        }
    }

    unsafe fn gen_statements(&mut self, stmts: Vec<Statement>) -> Result<LLVMValueRef, String> {
        for s in stmts {
            self.gen_statement(s)?;
        }
        Ok(ptr::null_mut())
    }

    unsafe fn gen_statement(&mut self, stmt: Statement) -> Result<LLVMValueRef, String> {
        match stmt {
            Statement::Open {
                meta: _,
                label: _,
                s,
            } => self.gen_open_statement(s),
            Statement::Closed {
                meta: _,
                label: _,
                s,
            } => self.gen_closed_statement(s),
        }
    }

    unsafe fn gen_open_statement(&mut self, stmt: OpenStatement) -> Result<LLVMValueRef, String> {
        // for and if
        match stmt {
            OpenStatement::If {
                meta: _,
                value: open_if,
            } => self.gen_open_if(open_if),
            OpenStatement::For {
                meta: _,
                var,
                init,
                inc,
                fin,
                s,
            } => self.gen_for(var, init, inc, fin, *s),
            _ => Err(format!("Statement {:?} not supported", stmt)),
        }
    }

    unsafe fn gen_open_if(&mut self, typ: OpenIf) -> Result<LLVMValueRef, String> {
        match typ {
            OpenIf::WithoutElse {
                meta: _,
                predicate,
                then,
            } => {
                let condition_tmp = self.gen_expression(&predicate)?;
                let condition = self.val_to_bool(condition_tmp);

                let func = self.cur_func.unwrap();

                let bb_then = LLVMAppendBasicBlock(func, CString::new("then").unwrap().as_ptr());
                let bb_else = LLVMAppendBasicBlock(func, CString::new("else").unwrap().as_ptr());
                let bb_merge = LLVMAppendBasicBlock(func, CString::new("merge").unwrap().as_ptr());

                LLVMBuildCondBr(self.builder, condition, bb_then, bb_else);

                LLVMPositionBuilderAtEnd(self.builder, bb_then);
                // then block
                self.gen_statement(*then)?;
                if cur_bb_has_no_terminator(self.builder) {
                    LLVMBuildBr(self.builder, bb_merge);
                }

                LLVMPositionBuilderAtEnd(self.builder, bb_else);
                if cur_bb_has_no_terminator(self.builder) {
                    LLVMBuildBr(self.builder, bb_merge);
                }
                LLVMPositionBuilderAtEnd(self.builder, bb_merge);
                Ok(ptr::null_mut())
            }
            OpenIf::WithElse {
                meta: _,
                predicate,
                then,
                els,
            } => {
                let condition_tmp = self.gen_expression(&predicate)?;
                let condition = self.val_to_bool(condition_tmp);

                let func = self.cur_func.unwrap();

                let bb_then = LLVMAppendBasicBlock(func, CString::new("then").unwrap().as_ptr());
                let bb_else = LLVMAppendBasicBlock(func, CString::new("else").unwrap().as_ptr());
                let bb_merge = LLVMAppendBasicBlock(func, CString::new("merge").unwrap().as_ptr());

                LLVMBuildCondBr(self.builder, condition, bb_then, bb_else);

                LLVMPositionBuilderAtEnd(self.builder, bb_then);
                // then block
                self.gen_statement(*then)?;
                if cur_bb_has_no_terminator(self.builder) {
                    LLVMBuildBr(self.builder, bb_merge);
                }

                LLVMPositionBuilderAtEnd(self.builder, bb_else);
                // else block
                self.gen_statement(*els)?;
                if cur_bb_has_no_terminator(self.builder) {
                    LLVMBuildBr(self.builder, bb_merge);
                }

                LLVMPositionBuilderAtEnd(self.builder, bb_merge);
                Ok(ptr::null_mut())
            }
        }
    }

    unsafe fn gen_expression(&mut self, e: &Expression) -> Result<LLVMValueRef, String> {
        match e {
            Expression::RelExpr {
                meta: _,
                lhs,
                op,
                rhs,
            } => self.gen_rel_expr(&*lhs, op, &*rhs),
            Expression::Simple {
                meta: _,
                value: expr,
            } => self.gen_simple_expr(expr),
        }
    }

    unsafe fn gen_rel_expr(
        &mut self,
        lhs: &SimpleExpression,
        op: &RelOp,
        rhs: &SimpleExpression,
    ) -> Result<LLVMValueRef, String> {
        let lhs_gen = self.gen_simple_expr(lhs)?;
        let lhs_v = self.load_if_needed(lhs_gen);
        let rhs_gen = self.gen_simple_expr(rhs)?;
        let rhs_v = self.load_if_needed(rhs_gen);
        match op {
            RelOp::Equal => Ok(LLVMBuildICmp(
                self.builder,
                llvm::LLVMIntPredicate::LLVMIntEQ,
                lhs_v,
                rhs_v,
                CString::new("eql").unwrap().as_ptr(),
            )),
            RelOp::Greater => Ok(LLVMBuildICmp(
                self.builder,
                llvm::LLVMIntPredicate::LLVMIntSGT,
                lhs_v,
                rhs_v,
                CString::new("gt").unwrap().as_ptr(),
            )),
            RelOp::Less => Ok(LLVMBuildICmp(
                self.builder,
                llvm::LLVMIntPredicate::LLVMIntSLT,
                lhs_v,
                rhs_v,
                CString::new("lt").unwrap().as_ptr(),
            )),
            _ => Err("Operation is not implemented".to_string()),
        }
    }

    unsafe fn gen_simple_expr(&mut self, e: &SimpleExpression) -> Result<LLVMValueRef, String> {
        match e {
            SimpleExpression::AddExpr {
                meta: _,
                lhs,
                op,
                rhs,
            } => self.gen_add_expr(&*lhs, op, &*rhs),
            SimpleExpression::Term { meta: _, value: t } => self.gen_term(t),
        }
    }

    unsafe fn gen_add_expr(
        &mut self,
        lhs: &SimpleExpression,
        op: &AddOp,
        rhs: &Term,
    ) -> Result<LLVMValueRef, String> {
        let lhs_gen = self.gen_simple_expr(lhs)?;
        let lhs_v = self.load_if_needed(lhs_gen);
        let rhs_gen = self.gen_term(rhs)?;
        let rhs_v = self.load_if_needed(rhs_gen);
        match op {
            AddOp::Add => Ok(LLVMBuildAdd(
                self.builder,
                lhs_v,
                rhs_v,
                CString::new("add").unwrap().as_ptr(),
            )),
            AddOp::Sub => Ok(LLVMBuildSub(
                self.builder,
                lhs_v,
                rhs_v,
                CString::new("sub").unwrap().as_ptr(),
            )),
            AddOp::Or => Ok(LLVMBuildOr(
                self.builder,
                lhs_v,
                rhs_v,
                CString::new("or").unwrap().as_ptr(),
            )),
        }
    }

    unsafe fn gen_term(&mut self, e: &Term) -> Result<LLVMValueRef, String> {
        match e {
            Term::MulExpr {
                meta: _,
                lhs,
                op,
                rhs,
            } => self.gen_mul_expr(&*lhs, op, &*rhs),
            Term::Factor { meta: _, value: f } => self.gen_factor(f),
        }
    }

    unsafe fn gen_mul_expr(
        &mut self,
        lhs: &Term,
        op: &MulOp,
        rhs: &Factor,
    ) -> Result<LLVMValueRef, String> {
        let lhs_gen = self.gen_term(lhs)?;
        let lhs_v = self.load_if_needed(lhs_gen);
        let rhs_gen = self.gen_factor(rhs)?;
        let rhs_v = self.load_if_needed(rhs_gen);
        match op {
            MulOp::Mul => Ok(LLVMBuildMul(
                self.builder,
                lhs_v,
                rhs_v,
                CString::new("mul").unwrap().as_ptr(),
            )),
            MulOp::IDiv => Ok(LLVMBuildSDiv(
                self.builder,
                lhs_v,
                rhs_v,
                CString::new("div").unwrap().as_ptr(),
            )),
            MulOp::And => Ok(LLVMBuildAnd(
                self.builder,
                lhs_v,
                rhs_v,
                CString::new("and").unwrap().as_ptr(),
            )),
            _ => Err(format!("Operation {:?} not supported", op)),
        }
    }

    unsafe fn gen_factor(&mut self, e: &Factor) -> Result<LLVMValueRef, String> {
        match e {
            Factor::Factor {
                meta: _,
                is_negative,
                expr,
            } => self.gen_unary(*is_negative, &*expr),
            Factor::Exponentiation { meta: _, value: e } => self.gen_exponentiation(e),
        }
    }

    unsafe fn gen_unary(&mut self, is_negative: bool, f: &Factor) -> Result<LLVMValueRef, String> {
        let v = self.gen_factor(f);
        if is_negative {
            return Ok(LLVMBuildNeg(
                self.builder,
                v?,
                CString::new("neg").unwrap().as_ptr(),
            ));
        }
        v
    }

    unsafe fn gen_exponentiation(&mut self, e: &Exponentiation) -> Result<LLVMValueRef, String> {
        match e {
            Exponentiation::Primary { meta: _, value: p } => self.gen_primary(p),
            _ => Err(format!("Operation {:?} not supported", e)),
        }
    }

    unsafe fn gen_primary(&mut self, p: &Primary) -> Result<LLVMValueRef, String> {
        match p {
            Primary::VariableAccess { meta: _, value: va } => self.gen_variable_access(&va, false),
            Primary::UnsignedConstant { meta: _, value: uc } => self.gen_unsigned_constant(&uc),
            Primary::Paren { meta: _, value: e } => self.gen_expression(&*e),
            _ => Err(format!("Operation {:?} not supported", p)),
        }
    }

    unsafe fn gen_variable_access(
        &mut self,
        v: &VariableAccess,
        modification: bool,
    ) -> Result<LLVMValueRef, String> {
        match v {
            VariableAccess::Identifier { meta, value: i } => {
                if let Some(varinfo) = self.global_varmap.get(i.as_str()) {
                    if modification && varinfo.is_const {
                        return Err(format!(
                            "Attempt to modify constant variable {} at {}",
                            i, meta
                        ));
                    }
                    return Ok(varinfo.clone().llvm_val);
                }
                Err(format!("Use of undeclared variable {} at {}", i, meta))
            }
            VariableAccess::Indexed {
                meta: _,
                var,
                index,
            } => {
                let i = self.gen_expression(&*index)?;
                let v = self.gen_variable_access(var, modification)?;
                let i_load = self.load_if_needed(i);
                let zero = self.gen_unsigned_constant(&UnsignedConstant::Number {
                    meta: Default::default(),
                    value: Number::Integer {
                        meta: Default::default(),
                        value: 0,
                    },
                })?;
                let r = LLVMBuildGEP(
                    self.builder,
                    v,
                    vec![zero, i_load].as_mut_slice().as_mut_ptr(),
                    2,
                    CString::new("arrayidx").unwrap().as_ptr(),
                );
                Ok(r)
            }
            _ => Err(
                "Variable access other than by identifier or index are not supported".to_string(),
            ),
        }
    }

    unsafe fn gen_unsigned_constant(
        &mut self,
        c: &UnsignedConstant,
    ) -> Result<LLVMValueRef, String> {
        match c {
            UnsignedConstant::Number { meta: _, value: n } => match n {
                Number::Integer { meta: _, value: i } => {
                    Ok(LLVMConstInt(LLVMInt32Type(), *i as u64, 0))
                }
                Number::Real { meta: _, value: r } => Ok(LLVMConstReal(LLVMFloatType(), *r as f64)),
            },
            UnsignedConstant::String { meta: _, value: _ } => {
                Err("Strings aren't working".to_string())
            }
            UnsignedConstant::Nil { meta } => Err(format!("No nil allowed yet at {}", meta)),
        }
    }

    unsafe fn gen_for(
        &mut self,
        var: Identifier,
        init: Expression,
        is_inc: bool,
        fin: Expression,
        stmt: Statement,
    ) -> Result<LLVMValueRef, String> {
        let func = self.cur_func.unwrap();

        let bb_init = LLVMAppendBasicBlock(func, CString::new("loop_init").unwrap().as_ptr());
        let bb_test = LLVMAppendBasicBlock(func, CString::new("loop_test").unwrap().as_ptr());
        let bb_body = LLVMAppendBasicBlock(func, CString::new("loop_body").unwrap().as_ptr());
        let bb_step = LLVMAppendBasicBlock(func, CString::new("loop_step").unwrap().as_ptr());
        let bb_exit = LLVMAppendBasicBlock(func, CString::new("loop_exit").unwrap().as_ptr());

        // init
        let loop_var = self.global_varmap.get(&var).unwrap().llvm_val;
        LLVMBuildBr(self.builder, bb_init);
        LLVMPositionBuilderAtEnd(self.builder, bb_init);
        self.gen_assignment(Assignment {
            meta: Default::default(),
            var: VariableAccess::Identifier {
                meta: Default::default(),
                value: var.clone(),
            },
            rhs: init,
        })?;

        // test
        LLVMBuildBr(self.builder, bb_test);
        LLVMPositionBuilderAtEnd(self.builder, bb_test);
        let fin_expr = self.gen_expression(&fin)?;
        let fin_expr_load = self.load_if_needed(fin_expr);
        let loop_var_load = self.load_if_needed(loop_var);
        let cond = LLVMBuildICmp(
            self.builder,
            llvm::LLVMIntPredicate::LLVMIntSLT,
            loop_var_load,
            fin_expr_load,
            CString::new("loop_bound_check").unwrap().as_ptr(),
        );
        LLVMBuildCondBr(self.builder, cond, bb_body, bb_exit);

        // body
        LLVMPositionBuilderAtEnd(self.builder, bb_body);
        self.gen_statement(stmt)?;
        LLVMBuildBr(self.builder, bb_step);

        // step
        LLVMPositionBuilderAtEnd(self.builder, bb_step);
        if is_inc {
            self.gen_increment(var)?;
        } else {
            self.gen_decrement(var)?;
        }
        LLVMBuildBr(self.builder, bb_test);

        LLVMPositionBuilderAtEnd(self.builder, bb_exit);
        Ok(ptr::null_mut())
    }

    unsafe fn gen_increment(&mut self, var: String) -> Result<LLVMValueRef, String> {
        self.gen_assignment(Assignment {
            meta: Default::default(),
            var: VariableAccess::Identifier {
                meta: Default::default(),
                value: var.clone(),
            },
            rhs: Expression::Simple {
                meta: Default::default(),
                value: SimpleExpression::AddExpr {
                    meta: Default::default(),
                    lhs: Box::new(SimpleExpression::Term {
                        meta: Default::default(),
                        value: Term::Factor {
                            meta: Default::default(),
                            value: Factor::Exponentiation {
                                meta: Default::default(),
                                value: Exponentiation::Primary {
                                    meta: Default::default(),
                                    value: Primary::VariableAccess {
                                        meta: Default::default(),
                                        value: VariableAccess::Identifier {
                                            meta: Default::default(),
                                            value: var,
                                        },
                                    },
                                },
                            },
                        },
                    }),
                    op: AddOp::Add,
                    rhs: Term::Factor {
                        meta: Default::default(),
                        value: Factor::Exponentiation {
                            meta: Default::default(),
                            value: Exponentiation::Primary {
                                meta: Default::default(),
                                value: Primary::UnsignedConstant {
                                    meta: Default::default(),
                                    value: UnsignedConstant::Number {
                                        meta: Default::default(),
                                        value: Number::Integer {
                                            meta: Default::default(),
                                            value: 1,
                                        },
                                    },
                                },
                            },
                        },
                    },
                },
            },
        })
    }

    unsafe fn gen_decrement(&mut self, var: String) -> Result<LLVMValueRef, String> {
        self.gen_assignment(Assignment {
            meta: Default::default(),
            var: VariableAccess::Identifier {
                meta: Default::default(),
                value: var.clone(),
            },
            rhs: Expression::Simple {
                meta: Default::default(),
                value: SimpleExpression::AddExpr {
                    meta: Default::default(),
                    lhs: Box::new(SimpleExpression::Term {
                        meta: Default::default(),
                        value: Term::Factor {
                            meta: Default::default(),
                            value: Factor::Exponentiation {
                                meta: Default::default(),
                                value: Exponentiation::Primary {
                                    meta: Default::default(),
                                    value: Primary::VariableAccess {
                                        meta: Default::default(),
                                        value: VariableAccess::Identifier {
                                            meta: Default::default(),
                                            value: var,
                                        },
                                    },
                                },
                            },
                        },
                    }),
                    op: AddOp::Sub,
                    rhs: Term::Factor {
                        meta: Default::default(),
                        value: Factor::Exponentiation {
                            meta: Default::default(),
                            value: Exponentiation::Primary {
                                meta: Default::default(),
                                value: Primary::UnsignedConstant {
                                    meta: Default::default(),
                                    value: UnsignedConstant::Number {
                                        meta: Default::default(),
                                        value: Number::Integer {
                                            meta: Default::default(),
                                            value: 1,
                                        },
                                    },
                                },
                            },
                        },
                    },
                },
            },
        })
    }

    unsafe fn gen_closed_statement(
        &mut self,
        stmt: ClosedStatement,
    ) -> Result<LLVMValueRef, String> {
        // assignment, empty, for, funcall
        match stmt {
            ClosedStatement::Assignment { meta: _, value: a } => self.gen_assignment(a),
            ClosedStatement::For {
                meta: _,
                var,
                init,
                inc,
                fin,
                s,
            } => self.gen_for(var, init, inc, fin, *s),
            ClosedStatement::Procedure {
                meta: _,
                id,
                params,
            } => self.gen_function_call(id, &params),
            ClosedStatement::Compound {
                meta: _,
                value: stmts,
            } => self.gen_statements(stmts),
            ClosedStatement::Empty { meta: _ } => Ok(ptr::null_mut()),
            _ => Err(format!("Statement {:?} not supported", stmt)),
        }
    }

    unsafe fn gen_assignment(&mut self, a: Assignment) -> Result<LLVMValueRef, String> {
        let lhs = self.gen_variable_access(&a.var, true)?;
        let rhs = self.gen_expression(&a.rhs)?;
        let rhs_v = self.load_if_needed(rhs);
        let r = LLVMBuildStore(self.builder, rhs_v, lhs);
        Ok(r)
    }

    unsafe fn gen_function_call(
        &mut self,
        id: Identifier,
        params: &Vec<Parameter>,
    ) -> Result<LLVMValueRef, String> {
        if id == "writeln" {
            let printf_var = self.global_varmap.get("printf").unwrap().llvm_val;
            let param = self.gen_expression(&params[0])?;
            let param_v = self.load_if_needed(param);
            use llvm::LLVMTypeKind;
            let (format, p) = match LLVMGetTypeKind(LLVMTypeOf(param_v)) {
                LLVMTypeKind::LLVMDoubleTypeKind | LLVMTypeKind::LLVMFloatTypeKind => (
                    b"%f\n\0",
                    LLVMBuildFPExt(
                        self.builder,
                        param_v,
                        LLVMDoubleType(),
                        CString::new("float_cast").unwrap().as_ptr(),
                    ),
                ),
                LLVMTypeKind::LLVMIntegerTypeKind => (
                    b"%d\n\0",
                    LLVMBuildZExtOrBitCast(
                        self.builder,
                        param_v,
                        LLVMInt32Type(),
                        CString::new("int_cast").unwrap().as_ptr(),
                    ),
                ),
                _ => return Err("Printing unsupported type".to_string()),
            };
            let format_str = LLVMBuildGlobalStringPtr(
                self.builder,
                format.as_ptr() as *const _,
                b".printf_format\0".as_ptr() as *const _,
            );
            let f = LLVMBuildZExtOrBitCast(
                self.builder,
                format_str,
                LLVMPointerType(LLVMInt8Type(), 0),
                CString::new("cast_format").unwrap().as_ptr(),
            );
            LLVMBuildCall(
                self.builder,
                printf_var,
                vec![f, p].as_mut_slice().as_mut_ptr(),
                2,
                b"\0".as_ptr() as *const _,
            );
            Ok(ptr::null_mut())
        } else {
            Err("Only writeln is implemented".to_string())
        }
    }

    unsafe fn val_to_bool(&mut self, val: LLVMValueRef) -> LLVMValueRef {
        match LLVMGetTypeKind(LLVMTypeOf(val)) {
            llvm::LLVMTypeKind::LLVMDoubleTypeKind | llvm::LLVMTypeKind::LLVMFloatTypeKind => {
                LLVMBuildFCmp(
                    self.builder,
                    llvm::LLVMRealPredicate::LLVMRealONE,
                    val,
                    LLVMConstNull(LLVMTypeOf(val)),
                    CString::new("to_bool").unwrap().as_ptr(),
                )
            }
            _ => LLVMBuildICmp(
                self.builder,
                llvm::LLVMIntPredicate::LLVMIntNE,
                val,
                LLVMConstNull(LLVMTypeOf(val)),
                CString::new("to_bool").unwrap().as_ptr(),
            ),
        }
    }

    unsafe fn load_if_needed(&mut self, v: LLVMValueRef) -> LLVMValueRef {
        match LLVMGetTypeKind(LLVMTypeOf(v)) {
            llvm::LLVMTypeKind::LLVMPointerTypeKind => {
                LLVMBuildLoad(self.builder, v, CString::new("load_ptr").unwrap().as_ptr())
            }
            _ => v,
        }
    }
}
