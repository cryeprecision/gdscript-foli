use std::fmt::Write;
use std::sync::Arc;

use anyhow::Context;
use tree_sitter::StreamingIterator;

mod query_struct;

fn format_source(node: tree_sitter::Node, source: &str) -> Result<String, std::str::Utf8Error> {
    use owo_colors::OwoColorize;

    let (line, rem_chars, rem_lines) = {
        let mut lines_iter = node
            .utf8_text(source.as_bytes())
            .expect("utf8 text")
            .lines();
        let mut line_iter = lines_iter.next().expect("at least one line").chars();

        let line = (&mut line_iter).take(50).collect::<String>();
        let rem_chars = line_iter.count();
        let rem_lines = lines_iter.count();
        (line, rem_chars, rem_lines)
    };

    let mut string = String::new();
    write!(&mut string, "{}", line.red()).unwrap();
    if rem_chars > 0 {
        write!(&mut string, "{}", "[...]".dimmed()).unwrap();
        write!(&mut string, "{}", format!(" (+{})", rem_chars).green()).unwrap();
    }
    if rem_lines > 0 {
        write!(&mut string, "{}", format!(" (+{})", rem_lines).yellow()).unwrap();
    }
    Ok(string)
}

pub trait NodeExt {
    fn to_source_span(&self) -> miette::SourceSpan;
    fn text<'a>(&self, source: &'a str) -> &'a str;
}

impl NodeExt for tree_sitter::Node<'_> {
    fn to_source_span(&self) -> miette::SourceSpan {
        miette::SourceSpan::new(
            self.start_byte().into(),
            self.end_byte() - self.start_byte(),
        )
    }
    fn text<'a>(&self, source: &'a str) -> &'a str {
        self.utf8_text(source.as_bytes()).expect("valid utf8")
    }
}

fn check_class_name_extends(root: tree_sitter::Node, source: Arc<str>) {
    use owo_colors::OwoColorize;
    assert!(root.kind() == "source", "Expected 'source' node");

    let query = tree_sitter::Query::new(
        &tree_sitter_gdscript::LANGUAGE.into(),
        r#"
            (_
              (extends_statement (type (identifier))) @extends
              (class_name_statement (name)) @class_name)
        "#,
    )
    .expect("valid query");

    let mut query_cursor = tree_sitter::QueryCursor::new();
    query_cursor.set_max_start_depth(Some(1));
    let mut matches = query_cursor.matches(&query, root, source.as_bytes());

    while let Some(match_) = matches.next() {
        let help = miette::miette!(
            severity = miette::Severity::Warning,
            help = "Ur facken retarded",
            labels = vec![
                miette::LabeledSpan::new_with_span(
                    Some("swap this".into()),
                    match_.captures[0].node.to_source_span(),
                ),
                miette::LabeledSpan::new_with_span(
                    Some("with this".into()),
                    match_.captures[1].node.to_source_span(),
                )
            ],
            "{} should precede {}",
            "class_name".red(),
            "extends".red(),
        )
        .with_source_code(Arc::clone(&source));
        println!("{:?}", help);
    }

    tracing::info!("{}", "Class name and extends statements are valid.".green());
}

struct TopLevelDefinitionQuery<'tree> {
    pub annotation: Option<tree_sitter::Node<'tree>>,
    pub name: tree_sitter::Node<'tree>,
    pub type_: Option<tree_sitter::Node<'tree>>,
    pub interred_type: Option<tree_sitter::Node<'tree>>,
    pub value: Option<tree_sitter::Node<'tree>>,
}

