use crate::ast::*;

extern crate libc;

use std::collections::HashMap;
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
}

impl VarInfo {
    fn new(typ: Type, llvm_typ: LLVMTypeRef, llvm_val: LLVMValueRef) -> VarInfo {
        VarInfo {
            typ: typ,
            llvm_typ: llvm_typ,
            llvm_val: llvm_val,
        }
    }
}

pub struct Codegen {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    global_varmap: HashMap<String, VarInfo>,
    cur_func: Option<LLVMValueRef>,
    printf: LLVMValueRef,
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
        let global_varmap = HashMap::new();

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

        Codegen {
            context: context,
            module: module,
            builder: LLVMCreateBuilderInContext(context),
            global_varmap: global_varmap,
            cur_func: None,
            printf: printf,
        }
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
        self.gen_variable_decls(p.block.variable_decls)?;

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

    unsafe fn gen_variable_decls(&mut self, vars: Vec<VariableDecl>) -> Result<(), String> {
        for v in vars.into_iter() {
            self.gen_variable_decl(v)?;
        }
        Ok(())
    }

    unsafe fn gen_variable_decl(&mut self, var: VariableDecl) -> Result<(), String> {
        let ids = var.ids.clone();
        let var_typ = var.typ.clone();
        match var_typ {
            Type::Identifier(i) => {
                let llvmtyp = self.identifier_to_llvmty(&i);
                for id in ids {
                    let gvar = LLVMAddGlobal(
                        self.module,
                        self.identifier_to_llvmty(&i),
                        CString::new(id.as_str()).unwrap().as_ptr(),
                    );
                    self.global_varmap.insert(
                        id.to_string(),
                        VarInfo::new(Type::Identifier(i.to_string()), llvmtyp, gvar),
                    );
                    LLVMSetLinkage(gvar, llvm::LLVMLinkage::LLVMCommonLinkage);
                    LLVMSetInitializer(gvar, LLVMConstNull(self.identifier_to_llvmty(&i)));
                }
            }
            Type::NewType(nt) => match *nt {
                NewType::StructuredType(tt) => match &*tt {
                    StructuredType::Array { index_list, typ } => {
                        let t = match *typ.clone() {
                            Type::Identifier(t) => self.identifier_to_llvmty(&t),
                            _ => return Err("Array of complex values".to_string()),
                        };
                        let size = match &index_list[0] {
                            OrdinalType::NewOrdinalType(NewOrdinalType::SubrangeType {
                                low: _, /* consider low to be always 0 */
                                high,
                            }) => {
                                if let Constant::NonString(NonString::Integer(x)) = high {
                                    x
                                } else {
                                    return Err("Array of non-integer size".to_string());
                                }
                            },
                            _ => return Err("Arrays with types other than integers as indices are not supported".to_string())
                        };
                        let llvmtyp = LLVMArrayType(t, *size as u32);
                        for id in var.ids {
                            let gvar = LLVMAddGlobal(
                                self.module,
                                LLVMArrayType(t, *size as u32),
                                CString::new(id.as_str()).unwrap().as_ptr(),
                            );
                            let typ = Type::NewType(Box::new(NewType::StructuredType(Box::new(
                                StructuredType::Array {
                                    index_list: index_list.clone(),
                                    typ: typ.clone(),
                                },
                            ))));
                            self.global_varmap
                                .insert(id, VarInfo::new(typ, llvmtyp, gvar));
                            LLVMSetLinkage(gvar, llvm::LLVMLinkage::LLVMCommonLinkage);
                            LLVMSetInitializer(gvar, LLVMConstNull(LLVMArrayType(t, *size as u32)));
                        }
                    }
                    _ => return Err("Types harder than arrays are not supported".to_string()),
                },
                _ => return Err("Types harder than arrays are not supported".to_string()),
            },
        };
        Ok(())
    }

