# pascal-compiler
Course project for compiler construction class. Implemented part of ISO 7185:1990 Standard Pascal grammar enough to compile bubble sort on integers. Grammar was copied and modified from LALRPOP example grammars.

Technology stack: Rust, LALRPOP, LLVM.

Five commands are available:
- lex - prints all lexemes found in file
- parse - prints AST
- symbols - prints symbol table
- typecheck - performs basic typecheck
- codegen - generates LLVM IR for input program
