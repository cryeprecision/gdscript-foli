use miette::{LabeledSpan, Severity};
use tree_sitter::{Node, QueryCapture, QueryError};

// A trait to convert from captured nodes to the right type
trait FromNodeCapture<'tree> {
    fn from_node_capture(node: Option<&QueryCapture<'tree>>, name: &'static str) -> Self;
}

impl<'tree> FromNodeCapture<'tree> for Node<'tree> {
    fn from_node_capture(node: Option<&QueryCapture<'tree>>, name: &'static str) -> Self {
        node.map(|n| n.node)
            .unwrap_or_else(|| panic!("required node {name:?} missing"))
    }
}

impl<'tree> FromNodeCapture<'tree> for Option<Node<'tree>> {
    fn from_node_capture(node: Option<&QueryCapture<'tree>>, _name: &'static str) -> Self {
        node.map(|n| n.node)
    }
}

// Helper function to convert the node based on the expected type
fn match_field_type<'tree, T>(node_opt: Option<&QueryCapture<'tree>>, name: &'static str) -> T
where
    T: FromNodeCapture<'tree>,
{
    T::from_node_capture(node_opt, name)
}

fn format_query_error(err: QueryError, source: &[u8]) -> miette::Report {
    miette::miette!(
        severity = Severity::Error,
        code = "query-error",
        labels = vec![LabeledSpan::new(
            Some(format!("{:?}", err.kind)),
            err.offset,
            0
        )],
        "Failed to construct tree-sitter query"
    )
    .with_source_code(source.to_owned())
}

// A macro to define query structs with their field mappings
macro_rules! define_query_struct {
    (
        $name:ident,
        $query_str:expr,
        {
            $($field:ident : $capture:literal => $type:ty),*
            $(,)?
        }
    ) => {
        define_query_struct!(
            $name,
            $query_str,
            {
                $($field : $capture => $type),*
            },
            max_start_depth = Some(0)
        );
    };
    (
        $name:ident,
        $query_str:expr,
        {
            $($field:ident : $capture:literal => $type:ty),*
            $(,)?
        },
        max_start_depth = $max_depth:expr
    ) => {
        #[allow(dead_code)]
        #[derive(Debug)]
        pub struct $name<'tree> {
            pub match_id: usize,
            $(pub $field: $type),*
        }

        impl<'tree> $name<'tree> {
            pub fn query(root: ::tree_sitter::Node<'tree>, source: &[u8]) -> ::std::vec::Vec<Self> {
                let query_str = $query_str;
                let query = ::tree_sitter::Query::new(
                    &::tree_sitter_gdscript::LANGUAGE.into(),
                    query_str,
                ).map_err(|err| format_query_error(err, query_str.as_bytes())).expect("valid query");
                let capture_count = query.capture_names().len();

                $(
                let $field = query.capture_index_for_name($capture)
                    .expect(&format!("valid capture index for {}", $capture)) as usize;
                )*

                let mut query_cursor = ::tree_sitter::QueryCursor::new();
                query_cursor.set_max_start_depth($max_depth);

                use ::tree_sitter::StreamingIterator;
                let mut query_matches = query_cursor.matches(&query, root, source);

                let mut results = ::std::vec::Vec::new();
                while let Some(match_) = query_matches.next() {
                    let mut captures = vec![::std::option::Option::None; capture_count];
                    for capture in match_.captures {
                        $(
                        if capture.index as usize == $field {
                            captures[$field] = Some(capture);
                            continue;
                        }
                        )*
                        panic!("unexpected capture index: {}", capture.index);
                    }
                    results.push(Self {
                        match_id: match_.id() as usize,
                        $(
                        $field: match_field_type::<$type>(
                            captures[$field],
                            stringify!($field),
                        ),
                        )*
                    });
                }
                results
            }
        }
    };
}

define_query_struct!(
    TopLevelDefinitionQuery,
    r#"
        (_ (_) @definition)
    "#,
    {
        definition: "definition" => Node<'tree>,
    }
);

define_query_struct!(
    FunctionDefinitionQuery,
    r#"
        (_ (
            function_definition
                name: (name) @name
                parameters: (parameters (_)? @parameters) @parameters_list
                return_type: (_)? @return_type))
    "#,
    {
        name: "name" => Node<'tree>,
        parameters: "parameters" => Option<Node<'tree>>,
        return_type: "return_type" => Option<Node<'tree>>,
        parameters_list: "parameters_list" => Node<'tree>,
    },
    // FIXME: I think this should be Some(0), but that doesn't work
    max_start_depth = Some(1)
);

define_query_struct!(
    PrintCallQuery,
    r#"
        (call
            (identifier) @print (#eq? @print "print")
            (arguments (_)))
    "#,
    {
        print: "print" => Node<'tree>,
    },
    max_start_depth = None
);
