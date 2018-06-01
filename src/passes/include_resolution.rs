use std::path::{PathBuf};
use std::collections::{HashMap, HashSet};
use ast::{Location, TUId};
use parser::{parse_file, ParseTree};
use errors::Errors;

pub struct IncludeResolver<'a> {
    include_dirs: &'a [PathBuf],
    include_files: HashMap<String, PathBuf>,
    id_file_map: TUIdFileMap,
}

impl<'a> IncludeResolver<'a> {
    pub fn new(include_dirs: &'a [PathBuf]) -> IncludeResolver<'a> {
        IncludeResolver {
            include_dirs,
            include_files: HashMap::new(),
            id_file_map: TUIdFileMap::new(),
        }
    }

    pub fn get_include(&self, include_name: &str) -> Option<TUId> {
        match self.include_files.get(include_name) {
            Some(ref path) => self.id_file_map.get_tuid(path),
            None => None
        }
    }

    pub fn resolve_include<'b>(&'b mut self, include_name: &str) -> Option<TUId> {
        if let Some(ref include_file_path) = self.include_files.get(include_name) {
            return Some(self.id_file_map.resolve_file_name(include_file_path));
        }

        // XXX The Python parser also checks '' for some reason.
        for include_dir in self.include_dirs {
            let mut new_include_path = include_dir.to_path_buf();
            new_include_path.push(include_name);

            if new_include_path.exists() {
                if let Ok(canonical_new_include_path) = new_include_path.canonicalize() {
                    let new_id = self.id_file_map.resolve_file_name(&canonical_new_include_path);
                    self.include_files.insert(String::from(include_name), canonical_new_include_path);
                    return Some(new_id);
                }
            }
        }

        None
    }

    fn print_include_context(include_context: &[PathBuf]) {
        for path in include_context {
            println!("  in file included from `{}':", path.display());
        }
    }

    #[allow(needless_pass_by_value)]
    pub fn resolve_includes(&mut self, parse_tree: ParseTree) -> Result<(TUId, HashMap<TUId, ParseTree>), Errors> {
        let canonical_file_path = match parse_tree.file_path.canonicalize() {
            Ok(cfp) => cfp,
            Err(_) => {
                return Err(Errors::one(&Location { file_name: parse_tree.file_path.clone(), lineno: 0, colno: 0}, &format!("can't locate file specified on the command line `{}'", parse_tree.file_path.display())))
            },
        };

        let mut work_list : Vec<(PathBuf, Vec<PathBuf>)> = Vec::new();
        let mut parsed_files = HashMap::new();
        let mut visited_files = HashSet::new();

        let file_id = self.id_file_map.resolve_file_name(&canonical_file_path);
        visited_files.insert(file_id);
        work_list.push((canonical_file_path.clone(), Vec::new()));

        while !work_list.is_empty() {
            let mut new_work_list = Vec::new();
            for (curr_file, include_context) in work_list {
                let curr_parse_tree = if *curr_file == canonical_file_path {
                    parse_tree.clone()
                } else {
                    match parse_file(&curr_file) {
                        Ok(tu) => tu,
                        Err(err) => {
                            Self::print_include_context(&include_context);
                            return Err(Errors::from(err))
                        }
                    }
                };

                let mut include_errors = Errors::none();

                for include in &curr_parse_tree.translation_unit.data.includes {
                    let include_filename = format!("{}{}{}", include.data.id.data, ".ipdl", if include.data.protocol.is_some() {""} else {"h"});
                    let include_id = match self.resolve_include(&include_filename) {
                        Some(tuid) => tuid,
                        None => {
                            include_errors.append_one(&Location { file_name: PathBuf::from(include_filename.clone()), lineno: 0, colno: 0 }, &format!("Cannot resolve include {}", include_filename));
                            continue
                        }
                    };

                    if visited_files.contains(&include_id) {
                        continue;
                    }

                    let mut new_include_context = include_context.clone();
                    new_include_context.push(curr_file.clone());

                    visited_files.insert(include_id);
                    new_work_list.push((self.include_files.get(&include_filename).expect("Resolve include is broken").clone(), new_include_context));
                }

                if !include_errors.is_empty() {
                    return Err(include_errors);
                }

                let curr_id = self.id_file_map.resolve_file_name(&curr_file);
                parsed_files.insert(curr_id, curr_parse_tree);
            }

            work_list = new_work_list;
        }

        Ok((file_id, parsed_files))
    }

}

pub struct TUIdFileMap {
    next_id: TUId,
    file_ids: HashMap<PathBuf, TUId>,
    id_files: HashMap<TUId, PathBuf>,
}

impl TUIdFileMap {
    fn new() -> TUIdFileMap {
        TUIdFileMap {
            next_id: 0,
            file_ids: HashMap::new(),
            id_files: HashMap::new(),
        }
    }

    fn get_tuid(&self, path: &PathBuf) -> Option<TUId> {
        self.file_ids.get(path).cloned()
    }

    fn resolve_file_name(&mut self, path: &PathBuf) -> TUId {
        if let Some(&id) = self.file_ids.get(path) {
            return id;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.id_files.insert(id, path.clone());
        self.file_ids.insert(path.clone(), id);
        id
    }
}