    pub unsafe fn identifier_to_llvmty(&mut self, id: &String) -> LLVMTypeRef {
        match id.as_ref() {
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

    pub unsafe fn gen_statements(&mut self, stmts: Vec<Statement>) -> Result<LLVMValueRef, String> {
        for s in stmts {
            self.gen_statement(s)?;
        }
        Ok(ptr::null_mut())
    }

    pub unsafe fn gen_statement(&mut self, stmt: Statement) -> Result<LLVMValueRef, String> {
        match stmt {
            Statement::Open { label: _, s } => self.gen_open_statement(s),
            Statement::Closed { label: _, s } => self.gen_closed_statement(s),
        }
    }

    pub unsafe fn gen_open_statement(
        &mut self,
        stmt: OpenStatement,
    ) -> Result<LLVMValueRef, String> {
        // for and if
        match stmt {
            OpenStatement::If(open_if) => self.gen_open_if(open_if),
            OpenStatement::For {
                var,
                init,
                inc,
                fin,
                s,
            } => self.gen_for(var, init, inc, fin, *s),
            _ => Err(format!("Statement {:?} not supported", stmt)),
        }
    }

    pub unsafe fn gen_open_if(&mut self, typ: OpenIf) -> Result<LLVMValueRef, String> {
        match typ {
            OpenIf::WithoutElse { predicate, then } => {
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

    pub unsafe fn gen_expression(&mut self, e: &Expression) -> Result<LLVMValueRef, String> {
        match e {
            Expression::RelExpr { lhs, op, rhs } => self.gen_rel_expr(&*lhs, op, &*rhs),
            Expression::Simple(expr) => self.gen_simple_expr(expr),
        }
    }

    pub unsafe fn gen_rel_expr(
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

    pub unsafe fn gen_simple_expr(&mut self, e: &SimpleExpression) -> Result<LLVMValueRef, String> {
        match e {
            SimpleExpression::AddExpr { lhs, op, rhs } => self.gen_add_expr(&*lhs, op, &*rhs),
            SimpleExpression::Term(t) => self.gen_term(t),
        }
    }

    pub unsafe fn gen_add_expr(
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

    pub unsafe fn gen_term(&mut self, e: &Term) -> Result<LLVMValueRef, String> {
        match e {
            Term::MulExpr { lhs, op, rhs } => self.gen_mul_expr(&*lhs, op, &*rhs),
            Term::Factor(f) => self.gen_factor(f),
        }
    }

    pub unsafe fn gen_mul_expr(
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

    pub unsafe fn gen_factor(&mut self, e: &Factor) -> Result<LLVMValueRef, String> {
        match e {
            Factor::Factor { is_negative, expr } => self.gen_unary(*is_negative, &*expr),
            Factor::Exponentiation(e) => self.gen_exponentiation(e),
        }
    }

    pub unsafe fn gen_unary(
        &mut self,
        is_negative: bool,
        f: &Factor,
    ) -> Result<LLVMValueRef, String> {
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

    pub unsafe fn gen_exponentiation(
        &mut self,
        e: &Exponentiation,
    ) -> Result<LLVMValueRef, String> {
        match e {
            Exponentiation::Primary(p) => self.gen_primary(p),
            _ => Err(format!("Operation {:?} not supported", e)),
        }
    }

    pub unsafe fn gen_primary(&mut self, p: &Primary) -> Result<LLVMValueRef, String> {
        match p {
            Primary::VariableAccess(va) => self.gen_variable_access(&va),
            Primary::UnsignedConstant(uc) => self.gen_unsigned_constant(&uc),
            Primary::Paren(e) => self.gen_expression(&*e),
            _ => Err(format!("Operation {:?} not supported", p)),
        }
    }

    pub unsafe fn gen_variable_access(
        &mut self,
        v: &VariableAccess,
    ) -> Result<LLVMValueRef, String> {
        match v {
            VariableAccess::Identifier(i) => {
                if let Some(varinfo) = self.global_varmap.get(i.as_str()) {
                    return Ok(varinfo.clone().llvm_val);
                }
                Err("Use of undeclared variable".to_string())
            }
            VariableAccess::Indexed { var, index } => {
                let i = self.gen_expression(&*index)?;
                let v = self.gen_variable_access(var)?;
                let i_load = self.load_if_needed(i);
                let zero =
                    self.gen_unsigned_constant(&UnsignedConstant::Number(Number::Integer(0)))?;
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

    pub unsafe fn gen_unsigned_constant(
        &mut self,
        c: &UnsignedConstant,
    ) -> Result<LLVMValueRef, String> {
        match c {
            UnsignedConstant::Number(n) => match n {
                Number::Integer(i) => Ok(LLVMConstInt(LLVMInt32Type(), *i as u64, 0)),
                Number::Real(r) => Ok(LLVMConstReal(LLVMFloatType(), *r as f64)),
            },
            UnsignedConstant::String(s) => Ok(LLVMBuildGlobalStringPtr(
                self.builder,
                CString::new(s.as_str()).unwrap().as_ptr(),
                CString::new("str").unwrap().as_ptr(),
            )),
            UnsignedConstant::Nil => Err("No nil".to_string()),
        }
    }

    pub unsafe fn gen_for(
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
            var: VariableAccess::Identifier(var.clone()),
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
            self.gen_assignment(Assignment {
                var: VariableAccess::Identifier(var.clone()),
                rhs: Expression::Simple(SimpleExpression::AddExpr {
                    lhs: Box::new(SimpleExpression::Term(Term::Factor(
                        Factor::Exponentiation(Exponentiation::Primary(Primary::VariableAccess(
                            VariableAccess::Identifier(var),
                        ))),
                    ))),
                    op: AddOp::Add,
                    rhs: Term::Factor(Factor::Exponentiation(Exponentiation::Primary(
                        Primary::UnsignedConstant(UnsignedConstant::Number(Number::Integer(1))),
                    ))),
                }),
            })?;
        } else {
            self.gen_assignment(Assignment {
                var: VariableAccess::Identifier(var.clone()),
                rhs: Expression::Simple(SimpleExpression::AddExpr {
                    lhs: Box::new(SimpleExpression::Term(Term::Factor(
                        Factor::Exponentiation(Exponentiation::Primary(Primary::VariableAccess(
                            VariableAccess::Identifier(var),
                        ))),
                    ))),
                    op: AddOp::Sub,
                    rhs: Term::Factor(Factor::Exponentiation(Exponentiation::Primary(
                        Primary::UnsignedConstant(UnsignedConstant::Number(Number::Integer(1))),
                    ))),
                }),
            })?;
        }
        LLVMBuildBr(self.builder, bb_test);

        LLVMPositionBuilderAtEnd(self.builder, bb_exit);
        Ok(ptr::null_mut())
    }

    pub unsafe fn gen_closed_statement(
        &mut self,
        stmt: ClosedStatement,
    ) -> Result<LLVMValueRef, String> {
        // assignment, empty, for, funcall
        match stmt {
            ClosedStatement::Assignment(a) => self.gen_assignment(a),
            ClosedStatement::For {
                var,
                init,
                inc,
                fin,
                s,
            } => self.gen_for(var, init, inc, fin, *s),
            ClosedStatement::Procedure { id, params } => self.gen_function_call(id, &params),
            ClosedStatement::Compound(stmts) => self.gen_statements(stmts),
            ClosedStatement::Empty => Ok(ptr::null_mut()),
            _ => Err(format!("Statement {:?} not supported", stmt)),
        }
    }

    pub unsafe fn gen_assignment(&mut self, a: Assignment) -> Result<LLVMValueRef, String> {
        let lhs = self.gen_variable_access(&a.var)?;
        let rhs = self.gen_expression(&a.rhs)?;
        let rhs_v = self.load_if_needed(rhs);
        let r = LLVMBuildStore(self.builder, rhs_v, lhs);
        Ok(r)
    }

    pub unsafe fn gen_function_call(
        &mut self,
        id: Identifier,
        params: &Vec<Parameter>,
    ) -> Result<LLVMValueRef, String> {
        if id == "writeln" {
            let printf_var = self.printf;
            let format = b"%d\n\0";
            let format_str = LLVMBuildGlobalStringPtr(
                self.builder,
                format.as_ptr() as *const _,
                b".int_format\0".as_ptr() as *const _,
            );
            let param = self.gen_expression(&params[0])?;
            let f = LLVMBuildZExtOrBitCast(
                self.builder,
                format_str,
                LLVMPointerType(LLVMInt8Type(), 0),
                CString::new("cast_format").unwrap().as_ptr(),
            );
            let param_v = self.load_if_needed(param);
            let p = LLVMBuildZExtOrBitCast(
                self.builder,
                param_v,
                LLVMInt32Type(),
                CString::new("cast_int").unwrap().as_ptr(),
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
