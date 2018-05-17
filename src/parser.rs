use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;

use pipdl;

use passes::include_resolution::IncludeResolver;
use passes::parsetree_to_ast::ParseTreeToAST;
use passes::type_check;

use errors;

use std::collections::HashMap;

#[derive(Clone)]
pub struct ParseTree {
    pub translation_unit: pipdl::Spanned<pipdl::TranslationUnit>,
    pub file_path: PathBuf,
}

pub fn parse_file(file_path: &Path) -> Result<ParseTree, pipdl::Error> {
    let mut file = File::open(file_path).expect("Cannot open file for parsing");

    let mut file_text = String::new();
    file.read_to_string(&mut file_text).expect("Cannot read file for parsing");

    let parse_tree = ParseTree {
        translation_unit: pipdl::parse(&file_text, file_path)?,
        file_path: file_path.to_path_buf(),
    };

    Ok(parse_tree)
}

pub fn parse(file_path: &Path, include_dirs: &[PathBuf]) -> Result<(), errors::Errors> {
    let parse_tree = parse_file(file_path)?;

    let mut include_resolver = IncludeResolver::new(include_dirs);

    let result = include_resolver.resolve_includes(parse_tree)?;

    let parsetree_to_ast = ParseTreeToAST::new(&include_resolver);

    let result = result.into_iter().map(|(tuid, parse_tree)| {
        Ok((tuid, parsetree_to_ast.parsetree_to_ast(parse_tree)?.translation_unit))
    }).collect::<Result<HashMap<_, _>, errors::Errors>>()?;

    type_check::check(&result)?;

    Ok(())
}