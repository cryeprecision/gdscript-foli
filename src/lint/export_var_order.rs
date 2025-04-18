//! TODO: Extend this to the full code order
//! https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_styleguide.html#code-order

use std::sync::Arc;

use miette::{LabeledSpan, Report, Severity};
use tree_sitter::Node;

use crate::{NodeExt, query_struct::TopLevelDefinitionQuery};

pub fn check_export_var_order(root: Node, source: Arc<str>) -> Vec<Report> {
    assert!(root.kind() == "source", "Expected 'source' node");

    let declarations = TopLevelDefinitionQuery::query(root, source.as_bytes())
        .into_iter()
        .map(|result| {
            let annotation = result
                .annotation
                .map(|annotation| annotation.text(source.as_bytes()));
            (result, annotation)
        });

    let mut reports = Vec::new();

    let mut past_export = false;
    let mut past_var = false;

    for (declaration, annotation) in declarations {
        match annotation {
            Some("export") => {
                if past_export {
                    reports.push(
                        miette::miette!(
                            severity = Severity::Warning,
                            code = "export-order",
                            labels = vec![LabeledSpan::new_with_span(
                                Some("out-of-order".into()),
                                declaration.statement.to_source_span(),
                            )],
                            "export variables should precede normal variables and onready variables",
                        )
                        .with_source_code(Arc::clone(&source))
                    );
                }
            }
            None => {
                past_export = true;
                if past_var {
                    reports.push(
                        miette::miette!(
                            severity = Severity::Warning,
                            code = "export-order",
                            labels = vec![LabeledSpan::new_with_span(
                                Some("out-of-order".into()),
                                declaration.statement.to_source_span(),
                            )],
                            "normal variables should be between export variables and onready variables",
                        )
                        .with_source_code(Arc::clone(&source))
                    );
                }
            }
            Some("onready") => {
                past_export = true;
                past_var = true;
            }
            _ => {
                tracing::warn!("Unknown annotation: {}", annotation.unwrap_or("unknown"));
            }
        }
    }

    reports
}
