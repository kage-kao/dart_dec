use crate::cfg::CFG;
use crate::dominators::DominatorTree;
use dart_dec_lifter::BlockId;
use serde::Serialize;

/// Information about a detected loop
#[derive(Debug, Clone, Serialize)]
pub struct LoopInfo {
    pub header: BlockId,
    pub body: Vec<BlockId>,
    pub back_edges: Vec<(BlockId, BlockId)>,
    pub kind: LoopKind,
    pub exit_blocks: Vec<BlockId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LoopKind {
    While,    // condition at beginning
    DoWhile,  // condition at end
    For,      // has inductive variable
    Infinite, // no exit condition detected
}

/// Detect all natural loops in a CFG
pub fn detect_loops(cfg: &CFG, dom_tree: &DominatorTree) -> Vec<LoopInfo> {
    let mut loops = Vec::new();

    // Find back edges: edge (A -> B) where B dominates A
    let mut back_edges: Vec<(BlockId, BlockId)> = Vec::new();

    for node in cfg.graph.node_indices() {
        let block = &cfg.graph[node];
        for &succ in &block.successors {
            if dom_tree.dominates(succ, block.id) {
                back_edges.push((block.id, succ));
            }
        }
    }

    // For each back edge, compute the natural loop
    for (tail, header) in &back_edges {
        let body = compute_loop_body(cfg, *header, *tail);
        let exit_blocks = find_exit_blocks(cfg, &body);

        let kind = determine_loop_kind(cfg, *header, *tail, &body);

        loops.push(LoopInfo {
            header: *header,
            body,
            back_edges: vec![(*tail, *header)],
            kind,
            exit_blocks,
        });
    }

    loops
}

fn compute_loop_body(cfg: &CFG, header: BlockId, tail: BlockId) -> Vec<BlockId> {
    let mut body = vec![header];
    let mut stack = vec![tail];

    while let Some(block) = stack.pop() {
        if !body.contains(&block) {
            body.push(block);
            if let Some(b) = cfg.block(block) {
                for &pred in &b.predecessors {
                    stack.push(pred);
                }
            }
        }
    }

    body.sort();
    body
}

fn find_exit_blocks(cfg: &CFG, body: &[BlockId]) -> Vec<BlockId> {
    let mut exits = Vec::new();
    for &block_id in body {
        if let Some(block) = cfg.block(block_id) {
            for &succ in &block.successors {
                if !body.contains(&succ) {
                    exits.push(block_id);
                    break;
                }
            }
        }
    }
    exits
}

fn determine_loop_kind(
    cfg: &CFG,
    header: BlockId,
    tail: BlockId,
    body: &[BlockId],
) -> LoopKind {
    // If header has a conditional branch with one target outside loop -> while
    if let Some(header_block) = cfg.block(header) {
        if header_block.successors.len() == 2 {
            let outside = header_block
                .successors
                .iter()
                .any(|s| !body.contains(s));
            if outside {
                return LoopKind::While;
            }
        }
    }

    // If tail has a conditional branch -> do-while
    if let Some(tail_block) = cfg.block(tail) {
        if tail_block.successors.len() == 2 {
            return LoopKind::DoWhile;
        }
    }

    // If no exit found -> infinite
    LoopKind::Infinite
}
