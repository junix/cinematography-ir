//! A layered, serializable intermediate representation for cinematography planning.

pub mod analyze;
pub mod diagnostic;
pub mod io;
pub mod model;
pub mod report;
pub mod validate;

pub use analyze::analyze_continuity;
pub use diagnostic::{Diagnostic, Severity, ValidationReport};
pub use io::{load_project, save_project_json};
pub use model::*;
pub use report::render_summary;
pub use validate::validate_project;
