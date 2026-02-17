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

    // Type check
    let checked = ag_checker::check(&parsed.module);
    if !checked.diagnostics.is_empty() {
        for diag in &checked.diagnostics {
            print_diagnostic(input_path, &source, diag);
        }
        process::exit(1);
    }

    // Codegen
    let js = ag_codegen::codegen(&parsed.module);

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

    let checked = ag_checker::check(&parsed.module);
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
