//! - [ ] Function name
//! - [ ] Class name
//! - [ ] Sub-class name
//! - [ ] Signal name
//! - [ ] Class variable name
//! - [ ] Class load variable name
//! - [ ] Function variable name
//! - [ ] Function preload variable name
//! - [ ] Function argument name
//! - [ ] Loop variable name
//! - [ ] Enum name
//! - [ ] Constant name
//! - [ ] Load constant name

use std::sync::Arc;

use miette::Report;
use tree_sitter::Node;

pub fn check_naming_convention(root: Node, source: Arc<str>) -> Vec<Report> {
    vec![]
}
