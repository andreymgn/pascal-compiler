use crate::ast::*;

use std::collections::HashMap;
use std::ffi::CString;

extern crate llvm_sys as llvm;

use self::llvm::core::*;
use self::llvm::prelude::*;

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
    local_varmap: Vec<HashMap<String, VarInfo>>,
    label_map: HashMap<String, LLVMBasicBlockRef>,
    cur_func: Option<LLVMValueRef>,
}

impl Codegen {
    pub unsafe fn new(p: Program) -> Codegen {
        llvm::execution_engine::LLVMLinkInInterpreter();
        llvm::target::LLVM_InitializeAllTargetMCs();
        llvm::target::LLVM_InitializeNativeTarget();
        llvm::target::LLVM_InitializeNativeAsmPrinter();
        llvm::target::LLVM_InitializeNativeAsmParser();

        let context = LLVMContextCreate();

        let c_mod_name = CString::new(p.name).unwrap();
        let module = LLVMModuleCreateWithNameInContext(c_mod_name.as_ptr(), context);
        let global_varmap = HashMap::new();

        let mut ee = 0 as llvm::execution_engine::LLVMExecutionEngineRef;
        let mut error = 0 as *mut i8;
        if llvm::execution_engine::LLVMCreateExecutionEngineForModule(&mut ee, module, &mut error)
            != 0
        {
            println!("err");
        }

        Codegen {
            context: context,
            module: module,
            builder: LLVMCreateBuilderInContext(context),
            global_varmap: global_varmap,
            local_varmap: Vec::new(),
            label_map: HashMap::new(),
            cur_func: None,
        }
    }

    pub unsafe fn run(&mut self, p: Program) -> Result<(), String> {
        let res = self.gen_program(p);
        res
    }

    unsafe fn gen_program(&mut self, p: Program) -> Result<(), String> {
        let err = self.gen_variable_decls(p.block.variable_decls).is_err();
        Ok(())
    }

    unsafe fn gen_variable_decls(&mut self, vars: Vec<VariableDecl>) -> Result<(), String> {
        for v in vars {
            self.gen_variable_decl(v);
        }
        Ok(())
    }

    unsafe fn gen_variable_decl(&mut self, var: VariableDecl) -> Result<(), String> {
        match var.typ {
            Type::Identifier(t) => {
                let llvmtyp = self.identifier_to_llvmty(&t);
                for id in var.ids {
                    let gvar = LLVMAddGlobal(
                        self.module,
                        self.identifier_to_llvmty(&t),
                        CString::new(id.as_str()).unwrap().as_ptr(),
                    );
                }
            }
            Type::NewType(t) => match *t {
                NewType::StructuredType(tt) => match *tt {
                    StructuredType::Array { index_list, typ } => {
                        let t = match *typ {
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
                        LLVMArrayType(t, *size as u32);
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
}
