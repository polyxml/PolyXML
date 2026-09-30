use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap, HashMap, HashSet};

use super::{QName, SchemaIR, TypeDef, TypeRef};

/// A bounded compilation unit chunk containing a subset of types from the IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeChunk {
    /// 0-based chunk index.
    pub index: usize,
    /// Chunk identifier (e.g. "chunk_00").
    pub name: String,
    /// Fully qualified names of all types assigned to this chunk in topological order.
    pub types: Vec<QName>,
}

/// A complete partitioning plan of an IR's types into topologically sorted chunks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPlan {
    pub chunks: Vec<TypeChunk>,
    /// Map from each type QName to its assigned chunk index.
    pub type_to_chunk: HashMap<QName, usize>,
}

impl ChunkPlan {
    /// Return the list of chunk indices that the specified chunk depends on.
    /// By invariant of topological SCC condensation, every dependency index `q`
    /// satisfies `q < chunk_index`.
    pub fn dependencies_of(&self, chunk_index: usize, ir: &SchemaIR) -> BTreeSet<usize> {
        let mut deps = BTreeSet::new();
        let Some(chunk) = self.chunks.get(chunk_index) else {
            return deps;
        };

        for qname in &chunk.types {
            let type_deps = extract_type_dependencies(qname, ir);
            for dep in type_deps {
                if let Some(&target_chunk) = self.type_to_chunk.get(&dep) {
                    if target_chunk != chunk_index {
                        deps.insert(target_chunk);
                    }
                }
            }
        }

        deps
    }
}

/// Helper wrapper for deterministic min-heap priority queue ordering.
#[derive(Eq, PartialEq)]
struct ReadyComponent {
    min_qname: QName,
    component_idx: usize,
}

impl Ord for ReadyComponent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other
            .min_qname
            .cmp(&self.min_qname)
            .then_with(|| other.component_idx.cmp(&self.component_idx))
    }
}

impl PartialOrd for ReadyComponent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Extract all direct named type dependencies for a given type in the IR.
pub fn extract_type_dependencies(qname: &QName, ir: &SchemaIR) -> BTreeSet<QName> {
    let mut deps = BTreeSet::new();
    let Some(type_def) = ir.types.get(qname) else {
        return deps;
    };

    match type_def {
        TypeDef::Struct(s) => {
            if let Some(ref base) = s.base_type {
                if ir.types.contains_key(base) && !ir.is_external_type(base) {
                    deps.insert(base.clone());
                }
            }
            for field in &s.fields {
                collect_type_refs(&field.type_ref, ir, &mut deps);
            }
        }
        TypeDef::Union(u) => {
            for branch in &u.branches {
                collect_type_refs(&branch.type_ref, ir, &mut deps);
            }
        }
        TypeDef::Enum(e) => {
            collect_type_refs(&e.base_type, ir, &mut deps);
        }
        TypeDef::Simple(st) => {
            collect_type_refs(&st.base_type, ir, &mut deps);
        }
    }

    // Do not include self-dependency in external dependency list
    deps.remove(qname);
    deps
}

fn collect_type_refs(type_ref: &TypeRef, ir: &SchemaIR, deps: &mut BTreeSet<QName>) {
    match type_ref {
        TypeRef::Named(target) => {
            if ir.types.contains_key(target) && !ir.is_external_type(target) {
                deps.insert(target.clone());
            }
        }
        TypeRef::Boxed(inner) | TypeRef::List(inner) => {
            collect_type_refs(inner, ir, deps);
        }
        TypeRef::Primitive(_) => {}
    }
}