impl<'tree> TopLevelDefinitionQuery<'tree> {
    fn query(root: tree_sitter::Node<'tree>, source: &str) -> Vec<Self> {
        let query = tree_sitter::Query::new(
            &tree_sitter_gdscript::LANGUAGE.into(),
            r#"
                (variable_statement
                  (annotations
                    (annotation (identifier) @annotation))?
                  name: (name) @name
                  type: [
                    (type (identifier) @type)
                    (inferred_type) @inferred_type]
                  value: (_)? @value)
            "#,
        )
        .expect("valid query");

        let annotation_index = query
            .capture_index_for_name("annotation")
            .expect("valid capture index");
        let name_index = query
            .capture_index_for_name("name")
            .expect("valid capture index");
        let type_index = query
            .capture_index_for_name("type")
            .expect("valid capture index");
        let inferred_type_index = query
            .capture_index_for_name("inferred_type")
            .expect("valid capture index");
        let value_index = query
            .capture_index_for_name("value")
            .expect("valid capture index");

        let mut query_cursor = tree_sitter::QueryCursor::new();
        query_cursor.set_max_start_depth(Some(1));

        let query_matches = query_cursor.matches(&query, root, source.as_bytes());

        let (min_size, _) = query_matches.size_hint();
        let mut matches = Vec::with_capacity(min_size);

        query_matches.for_each(|match_| {
            let mut captures = [Option::<&tree_sitter::QueryCapture>::None; 5];
            for capture in match_.captures {
                if capture.index == annotation_index {
                    captures[0] = Some(capture);
                } else if capture.index == name_index {
                    captures[1] = Some(capture);
                } else if capture.index == type_index {
                    captures[2] = Some(capture);
                } else if capture.index == inferred_type_index {
                    captures[3] = Some(capture);
                } else if capture.index == value_index {
                    captures[4] = Some(capture);
                }
            }
            matches.push(TopLevelDefinitionQuery {
                annotation: captures[0].map(|c| c.node),
                name: captures[1].expect("name capture").node,
                type_: captures[2].map(|c| c.node),
                interred_type: captures[3].map(|c| c.node),
                value: captures[4].map(|c| c.node),
            });
        });
        matches
    }
}

fn check_export_var_order(root: tree_sitter::Node, source: Arc<str>) {
    assert!(root.kind() == "source", "Expected 'source' node");

    for match_ in TopLevelDefinitionQuery::query(root, source.as_ref()) {
        let mut statement = String::new();

        if let Some(annotation) = match_.annotation {
            statement.push('@');
            statement.push_str(annotation.text(source.as_ref()));
            statement.push(' ');
        }
        statement.push_str("var ");
        statement.push_str(match_.name.text(source.as_ref()));
        if let Some(type_) = match_.type_ {
            statement.push_str(": ");
            statement.push_str(type_.text(source.as_ref()));
        }
        if let Some(value) = match_.value {
            if let Some(_) = match_.interred_type {
                statement.push_str(" := ");
            } else {
                statement.push_str(" = ");
            }
            statement.push_str(value.text(source.as_ref()));
        }
        tracing::info!("Found variable statement: {:?}", statement);
    }
}

fn dump_tree(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
) -> Result<(), std::str::Utf8Error> {
    use owo_colors::OwoColorize;

    let indent = "  ".repeat(cursor.depth() as usize);
    println!(
        "[walk] {}{} {} (d: {}, i: {})",
        indent,
        cursor.node().kind().blue(),
        format_source(cursor.node(), source)?,
        cursor.depth(),
        cursor.descendant_index(),
    );

    if cursor.goto_first_child() {
        dump_tree(cursor, source)?;
    }

    while cursor.goto_next_sibling() {
        dump_tree(cursor, source)?;
    }

    cursor.goto_parent();
    Ok(())
}

fn run_checks(root: tree_sitter::Node, source: Arc<str>) {
    use owo_colors::OwoColorize;
    assert!(root.kind() == "source", "Expected 'source' node");

    check_class_name_extends(root, Arc::clone(&source));
    check_export_var_order(root, Arc::clone(&source));

    tracing::info!("{}", "Checks passed.".green());
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
                && entry.path().extension().map_or(false, |ext| ext == "gd")
                && !entry
                    .path()
                    .to_str()
                    .map_or(false, |f| f.contains("/addons/"))
        })
        .filter(|entry| {
            entry
                .path()
                .to_str()
                .map_or(false, |name| name.ends_with("ranged_weapon.gd"))
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
        let mut cursor = tree.walk();
        dump_tree(&mut cursor, content.as_ref()).context("failed to dump tree")?;

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
