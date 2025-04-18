pub type CheckFn = fn(root: tree_sitter::Node, source: std::sync::Arc<str>) -> Vec<miette::Report>;

mod class_name_extends;
pub use class_name_extends::check_class_name_extends;

mod export_var_order;
pub use export_var_order::check_export_var_order;

mod typed_function_signature;
pub use typed_function_signature::check_typed_function_signature;

mod no_print_call;
pub use no_print_call::check_no_print_call;
