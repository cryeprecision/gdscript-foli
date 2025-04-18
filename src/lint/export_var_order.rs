//! TODO: Extend this to the full code order
//! https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_styleguide.html#code-order
//!
//! See [`Order`] for what's implemented already.

use std::sync::Arc;

use miette::{LabeledSpan, Report, Severity};
use owo_colors::OwoColorize;
use tree_sitter::Node;

use crate::{NodeExt, query_struct::TopLevelDefinitionQuery};

/// See also [`Order::to_numeric`].
#[derive(Debug, Clone, Copy)]
enum Order {
    /// `class_name Foo`
    ClassNameStatement,
    /// `extends Bar`
    ExtendsStatement,
    /// `## He who reads this is a poophead`
    DocComment,
    /// `# He who reads this is cool`
    Comment,
    /// `signal sharted(amount: float)`
    SignalStatement,
    /// `enum Foo { BAR, BAZ }`
    EnumDefinition,
    /// `const FOO := 2.0`
    ConstStatement,
    /// `@export var foo := 2.0`
    ExportVariableStatement,
    /// `var foo := 2.0`
    VariableStatement,
    /// `@onready var foo = get_node("Foo")`
    OnReadyVariableStatement,
    /// `static func adopt_cat() -> void`
    StaticFunctionDefinition,
    /// `func adopt_dog() -> void`
    FunctionDefinition,
}

impl Order {
    /// Returns an integer representing the relative order between top-level stuff.
    ///
    /// # Examples
    ///
    /// - `0200`: `class_name`
    /// - `1300`: remaining static methods
    /// - `1302`: `_enter_tree`
    ///
    /// See [code order](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_styleguide.html#code-order)
    fn to_numeric(self) -> Option<usize> {
        match self {
            Order::ClassNameStatement => Some(200),
            Order::ExtendsStatement => Some(300),
            Order::DocComment => Some(400),
            Order::Comment => None,
            Order::SignalStatement => Some(500),
            Order::EnumDefinition => Some(600),
            Order::ConstStatement => Some(700),
            Order::ExportVariableStatement => Some(900),
            Order::VariableStatement => Some(1000),
            Order::OnReadyVariableStatement => Some(1100),
            Order::StaticFunctionDefinition => Some(1300),
            Order::FunctionDefinition => Some(1600),
        }
    }
}

fn definition_order(definition: Node, source: &[u8]) -> Option<Order> {
    match definition.kind() {
        "class_name_statement" => Some(Order::ClassNameStatement),
        "extends_statement" => Some(Order::ExtendsStatement),
        "comment" => match definition.text(source).starts_with("##") {
            true => Some(Order::DocComment),
            false => Some(Order::Comment),
        },
        "signal_statement" => Some(Order::SignalStatement),
        "enum_definition" => Some(Order::EnumDefinition),
        "const_statement" => Some(Order::ConstStatement),
        "variable_statement" => match definition
            .child_by_field_name("annotation")
            .map(|annotation| annotation.text(source))
        {
            None => Some(Order::VariableStatement),
            Some("export") => Some(Order::ExportVariableStatement),
            Some("onready") => Some(Order::OnReadyVariableStatement),
            _ => None,
        },
        // TODO: There's a lot left to handle for methods
        "function_definition" => match definition.child(0).expect("at least one child").kind() {
            "static_keyword" => Some(Order::StaticFunctionDefinition),
            _ => Some(Order::FunctionDefinition),
        },
        _ => None,
    }
}

pub fn check_export_var_order(root: Node, source: Arc<str>) -> Vec<Report> {
    assert!(root.kind() == "source", "Expected 'source' node");

    let mut reports = Vec::new();
    let declarations = TopLevelDefinitionQuery::query(root, source.as_bytes())
        .into_iter()
        .filter_map(|result| {
            // If we can't recognize the thing, warn the user.
            let Some(order) = definition_order(result.definition, source.as_bytes()) else {
                reports.push(
                    miette::miette!(
                        severity = Severity::Warning,
                        code = "top-level-order-unknown",
                        labels = vec![LabeledSpan::new_with_span(
                            Some(result.definition.kind().to_string()),
                            result.definition.to_source_point_start(),
                        )],
                        url = "https://github.com/cryeprecision/gdscript-foli/issues",
                        help = "gotta complain to the idiot developer about this one",
                        "statement has no associated order"
                    )
                    .with_source_code(Arc::clone(&source)),
                );
                return None;
            };
            // If we recognized the thing but didn't associate an order, ignore it.
            order.to_numeric().map(|num| (result, num))
        })
        .collect::<Vec<_>>();

    for i in 0..declarations.len() {
        let (declaration_i, order_i) = &declarations[i];

        // Scan from the current element upwards and check if we should precede one of them
        let mut out_of_order = None;
        for j in (0..i).rev() {
            let (declaration_j, order_j) = &declarations[j];
            if order_j > order_i {
                out_of_order = Some((declaration_j, order_j));
                break;
            }
        }

        if let Some((declaration_j, order_j)) = out_of_order {
            // TODO: I feel like this error message is kind of hard to read
            reports.push(
                miette::miette!(
                    severity = Severity::Warning,
                    code = "top-level-order",
                    labels = vec![
                        LabeledSpan::new_with_span(
                            Some(format!(
                                "{} ({}) should come {}",
                                declaration_j.definition.kind(),
                                order_j,
                                "after".underline()
                            )),
                            declaration_j.definition.to_source_point_start(),
                        ),
                        LabeledSpan::new_primary_with_span(
                            Some(format!("{} ({})", declaration_i.definition.kind(), order_i)),
                            declaration_i.definition.to_source_point_start(),
                        )
                    ],
                    url = "https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/\
                                gdscript_styleguide.html#code-order",
                    help = "move the bottom one above the top one to fix this",
                    "invalid declaration order (see link)",
                )
                .with_source_code(Arc::clone(&source)),
            );
        }
    }

    reports
}
