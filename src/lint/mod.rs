pub type CheckFn = fn(root: tree_sitter::Node, source: std::sync::Arc<str>) -> Vec<miette::Report>;

mod export_var_order;
pub use export_var_order::check_export_var_order;

mod typed_function_signature;
pub use typed_function_signature::check_typed_function_signature;

mod no_print_call;
pub use no_print_call::check_no_print_call;

mod naming_convention;
pub use naming_convention::check_naming_convention;