/// Partition the types of a SchemaIR into topological SCC chunks.
///
/// Each chunk contains at most `budget` types (unless a single cyclic SCC
/// contains more types than `budget`, in which case it stays co-located in one chunk).
///
/// Guaranteed Properties:
/// 1. Cycle Condensation: All mutually recursive types belong to the same chunk.
/// 2. Topological Order: If type `A` in chunk `p` depends on type `B` in chunk `q`,
///    then `q <= p`.
/// 3. Zero Circular Imports: Cross-chunk circular dependencies are mathematically impossible.
pub fn partition_topological_chunks(ir: &SchemaIR, budget: usize) -> ChunkPlan {
    // Collect all local emitted types (excluding external types)
    let emitted_types: Vec<QName> = ir
        .types
        .keys()
        .filter(|q| !ir.is_external_type(q))
        .cloned()
        .collect();

    if emitted_types.is_empty() {
        return ChunkPlan {
            chunks: Vec::new(),
            type_to_chunk: HashMap::new(),
        };
    }

    // If budget is 0 or all types fit in one chunk, return a single chunk
    if budget == 0 || emitted_types.len() <= budget {
        // Still sort topologically within the single chunk for cleanliness
        let sorted = topological_sort_types(&emitted_types, ir);
        let mut type_to_chunk = HashMap::new();
        for q in &sorted {
            type_to_chunk.insert(q.clone(), 0);
        }
        return ChunkPlan {
            chunks: vec![TypeChunk {
                index: 0,
                name: "chunk_00".to_string(),
                types: sorted,
            }],
            type_to_chunk,
        };
    }

    // 1. Build adjacency list of dependencies: node -> set of nodes it depends on
    let mut adj: HashMap<QName, Vec<QName>> = HashMap::new();
    for qname in &emitted_types {
        let deps = extract_type_dependencies(qname, ir);
        adj.insert(qname.clone(), deps.into_iter().collect());
    }

    // 2. Run Tarjan's SCC algorithm on the emitted types
    let sccs = run_tarjan_scc(&emitted_types, &adj);

    // 3. Build Condensation DAG
    // Map each node to its SCC index
    let mut node_to_scc: HashMap<QName, usize> = HashMap::new();
    for (scc_idx, scc) in sccs.iter().enumerate() {
        for qname in scc {
            node_to_scc.insert(qname.clone(), scc_idx);
        }
    }

    let num_sccs = sccs.len();
    // prereqs[i]: SCCs that i depends on (must come before i)
    let mut prereqs: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); num_sccs];
    // dependents[j]: SCCs that depend on j (come after j)
    let mut dependents: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); num_sccs];

    for (scc_idx, scc) in sccs.iter().enumerate() {
        for qname in scc {
            if let Some(deps) = adj.get(qname) {
                for dep in deps {
                    if let Some(&dep_scc) = node_to_scc.get(dep) {
                        if dep_scc != scc_idx {
                            prereqs[scc_idx].insert(dep_scc);
                            dependents[dep_scc].insert(scc_idx);
                        }
                    }
                }
            }
        }
    }

    // 4. Topological Sort of Condensation DAG using Kahn's algorithm
    // In-degree is the number of unsatisfied prerequisites (SCCs that must come before)
    let mut in_degree: Vec<usize> = prereqs.iter().map(|p| p.len()).collect();
    let mut ready_heap = BinaryHeap::new();

    for (idx, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            let min_q = sccs[idx].iter().min().cloned().unwrap();
            ready_heap.push(ReadyComponent {
                min_qname: min_q,
                component_idx: idx,
            });
        }
    }

    let mut sorted_sccs: Vec<usize> = Vec::with_capacity(num_sccs);
    while let Some(ReadyComponent { component_idx, .. }) = ready_heap.pop() {
        sorted_sccs.push(component_idx);
        for &dep_idx in &dependents[component_idx] {
            in_degree[dep_idx] -= 1;
            if in_degree[dep_idx] == 0 {
                let min_q = sccs[dep_idx].iter().min().cloned().unwrap();
                ready_heap.push(ReadyComponent {
                    min_qname: min_q,
                    component_idx: dep_idx,
                });
            }
        }
    }

    // Safeguard for completeness
    if sorted_sccs.len() < num_sccs {
        for idx in 0..num_sccs {
            if !sorted_sccs.contains(&idx) {
                sorted_sccs.push(idx);
            }
        }
    }

    // 5. Bounded chunk packing
    let mut chunks: Vec<TypeChunk> = Vec::new();
    let mut current_chunk_types: Vec<QName> = Vec::new();

    for &scc_idx in &sorted_sccs {
        let scc_types = &sccs[scc_idx];
        if !current_chunk_types.is_empty() && current_chunk_types.len() + scc_types.len() > budget {
            let idx = chunks.len();
            chunks.push(TypeChunk {
                index: idx,
                name: format!("chunk_{:02}", idx),
                types: std::mem::take(&mut current_chunk_types),
            });
        }
        current_chunk_types.extend(scc_types.iter().cloned());
    }

    if !current_chunk_types.is_empty() {
        let idx = chunks.len();
        chunks.push(TypeChunk {
            index: idx,
            name: format!("chunk_{:02}", idx),
            types: current_chunk_types,
        });
    }

    let mut type_to_chunk = HashMap::new();
    for chunk in &chunks {
        for q in &chunk.types {
            type_to_chunk.insert(q.clone(), chunk.index);
        }
    }

    ChunkPlan {
        chunks,
        type_to_chunk,
    }
}

/// Helper to topologically sort a list of types.
fn topological_sort_types(types: &[QName], ir: &SchemaIR) -> Vec<QName> {
    let mut adj: HashMap<QName, Vec<QName>> = HashMap::new();
    for q in types {
        let deps = extract_type_dependencies(q, ir);
        adj.insert(q.clone(), deps.into_iter().collect());
    }
    let sccs = run_tarjan_scc(types, &adj);
    let mut flat = Vec::new();
    for scc in sccs {
        flat.extend(scc);
    }
    flat
}

