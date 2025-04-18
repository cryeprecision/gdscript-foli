use std::{collections::HashMap, sync::Arc};

use miette::{LabeledSpan, Report, Severity};
use tree_sitter::Node;

use crate::{NodeExt, query_struct::FunctionDefinitionQuery};

struct Definition<'tree> {
    ret: Option<Node<'tree>>,
    params: Vec<Node<'tree>>,
    param_list: Node<'tree>,
}

pub fn check_typed_function_signature(root: Node, source: Arc<str>) -> Vec<Report> {
    assert!(root.kind() == "source", "Expected 'source' node");

    // TODO: Explain this, bla bla group matches by function name
    let definitions: Vec<Definition> = {
        let results = FunctionDefinitionQuery::query(root, source.as_bytes());
        let mut definitions: HashMap<usize, Definition> = HashMap::new();
        for result in results {
            definitions
                .entry(result.name.id())
                .and_modify(|entry| {
                    if let Some(param) = result.parameters {
                        entry.params.push(param);
                    }
                })
                .or_insert_with(|| Definition {
                    ret: result.return_type,
                    params: match result.parameters {
                        Some(param) => vec![param],
                        None => vec![],
                    },
                    param_list: result.parameters_list,
                });
        }
        definitions.into_values().collect()
    };

    let mut reports = Vec::new();

    for definition in definitions {
        let mut labels = Vec::new();

        if definition.ret.is_none() {
            labels.push(LabeledSpan::new_with_span(
                Some("function is missing a return type".into()),
                definition.param_list.to_source_point_end(),
            ));
        }

        for param in definition.params {
            let is_typed = param.child_by_field_name("type").is_some();
            if !is_typed {
                labels.push(LabeledSpan::new_with_span(
                    Some("parameter is missing a type annotation".into()),
                    param.to_source_span(),
                ));
            }
        }

        if !labels.is_empty() {
            reports.push(
                miette::miette!(
                    severity = Severity::Warning,
                    code = "typed-function-signature",
                    labels = labels,
                    "function signatures should be fully typed",
                )
                .with_source_code(Arc::clone(&source)),
            );
        }
    }

    reports
}
