use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SliceEntry {
    pub file: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlowEntry {
    pub var: String,
    pub from: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeSuggestion {
    pub line: usize,
    pub kind: &'static str,
    pub text: String,
}

pub mod analysis;
pub mod dataflow;
pub mod lang;
pub mod slice;
pub mod suggest;

pub use analysis::{CallGraphNode, CodeTypeInfo, ImpactResult};
pub use analysis::{build_call_graph, code_search, forward_slice, impact_analysis, type_info};
pub use dataflow::trace_variable;
pub use lang::{find_decl_line, find_function_start, list_functions};
pub use slice::{backward_slice, flatten_stmts};
pub use suggest::suggest;
