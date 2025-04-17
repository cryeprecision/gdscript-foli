// A trait to convert from captured nodes to the right type
trait FromNodeCapture<'tree> {
    fn from_node_capture(node: Option<&tree_sitter::QueryCapture<'tree>>) -> Self;
}

impl<'tree> FromNodeCapture<'tree> for tree_sitter::Node<'tree> {
    fn from_node_capture(node: Option<&tree_sitter::QueryCapture<'tree>>) -> Self {
        node.expect("required node missing").node
    }
}

impl<'tree> FromNodeCapture<'tree> for Option<tree_sitter::Node<'tree>> {
    fn from_node_capture(node: Option<&tree_sitter::QueryCapture<'tree>>) -> Self {
        node.map(|n| n.node)
    }
}

// Helper function to convert the node based on the expected type
fn match_field_type<'tree, T>(node_opt: Option<&tree_sitter::QueryCapture<'tree>>) -> T
where
    T: FromNodeCapture<'tree>,
{
    T::from_node_capture(node_opt)
}

// A macro to define query structs with their field mappings
macro_rules! define_query_struct {
    ($name:ident, $query_str:expr, { $($field:ident : $capture:literal => $type:ty),* $(,)? }) => {
        pub struct $name<'tree> {
            $(pub $field: $type),*
        }

        impl<'tree> $name<'tree> {
            pub fn query(root: ::tree_sitter::Node<'tree>, source: &[u8]) -> ::std::vec::Vec<Self> {
                let query = ::tree_sitter::Query::new(
                    &::tree_sitter_gdscript::LANGUAGE.into(),
                    $query_str,
                ).expect("valid query");
                let capture_count = query.capture_names().len();

                // Get capture indices
                $(
                let $field = query.capture_index_for_name($capture)
                    .expect(&format!("valid capture index for {}", $capture)) as usize;
                )*

                let mut query_cursor = ::tree_sitter::QueryCursor::new();
                query_cursor.set_max_start_depth(Some(1));


                use ::tree_sitter::StreamingIterator;
                let query_matches = query_cursor.matches(&query, root, source);
                let (min_size, _) = query_matches.size_hint();

                let mut results = ::std::vec::Vec::with_capacity(min_size);

                query_matches.for_each(|match_| {
                    // Create a map of capture name to node
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
                        $(
                        $field: match_field_type::<$type>(captures[$field]),
                        )*
                    });
                });

                results
            }
        }
    };
}

define_query_struct!(
    TopLevelDefinitionQuery,
    r#"
    (variable_statement
      (annotations
        (annotation (identifier) @annotation))?
      name: (name) @name
      type: (type (identifier) @type)
      value: (_)? @value)
    "#,
    {
        annotation: "annotation" => Option<tree_sitter::Node<'tree>>,
        name: "name" => tree_sitter::Node<'tree>,
        type_: "type" => Option<tree_sitter::Node<'tree>>,
        value: "value" => Option<tree_sitter::Node<'tree>>,
    }
);
