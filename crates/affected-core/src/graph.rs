use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;
use std::collections::{HashMap, HashSet};

use crate::types::{PackageId, ProjectGraph};

/// Dependency graph wrapper around petgraph.
pub struct DepGraph {
    graph: DiGraph<PackageId, ()>,
    node_map: HashMap<PackageId, NodeIndex>,
}

impl DepGraph {
    /// Build from a ProjectGraph. Edges go from dependent → dependency.
    pub fn from_project_graph(pg: &ProjectGraph) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        for id in pg.packages.keys() {
            let idx = graph.add_node(id.clone());
            node_map.insert(id.clone(), idx);
        }

        for (from, to) in &pg.edges {
            if let (Some(&from_idx), Some(&to_idx)) = (node_map.get(from), node_map.get(to)) {
                graph.add_edge(from_idx, to_idx, ());
            }
        }

        Self { graph, node_map }
    }

    /// Given a set of directly changed packages, return all transitively
    /// affected packages (changed + everything that depends on them).
    ///
    /// Uses BFS on the reversed graph: if A→B means "A depends on B",
    /// then in the reversed graph B→A, and BFS from a changed node B
    /// finds all packages that transitively depend on B.
    pub fn affected_by(&self, changed: &HashSet<PackageId>) -> HashSet<PackageId> {
        let reversed = petgraph::visit::Reversed(&self.graph);
        let mut result = HashSet::new();

        for pkg in changed {
            if let Some(&start) = self.node_map.get(pkg) {
                let mut bfs = Bfs::new(&reversed, start);
                while let Some(node) = bfs.next(&reversed) {
                    result.insert(self.graph[node].clone());
                }
            }
        }

        result
    }

    /// Return all package IDs in the graph.
    pub fn all_packages(&self) -> Vec<&PackageId> {
        self.graph.node_weights().collect()
    }

    /// Generate DOT format output for graphviz visualization.
    pub fn to_dot(&self) -> String {
        let mut lines = vec!["digraph dependencies {".to_string()];
        for edge in self.graph.edge_indices() {
            let (a, b) = self.graph.edge_endpoints(edge).unwrap();
            lines.push(format!(
                "    \"{}\" -> \"{}\";",
                self.graph[a], self.graph[b]
            ));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    /// Return all edges as (from, to) pairs for display.
    pub fn edges(&self) -> Vec<(&PackageId, &PackageId)> {
        self.graph
            .edge_indices()
            .map(|e| {
                let (a, b) = self.graph.edge_endpoints(e).unwrap();
                (&self.graph[a], &self.graph[b])
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Package;
    use std::path::PathBuf;

    fn make_graph(
        names: &[&str],
        edges: &[(&str, &str)],
    ) -> ProjectGraph {
        let mut packages = HashMap::new();
        for name in names {
            let id = PackageId(name.to_string());
            packages.insert(
                id.clone(),
                Package {
                    id: id.clone(),
                    name: name.to_string(),
                    version: None,
                    path: PathBuf::from(format!("/{name}")),
                    manifest_path: PathBuf::from(format!("/{name}/Cargo.toml")),
                },
            );
        }
        let edges = edges
            .iter()
            .map(|(a, b)| (PackageId(a.to_string()), PackageId(b.to_string())))
            .collect();
        ProjectGraph {
            packages,
            edges,
            root: PathBuf::from("/"),
        }
    }

    #[test]
    fn test_linear_chain() {
        // cli -> api -> core
        let pg = make_graph(
            &["core", "api", "cli"],
            &[("api", "core"), ("cli", "api")],
        );
        let dg = DepGraph::from_project_graph(&pg);

        let changed: HashSet<_> = [PackageId("core".into())].into();
        let affected = dg.affected_by(&changed);

        assert!(affected.contains(&PackageId("core".into())));
        assert!(affected.contains(&PackageId("api".into())));
        assert!(affected.contains(&PackageId("cli".into())));
        assert_eq!(affected.len(), 3);
    }

    #[test]
    fn test_leaf_change() {
        // cli -> api -> core
        let pg = make_graph(
            &["core", "api", "cli"],
            &[("api", "core"), ("cli", "api")],
        );
        let dg = DepGraph::from_project_graph(&pg);

        let changed: HashSet<_> = [PackageId("cli".into())].into();
        let affected = dg.affected_by(&changed);

        assert!(affected.contains(&PackageId("cli".into())));
        assert_eq!(affected.len(), 1);
    }

    #[test]
    fn test_diamond_dependency() {
        //   app
        //  /   \
        // ui   api
        //  \   /
        //  core
        let pg = make_graph(
            &["core", "ui", "api", "app"],
            &[
                ("ui", "core"),
                ("api", "core"),
                ("app", "ui"),
                ("app", "api"),
            ],
        );
        let dg = DepGraph::from_project_graph(&pg);

        let changed: HashSet<_> = [PackageId("core".into())].into();
        let affected = dg.affected_by(&changed);

        assert_eq!(affected.len(), 4);
    }

    #[test]
    fn test_isolated_package() {
        let pg = make_graph(
            &["core", "api", "standalone"],
            &[("api", "core")],
        );
        let dg = DepGraph::from_project_graph(&pg);

        let changed: HashSet<_> = [PackageId("core".into())].into();
        let affected = dg.affected_by(&changed);

        assert!(affected.contains(&PackageId("core".into())));
        assert!(affected.contains(&PackageId("api".into())));
        assert!(!affected.contains(&PackageId("standalone".into())));
    }
}
