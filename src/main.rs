use std::sync::Arc;

use anyhow::Context;
use node_ext::NodeExt;
use query_struct::FunctionDefinitionQuery;

mod format;
mod lint;
mod node_ext;
mod query_struct;
mod util;

fn run_checks(root: tree_sitter::Node, source: Arc<str>) {
    static CHECK_FNS: &[lint::CheckFn] = &[
        lint::check_class_name_extends,
        lint::check_export_var_order,
        lint::check_typed_function_signature,
        lint::check_no_print_call,
    ];

    assert!(root.kind() == "source", "Expected 'source' node");

    let mut reports = Vec::new();
    for check_fn in CHECK_FNS {
        reports.extend(check_fn(root, Arc::clone(&source)));
    }

    if reports.is_empty() {
        tracing::info!("✅ You're good to go!");
    } else {
        for report in &reports {
            println!("{:?}", report);
        }
        tracing::error!("❌ Found {} issues.", reports.len());
    }
}

fn main() -> anyhow::Result<()> {
    use owo_colors::OwoColorize;

    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("Starting GDScript parser...");

    dotenvy::dotenv().context("failed to load .env file")?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_gdscript::LANGUAGE.into())
        .context("setting tree-sitter language")?;

    let project_root = dotenvy::var("PROJECT_ROOT")?;
    println!("Project root: {}", project_root);

    let files = walkdir::WalkDir::new(&project_root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().is_some_and(|ext| ext == "gd")
                && !entry
                    .path()
                    .to_str()
                    .is_some_and(|f| f.contains("/addons/"))
        })
        .filter(|entry| {
            entry
                .path()
                .to_str()
                .is_some_and(|name| name.ends_with("ranged_weapon.gd"))
        })
        .map(|entry| {
            std::fs::read_to_string(entry.path())
                .map(Arc::<str>::from)
                .map(|content| (entry, content))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let sep = "-".repeat(80);
    for (file, content) in files {
        println!("[{}]", file.path().display().red());
        println!("File content: \n{sep}\n{}\n{sep}\n", content.dimmed());

        let start = std::time::Instant::now();

        // parse
        let tree = parser
            .parse(content.as_bytes(), None)
            .context("failed to parse file")?;
        let parse_duration = start.elapsed();

        if tree.root_node().has_error() {
            tracing::error!("Parse error");
            continue;
        }

        println!(
            "File SExp: \n{sep}\n{}\n{sep}\n",
            tree.root_node().to_sexp().dimmed()
        );

        // dump tree
        // let mut cursor = tree.walk();
        // dump_tree(&mut cursor, content.as_ref()).context("failed to dump tree")?;

        let results = FunctionDefinitionQuery::query(tree.root_node(), content.as_bytes());
        for result in results {
            tracing::warn!(
                "name: {:?}, params: {:?}, ret: {:?}",
                result.name.text(content.as_bytes()),
                result.parameters.map(|node| node.text(content.as_bytes())),
                result.return_type.map(|node| node.text(content.as_bytes())),
            );
        }

        // process
        run_checks(tree.root_node(), content);
        // walk_tree(content.as_bytes(), &mut tree.walk()).context("failed to process tree")?;
        let process_duration = start.elapsed() - parse_duration;

        println!(
            "Parsing: {:?}; Processing: {:?}; Total: {:?}",
            parse_duration,
            process_duration,
            (parse_duration + process_duration).green()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

    const CODE_1: &str = r#"var a := foo(1, 1 + 1, 2)"#;
    const CODE_2: &str = r#"var a := foo()"#;
    const QUERY: &str = r#"
        (variable_statement
            value: (call (_) @fn
                (arguments (_)? @cap)))
    "#;

    #[test]
    fn multi_capture_some() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_gdscript::LANGUAGE.into())
            .unwrap();

        let tree = parser.parse(CODE_1.as_bytes(), None).unwrap();

        let query = Query::new(&tree_sitter_gdscript::LANGUAGE.into(), QUERY).unwrap();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), CODE_1.as_bytes());

        let mut results = Vec::new();
        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                results.push((match_.id(), capture.index, capture.node.byte_range()));
            }
        }

        assert_eq!(
            results,
            vec![
                (0, 0, 9..12),  // Match 0, Capture 0: "foo"
                (0, 1, 13..14), // Match 0, Capture 1: "1"
                (1, 0, 9..12),  // Match 1, Capture 0: "foo"
                (1, 1, 16..21), // Match 1, Capture 1: "1 + 1"
                (2, 0, 9..12),  // Match 2, Capture 0: "foo"
                (2, 1, 23..24)  // Match 2, Capture 1: "2"
            ]
        );
    }

    #[test]
    fn multi_capture_none() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_gdscript::LANGUAGE.into())
            .unwrap();

        let tree = parser.parse(CODE_2.as_bytes(), None).unwrap();

        let query = Query::new(&tree_sitter_gdscript::LANGUAGE.into(), QUERY).unwrap();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), CODE_2.as_bytes());

        let mut results = Vec::new();
        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                results.push((match_.id(), capture.index, capture.node.byte_range()));
            }
        }

        assert_eq!(
            results,
            vec![
                (0, 0, 9..12) // Match 0, Capture 0: "foo"
            ]
        );
    }
}
