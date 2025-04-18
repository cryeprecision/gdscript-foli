use std::sync::Arc;

use miette::{LabeledSpan, Report, Severity};
use tree_sitter::Node;

use crate::{NodeExt, query_struct::ClassNameExtendsQuery};

pub fn check_class_name_extends(root: Node, source: Arc<str>) -> Vec<Report> {
    assert!(root.kind() == "source", "Expected 'source' node");

    let statements = ClassNameExtendsQuery::query(root, source.as_bytes());
    let mut reports = Vec::new();

    for statement in statements {
        reports.push(
            miette::miette!(
                severity = Severity::Warning,
                code = "class-name-extends",
                labels = vec![
                    LabeledSpan::new_with_span(
                        Some("swap this".into()),
                        statement.extends.to_source_span(),
                    ),
                    LabeledSpan::new_with_span(
                        Some("with this".into()),
                        statement.class_name.to_source_span(),
                    ),
                ],
                "class_name should precede extends",
            )
            .with_source_code(Arc::clone(&source)),
        );
    }

    reports
}
