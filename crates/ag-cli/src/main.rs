use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: asc <command> <file.ag> [options]");
        eprintln!("Commands:");
        eprintln!("  build <file.ag> [-o <output>]  Compile to JavaScript");
        eprintln!("  check <file.ag>                Type check only");
        process::exit(1);
    }

    let command = &args[1];
    match command.as_str() {
        "build" => cmd_build(&args[2..]),
        "check" => cmd_check(&args[2..]),
        _ => {
            eprintln!("Unknown command: {}", command);
            process::exit(1);
        }
    }
}

fn cmd_build(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: asc build <file.ag> [-o <output>]");
        process::exit(1);
    }

    let input_path = &args[0];
    let output_path = parse_output_flag(args).unwrap_or_else(|| {
        let p = Path::new(input_path);
        p.with_extension("js").to_string_lossy().to_string()
    });

    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    // Lex + Parse
    let parsed = ag_parser::parse(&source);
    if !parsed.diagnostics.is_empty() {
        for diag in &parsed.diagnostics {
            print_diagnostic(input_path, &source, diag);
        }
        process::exit(1);
    }

    // Resolve std: imports
    let mut module = parsed.module;
    if let Err(errs) = resolve_std_imports(&mut module) {
        for msg in errs {
            eprintln!("{}: error: {}", input_path, msg);
        }
        process::exit(1);
    }

    // Type check
    let checked = ag_checker::check(&module);
    if !checked.diagnostics.is_empty() {
        for diag in &checked.diagnostics {
            print_diagnostic(input_path, &source, diag);
        }
        process::exit(1);
    }

    // Codegen
    let js = ag_codegen::codegen(&module);

    if let Err(e) = fs::write(&output_path, &js) {
        eprintln!("error: cannot write '{}': {}", output_path, e);
        process::exit(1);
    }

    eprintln!("compiled {} -> {}", input_path, output_path);
}

fn cmd_check(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: asc check <file.ag>");
        process::exit(1);
    }

    let input_path = &args[0];
    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    let parsed = ag_parser::parse(&source);
    if !parsed.diagnostics.is_empty() {
        for diag in &parsed.diagnostics {
            print_diagnostic(input_path, &source, diag);
        }
        process::exit(1);
    }

    // Resolve std: imports
    let mut module = parsed.module;
    if let Err(errs) = resolve_std_imports(&mut module) {
        for msg in errs {
            eprintln!("{}: error: {}", input_path, msg);
        }
        process::exit(1);
    }

    let checked = ag_checker::check(&module);
    if !checked.diagnostics.is_empty() {
        for diag in &checked.diagnostics {
            print_diagnostic(input_path, &source, diag);
        }
        process::exit(1);
    }

    eprintln!("{}: ok", input_path);
}

fn parse_output_flag(args: &[String]) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == "-o" && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

/// Resolves `std:` prefixed imports by parsing stdlib module sources
/// and injecting their declarations into the module.
fn resolve_std_imports(module: &mut ag_ast::Module) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut injected_items = Vec::new();

    let std_imports: Vec<(usize, ag_ast::Import)> = module
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            if let ag_ast::Item::Import(imp) = item {
                if imp.path.starts_with("std:") {
                    return Some((i, imp.clone()));
                }
            }
            None
        })
        .collect();

    for (_, imp) in &std_imports {
        let std_path = &imp.path[4..]; // strip "std:" prefix
        match ag_stdlib::resolve_std_module(std_path) {
            Some(source) => {
                let parsed = ag_parser::parse(source);
                if !parsed.diagnostics.is_empty() {
                    for diag in &parsed.diagnostics {
                        errors.push(format!(
                            "error in stdlib module `{}`: {}",
                            std_path, diag.message
                        ));
                    }
                    continue;
                }

                // If selective import, only inject requested names
                if !imp.names.is_empty() {
                    let requested: std::collections::HashSet<&str> =
                        imp.names.iter().map(|n| n.name.as_str()).collect();
                    for item in parsed.module.items {
                        let name = match &item {
                            ag_ast::Item::ExternFnDecl(ef) => Some(ef.name.as_str()),
                            ag_ast::Item::ExternStructDecl(es) => Some(es.name.as_str()),
                            ag_ast::Item::ExternTypeDecl(et) => Some(et.name.as_str()),
                            _ => None,
                        };
                        if let Some(n) = name {
                            if requested.contains(n) {
                                injected_items.push(item);
                            }
                        }
                    }
                    // Check for unknown imports
                    let available: std::collections::HashSet<String> = injected_items
                        .iter()
                        .filter_map(|item| match item {
                            ag_ast::Item::ExternFnDecl(ef) => Some(ef.name.clone()),
                            ag_ast::Item::ExternStructDecl(es) => Some(es.name.clone()),
                            ag_ast::Item::ExternTypeDecl(et) => Some(et.name.clone()),
                            _ => None,
                        })
                        .collect();
                    for name in &imp.names {
                        if !available.contains(&name.name) {
                            errors.push(format!(
                                "`{}` is not exported by module `std:{}`",
                                name.name, std_path
                            ));
                        }
                    }
                } else {
                    // Import all declarations
                    for item in parsed.module.items {
                        match &item {
                            ag_ast::Item::ExternFnDecl(_)
                            | ag_ast::Item::ExternStructDecl(_)
                            | ag_ast::Item::ExternTypeDecl(_) => {
                                injected_items.push(item);
                            }
                            _ => {}
                        }
                    }
                }
            }
            None => {
                errors.push(format!("unknown standard library module `std:{}`", std_path));
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Remove std: import items from the module (they're resolved at compile time)
    let std_indices: std::collections::HashSet<usize> =
        std_imports.iter().map(|(i, _)| *i).collect();
    let mut new_items: Vec<ag_ast::Item> = module
        .items
        .drain(..)
        .enumerate()
        .filter(|(i, _)| !std_indices.contains(i))
        .map(|(_, item)| item)
        .collect();

    // Prepend injected declarations
    injected_items.append(&mut new_items);
    module.items = injected_items;

    Ok(())
}

fn print_diagnostic(file: &str, source: &str, diag: &ag_ast::Diagnostic) {
    let (line, col) = offset_to_line_col(source, diag.span.start as usize);
    eprintln!("{}:{}:{}: error: {}", file, line, col, diag.message);
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
