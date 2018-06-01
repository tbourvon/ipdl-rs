/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use pipdl;

use passes::include_resolution::IncludeResolver;
use passes::parsetree_to_tu::ParseTreeToTU;
use passes::type_check;

use errors;

use std::collections::HashMap;

use ast::AST;

#[derive(Clone)]
pub struct ParseTree {
    pub translation_unit: pipdl::Spanned<pipdl::TranslationUnit>,
    pub file_path: PathBuf,
}

pub fn parse_file(file_path: &Path) -> Result<ParseTree, pipdl::Error> {
    let mut file = File::open(file_path).expect("Cannot open file for parsing");

    let mut file_text = String::new();
    file.read_to_string(&mut file_text)
        .expect("Cannot read file for parsing");

    let parse_tree = ParseTree {
        translation_unit: pipdl::parse(&file_text, file_path)?,
        file_path: file_path.to_path_buf(),
    };

    Ok(parse_tree)
}

pub fn parse(file_path: &Path, include_dirs: &[PathBuf]) -> Result<AST, errors::Errors> {
    let parse_tree = parse_file(file_path)?;

    let mut include_resolver = IncludeResolver::new(include_dirs);

    let (main_tuid, result) = include_resolver.resolve_includes(parse_tree)?;

    let parsetree_to_translation_unit = ParseTreeToTU::new(&include_resolver);

    let ast = AST {
        main_tuid,
        translation_units: {
            result
                .into_iter()
                .map(|(tuid, parse_tree)| {
                    Ok((
                        tuid,
                        parsetree_to_translation_unit.parsetree_to_translation_unit(parse_tree)?,
                    ))
                })
                .collect::<Result<HashMap<_, _>, errors::Errors>>()?
        },
    };

    type_check::check(&ast.translation_units)?;

    Ok(ast)
}
