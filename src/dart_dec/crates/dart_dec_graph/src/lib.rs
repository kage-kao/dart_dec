//! dart_dec_graph — CFG construction, SSA transformation, loop detection,
//! control flow structuring, type propagation, and AST generation.

mod ast;
mod cfg;
mod dominators;
mod loop_detect;
mod ssa;
mod structuring;
mod type_propagation;

pub use ast::*;
pub use cfg::{BasicBlock, CFG, EdgeKind};
pub use dominators::DominatorTree;
pub use loop_detect::LoopInfo;
pub use ssa::ssa_transform;
pub use structuring::structure_cfg;
pub use type_propagation::propagate_types;

#[cfg(test)]
mod tests;