/// Tarjan's SCC algorithm implementation returning SCCs with sorted elements.
fn run_tarjan_scc(nodes: &[QName], adj: &HashMap<QName, Vec<QName>>) -> Vec<Vec<QName>> {
    struct Tarjan<'a> {
        adj: &'a HashMap<QName, Vec<QName>>,
        index: usize,
        indices: HashMap<QName, usize>,
        lowlinks: HashMap<QName, usize>,
        on_stack: HashSet<QName>,
        stack: Vec<QName>,
        sccs: Vec<Vec<QName>>,
    }

    impl<'a> Tarjan<'a> {
        fn strongconnect(&mut self, node: &QName) {
            self.indices.insert(node.clone(), self.index);
            self.lowlinks.insert(node.clone(), self.index);
            self.index += 1;
            self.stack.push(node.clone());
            self.on_stack.insert(node.clone());

            if let Some(neighbors) = self.adj.get(node) {
                for neighbor in neighbors {
                    if !self.indices.contains_key(neighbor) {
                        self.strongconnect(neighbor);
                        let n_lowlink = *self.lowlinks.get(neighbor).unwrap();
                        let curr_lowlink = self.lowlinks.get_mut(node).unwrap();
                        *curr_lowlink = std::cmp::min(*curr_lowlink, n_lowlink);
                    } else if self.on_stack.contains(neighbor) {
                        let n_index = *self.indices.get(neighbor).unwrap();
                        let curr_lowlink = self.lowlinks.get_mut(node).unwrap();
                        *curr_lowlink = std::cmp::min(*curr_lowlink, n_index);
                    }
                }
            }

            if self.lowlinks.get(node) == self.indices.get(node) {
                let mut scc = Vec::new();
                while let Some(w) = self.stack.pop() {
                    self.on_stack.remove(&w);
                    scc.push(w.clone());
                    if &w == node {
                        break;
                    }
                }
                // Sort deterministically within each SCC
                scc.sort();
                self.sccs.push(scc);
            }
        }
    }

    let mut tarjan = Tarjan {
        adj,
        index: 0,
        indices: HashMap::new(),
        lowlinks: HashMap::new(),
        on_stack: HashSet::new(),
        stack: Vec::new(),
        sccs: Vec::new(),
    };

    // Deterministic start order
    let mut sorted_nodes = nodes.to_vec();
    sorted_nodes.sort();
    for node in &sorted_nodes {
        if !tarjan.indices.contains_key(node) {
            tarjan.strongconnect(node);
        }
    }

    tarjan.sccs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Cardinality, FieldDef, FieldKind, StructDef, TypeDef};

    fn make_struct(name: &str, fields: &[(&str, &str)]) -> TypeDef {
        let qname = QName::local(name);
        let field_defs = fields
            .iter()
            .map(|(fname, target)| {
                let mut f = FieldDef::new(
                    *fname,
                    *fname,
                    FieldKind::Element,
                    TypeRef::Named(QName::local(*target)),
                );
                f.cardinality = Cardinality::required_one();
                f
            })
            .collect();
        TypeDef::Struct(StructDef {
            qname,
            base_type: None,
            is_abstract: false,
            is_mixed: false,
            fields: field_defs,
            documentation: None,
        })
    }

    #[test]
    fn test_topological_scc_chunking_dag_invariants() {
        let mut ir = SchemaIR::new();
        // A -> B -> C (linear dependency: A depends on B, B depends on C)
        ir.types.insert(QName::local("C"), make_struct("C", &[]));
        ir.types
            .insert(QName::local("B"), make_struct("B", &[("c", "C")]));
        ir.types
            .insert(QName::local("A"), make_struct("A", &[("b", "B")]));

        // D and E form a cycle (mutual recursion)
        ir.types
            .insert(QName::local("D"), make_struct("D", &[("e", "E")]));
        ir.types
            .insert(QName::local("E"), make_struct("E", &[("d", "D")]));

        // F depends on D
        ir.types
            .insert(QName::local("F"), make_struct("F", &[("d", "D")]));

        // Partition with budget = 2
        let plan = partition_topological_chunks(&ir, 2);

        // Verify:
        // 1. D and E MUST be in the exact same chunk because they form an SCC!
        let chunk_d = plan.type_to_chunk[&QName::local("D")];
        let chunk_e = plan.type_to_chunk[&QName::local("E")];
        assert_eq!(
            chunk_d, chunk_e,
            "Cyclic types D and E must be in the same chunk"
        );

        // 2. F depends on D, so chunk_of(F) >= chunk_of(D)
        let chunk_f = plan.type_to_chunk[&QName::local("F")];
        assert!(
            chunk_f >= chunk_d,
            "Dependent F must come in or after D's chunk"
        );

        // 3. For EVERY chunk and EVERY type, all dependencies must be in <= current chunk
        for (qname, &chunk_idx) in &plan.type_to_chunk {
            let deps = extract_type_dependencies(qname, &ir);
            for dep in deps {
                let dep_chunk = plan.type_to_chunk[&dep];
                assert!(
                    dep_chunk <= chunk_idx,
                    "Type {} in chunk {} depends on {} in chunk {} (forward reference violation!)",
                    qname,
                    chunk_idx,
                    dep,
                    dep_chunk
                );
            }
        }
    }
}
