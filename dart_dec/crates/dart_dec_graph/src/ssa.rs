use crate::cfg::CFG;
use crate::dominators::DominatorTree;
use dart_dec_lifter::{BlockId, IR, Reg};
use ahash::AHashMap;

/// Transform IR into SSA form by inserting phi functions and renaming variables
pub fn ssa_transform(cfg: &mut CFG, dom_tree: &DominatorTree) {
    // Step 1: Compute which blocks define which registers
    let defs = compute_definitions(cfg);

    // Step 2: Insert phi functions at dominance frontiers
    insert_phi_functions(cfg, dom_tree, &defs);

    // Step 3: Rename variables (each definition gets a unique version)
    rename_variables(cfg, dom_tree);
}

fn compute_definitions(cfg: &CFG) -> AHashMap<Reg, Vec<BlockId>> {
    let mut defs: AHashMap<Reg, Vec<BlockId>> = AHashMap::new();

    for block in cfg.blocks() {
        for ir in &block.instructions {
            if let Some(reg) = ir.dest_reg() {
                defs.entry(reg).or_default().push(block.id);
            }
        }
    }

    defs
}

fn insert_phi_functions(
    cfg: &mut CFG,
    dom_tree: &DominatorTree,
    defs: &AHashMap<Reg, Vec<BlockId>>,
) {
    for (reg, def_blocks) in defs {
        let mut worklist: Vec<BlockId> = def_blocks.clone();
        let mut has_phi: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
        let mut ever_on_worklist: std::collections::HashSet<BlockId> =
            def_blocks.iter().copied().collect();

        while let Some(block_id) = worklist.pop() {
            for &frontier_block in dom_tree.frontier(block_id) {
                if !has_phi.contains(&frontier_block) {
                    has_phi.insert(frontier_block);

                    // Insert phi function at the beginning of frontier_block
                    if let Some(&node) = cfg.block_map.get(&frontier_block) {
                        let block = &cfg.graph[node];
                        let sources: Vec<(BlockId, Reg)> = block
                            .predecessors
                            .iter()
                            .map(|&pred| (pred, *reg))
                            .collect();

                        let phi = IR::Phi {
                            dst: *reg,
                            sources,
                        };

                        let block_mut = &mut cfg.graph[node];
                        block_mut.instructions.insert(0, phi);
                    }

                    if !ever_on_worklist.contains(&frontier_block) {
                        ever_on_worklist.insert(frontier_block);
                        worklist.push(frontier_block);
                    }
                }
            }
        }
    }
}

fn rename_variables(cfg: &mut CFG, _dom_tree: &DominatorTree) {
    // Variable renaming in SSA: each register gets a version counter
    let mut counters: AHashMap<Reg, u16> = AHashMap::new();

    // Process blocks in dominator tree order
    let block_ids: Vec<BlockId> = cfg.blocks().iter().map(|b| b.id).collect();

    for block_id in block_ids {
        if let Some(&node) = cfg.block_map.get(&block_id) {
            let instrs = cfg.graph[node].instructions.clone();
            let mut new_instrs = Vec::with_capacity(instrs.len());

            for ir in instrs {
                // For each instruction that defines a register, create a new version
                if let Some(reg) = ir.dest_reg() {
                    let counter = counters.entry(reg).or_insert(0);
                    *counter += 1;
                    let new_reg = Reg(reg.0 * 100 + *counter);
                    // Remap the instruction with the new register
                    let new_ir = remap_dest_reg(ir, new_reg);
                    new_instrs.push(new_ir);
                } else {
                    new_instrs.push(ir);
                }
            }

            cfg.graph[node].instructions = new_instrs;
        }
    }
}

fn remap_dest_reg(ir: IR, new_reg: Reg) -> IR {
    match ir {
        IR::Assign(_, op) => IR::Assign(new_reg, op),
        IR::BinOp { op, lhs, rhs, .. } => IR::BinOp {
            dst: new_reg,
            op,
            lhs,
            rhs,
        },
        IR::UnaryOp { op, src, .. } => IR::UnaryOp {
            dst: new_reg,
            op,
            src,
        },
        IR::Call { kind, args, .. } => IR::Call {
            dst: Some(new_reg),
            kind,
            args,
        },
        IR::LoadPool { addr, resolved, .. } => IR::LoadPool {
            dst: new_reg,
            addr,
            resolved,
        },
        IR::LoadField {
            base,
            offset,
            field_name,
            ..
        } => IR::LoadField {
            dst: new_reg,
            base,
            offset,
            field_name,
        },
        IR::Phi { sources, .. } => IR::Phi {
            dst: new_reg,
            sources,
        },
        other => other,
    }
}
