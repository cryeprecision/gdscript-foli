use std::sync::Arc;

use miette::{LabeledSpan, Report, Severity};
use tree_sitter::Node;

use crate::{NodeExt, query_struct::PrintCallQuery};

pub fn check_no_print_call(root: Node, source: Arc<str>) -> Vec<Report> {
    assert!(root.kind() == "source", "Expected 'source' node");

    let statements = PrintCallQuery::query(root, source.as_bytes());
    let mut reports = Vec::new();

    for statement in statements {
        reports.push(
            miette::miette!(
                severity = Severity::Warning,
                code = "no-print",
                labels = vec![LabeledSpan::new_with_span(
                    Some("What were you thinking?!".into()),
                    statement.print.to_source_span(),
                )],
                "calling print is discouraged, use a custom logger",
            )
            .with_source_code(Arc::clone(&source)),
        );
    }

    reports
}
