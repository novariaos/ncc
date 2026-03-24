pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod parser;
pub mod preprocess;
pub mod token;
pub mod types;

use std::path::{Path, PathBuf};

pub fn compile(source: &str, file_path: &Path, include_dirs: &[PathBuf]) -> Result<String, String> {
    let mut pp = preprocess::Preprocessor::new(include_dirs.to_vec());
    let processed = pp.process(source, file_path)?;

    let mut lex = lexer::Lexer::new(&processed);
    let tokens = lex.tokenize()?;

    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse_program()?;

    let types = types::TypeContext::build(&program)?;

    let asm_text = codegen::Codegen::generate(&program, types)?;

    Ok(asm_text)
}
