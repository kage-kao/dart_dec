use dart_dec_lifter::{BlockId, IR};
use petgraph::graph::{Graph, NodeIndex};
use serde::Serialize;
use ahash::AHashMap;

/// Edge type in the CFG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EdgeKind {
    Unconditional,
    TrueEdge,
    FalseEdge,
    ExceptionEdge,
}

/// A basic block in the control flow graph
#[derive(Debug, Clone, Serialize)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<IR>,
    pub predecessors: Vec<BlockId>,
    pub successors: Vec<BlockId>,
    pub start_address: u64,
    pub end_address: u64,
}

/// Control Flow Graph
pub struct CFG {
    pub graph: Graph<BasicBlock, EdgeKind>,
    pub entry: NodeIndex,
    pub block_map: AHashMap<BlockId, NodeIndex>,
}

impl CFG {
    /// Build a CFG from a list of IR instructions
    pub fn build(ir_instructions: &[IR]) -> Self {
        let mut graph = Graph::new();
        let mut block_map = AHashMap::new();
        let mut current_block_instrs: Vec<IR> = Vec::new();
        let mut current_block_id: BlockId = 0;
        let mut blocks = Vec::new();

        // Step 1: Find block leaders
        let mut leaders = std::collections::BTreeSet::new();
        leaders.insert(0u32); // first instruction is always a leader

        for (i, ir) in ir_instructions.iter().enumerate() {
            match ir {
                IR::Branch { true_target, false_target, .. } => {
                    leaders.insert(*true_target);
                    leaders.insert(*false_target);
                    if i + 1 < ir_instructions.len() {
                        leaders.insert((i + 1) as u32);
                    }
                }
                IR::Jump(target) => {
                    leaders.insert(*target);
                    if i + 1 < ir_instructions.len() {
                        leaders.insert((i + 1) as u32);
                    }
                }
                IR::Return(_) | IR::Throw(_) => {
                    if i + 1 < ir_instructions.len() {
                        leaders.insert((i + 1) as u32);
                    }
                }
                _ => {}
            }
        }

        // Step 2: Split into basic blocks
        let leader_vec: Vec<u32> = leaders.into_iter().collect();
        for (li, &leader) in leader_vec.iter().enumerate() {
            let end = if li + 1 < leader_vec.len() {
                leader_vec[li + 1] as usize
            } else {
                ir_instructions.len()
            };

            let start = leader as usize;
            if start >= ir_instructions.len() {
                continue;
            }

            let instrs: Vec<IR> = ir_instructions[start..end.min(ir_instructions.len())].to_vec();

            let block = BasicBlock {
                id: leader,
                instructions: instrs,
                predecessors: vec![],
                successors: vec![],
                start_address: leader as u64,
                end_address: end as u64,
            };

            blocks.push(block);
        }

        // Step 3: Add nodes to graph
        for block in &blocks {
            let node = graph.add_node(block.clone());
            block_map.insert(block.id, node);
        }

        // Step 4: Add edges
        for block in &blocks {
            let from_node = block_map[&block.id];

            if let Some(last_ir) = block.instructions.last() {
                match last_ir {
                    IR::Branch { true_target, false_target, .. } => {
                        if let Some(&true_node) = block_map.get(true_target) {
                            graph.add_edge(from_node, true_node, EdgeKind::TrueEdge);
                        }
                        if let Some(&false_node) = block_map.get(false_target) {
                            graph.add_edge(from_node, false_node, EdgeKind::FalseEdge);
                        }
                    }
                    IR::Jump(target) => {
                        if let Some(&target_node) = block_map.get(target) {
                            graph.add_edge(from_node, target_node, EdgeKind::Unconditional);
                        }
                    }
                    IR::Return(_) | IR::Throw(_) => {
                        // No outgoing edges for return/throw
                    }
                    _ => {
                        // Fallthrough to next block
                        let next_id = block.end_address as BlockId;
                        if let Some(&next_node) = block_map.get(&next_id) {
                            graph.add_edge(from_node, next_node, EdgeKind::Unconditional);
                        }
                    }
                }
            }
        }

        // Update predecessor/successor lists
        let edges: Vec<_> = graph
            .edge_indices()
            .map(|e| {
                let (src, dst) = graph.edge_endpoints(e).unwrap();
                (graph[src].id, graph[dst].id)
            })
            .collect();

        for (src_id, dst_id) in edges {
            if let Some(&src_node) = block_map.get(&src_id) {
                graph[src_node].successors.push(dst_id);
            }
            if let Some(&dst_node) = block_map.get(&dst_id) {
                graph[dst_node].predecessors.push(src_id);
            }
        }

        let entry = block_map
            .get(&0)
            .copied()
            .unwrap_or_else(|| graph.add_node(BasicBlock {
                id: 0,
                instructions: vec![],
                predecessors: vec![],
                successors: vec![],
                start_address: 0,
                end_address: 0,
            }));

        CFG {
            graph,
            entry,
            block_map,
        }
    }

    /// Get all basic blocks in order
    pub fn blocks(&self) -> Vec<&BasicBlock> {
        let mut blocks: Vec<&BasicBlock> = self.graph.node_weights().collect();
        blocks.sort_by_key(|b| b.id);
        blocks
    }

    /// Get a specific block by ID
    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.block_map
            .get(&id)
            .map(|&node| &self.graph[node])
    }

    /// Export to DOT format for Graphviz
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph CFG {\n");
        dot.push_str("  node [shape=box, fontname=\"monospace\"];\n");

        for node in self.graph.node_indices() {
            let block = &self.graph[node];
            let label = format!(
                "BB{} ({} instrs)",
                block.id,
                block.instructions.len()
            );
            dot.push_str(&format!("  {} [label=\"{}\"];\n", block.id, label));
        }

        for edge in self.graph.edge_indices() {
            let (src, dst) = self.graph.edge_endpoints(edge).unwrap();
            let kind = self.graph[edge];
            let style = match kind {
                EdgeKind::TrueEdge => "color=green",
                EdgeKind::FalseEdge => "color=red",
                EdgeKind::ExceptionEdge => "color=orange, style=dashed",
                EdgeKind::Unconditional => "color=black",
            };
            dot.push_str(&format!(
                "  {} -> {} [{}];\n",
                self.graph[src].id, self.graph[dst].id, style
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Number of blocks
    pub fn num_blocks(&self) -> usize {
        self.graph.node_count()
    }
}
