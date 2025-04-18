use std::{fmt::Write, str::Utf8Error};

use owo_colors::OwoColorize;
use tree_sitter::{Node, TreeCursor};

#[allow(dead_code)]
pub fn format_source(node: Node, source: &str) -> Result<String, Utf8Error> {
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

#[allow(dead_code)]
pub fn dump_tree(cursor: &mut TreeCursor, source: &str) -> Result<(), Utf8Error> {
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
