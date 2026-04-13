use crate::cfg::CFG;
use dart_dec_lifter::BlockId;
use petgraph::algo::dominators::simple_fast;
use petgraph::graph::NodeIndex;
use ahash::AHashMap;

/// Dominator tree for a CFG
pub struct DominatorTree {
    idom: AHashMap<BlockId, BlockId>,
    dominance_frontier: AHashMap<BlockId, Vec<BlockId>>,
}

impl DominatorTree {
    /// Compute the dominator tree for a CFG
    pub fn compute(cfg: &CFG) -> Self {
        let dominators = simple_fast(&cfg.graph, cfg.entry);

        let mut idom = AHashMap::new();
        for node in cfg.graph.node_indices() {
            let block_id = cfg.graph[node].id;
            if let Some(dom_node) = dominators.immediate_dominator(node) {
                idom.insert(block_id, cfg.graph[dom_node].id);
            }
        }

        let dominance_frontier = compute_dominance_frontier(cfg, &idom);

        Self {
            idom,
            dominance_frontier,
        }
    }

    /// Get immediate dominator of a block
    pub fn idom(&self, block: BlockId) -> Option<BlockId> {
        self.idom.get(&block).copied()
    }

    /// Check if block A dominates block B
    pub fn dominates(&self, a: BlockId, b: BlockId) -> bool {
        if a == b {
            return true;
        }
        let mut current = b;
        while let Some(dom) = self.idom.get(&current) {
            if *dom == a {
                return true;
            }
            if *dom == current {
                break; // reached root
            }
            current = *dom;
        }
        false
    }

    /// Get the dominance frontier of a block
    pub fn frontier(&self, block: BlockId) -> &[BlockId] {
        self.dominance_frontier
            .get(&block)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

fn compute_dominance_frontier(
    cfg: &CFG,
    idom: &AHashMap<BlockId, BlockId>,
) -> AHashMap<BlockId, Vec<BlockId>> {
    let mut df: AHashMap<BlockId, Vec<BlockId>> = AHashMap::new();

    for node in cfg.graph.node_indices() {
        let block = &cfg.graph[node];
        if block.predecessors.len() >= 2 {
            for &pred in &block.predecessors {
                let mut runner = pred;
                while runner != *idom.get(&block.id).unwrap_or(&block.id) {
                    df.entry(runner).or_default().push(block.id);
                    match idom.get(&runner) {
                        Some(&dom) if dom != runner => runner = dom,
                        _ => break,
                    }
                }
            }
        }
    }

    df
}
