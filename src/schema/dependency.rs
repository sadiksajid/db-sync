use crate::schema::types::MySQLSchema;
use anyhow::Result;
use petgraph::{algo::toposort, Graph};
use std::collections::HashMap;

pub struct DependencyGraph {
    graph: Graph<String, ()>,
    node_indices: HashMap<String, petgraph::graph::NodeIndex>,
}

impl DependencyGraph {
    pub fn from_schema(schema: &MySQLSchema) -> Self {
        let mut graph = Graph::<String, ()>::new();
        let mut node_indices = HashMap::new();

        // Create nodes for all tables
        for table_name in schema.tables.keys() {
            let idx = graph.add_node(table_name.clone());
            node_indices.insert(table_name.clone(), idx);
        }

        // Add edges based on foreign keys
        for table in schema.tables.values() {
            if let Some(&source_idx) = node_indices.get(&table.name) {
                for fk in &table.foreign_keys {
                    if let Some(&target_idx) = node_indices.get(&fk.referenced_table) {
                        graph.add_edge(source_idx, target_idx, ());
                    }
                }
            }
        }

        Self {
            graph,
            node_indices,
        }
    }

    pub fn get_creation_order(&self) -> Result<Vec<String>> {
        // Topological sort: tables with no dependencies come first
        // In our graph, edge A -> B means A depends on B, so B should be created first
        // We need to reverse the graph for topological sort
        let mut reversed_graph = Graph::<String, ()>::new();
        let mut reversed_indices = HashMap::new();

        // Create nodes
        for node in self.graph.node_indices() {
            let name = self.graph[node].clone();
            let idx = reversed_graph.add_node(name.clone());
            reversed_indices.insert(name, idx);
        }

        // Reverse edges
        for edge in self.graph.edge_indices() {
            let (source, target) = self.graph.edge_endpoints(edge).unwrap();
            let source_name = &self.graph[source];
            let target_name = &self.graph[target];
            if let (Some(&source_idx), Some(&target_idx)) =
                (reversed_indices.get(source_name), reversed_indices.get(target_name))
            {
                reversed_graph.add_edge(target_idx, source_idx, ());
            }
        }

        // Perform topological sort
        match toposort(&reversed_graph, None) {
            Ok(sorted_indices) => {
                let mut result = Vec::new();
                for idx in sorted_indices {
                    result.push(reversed_graph[idx].clone());
                }
                Ok(result)
            }
            Err(cycle) => {
                let cycle_node = reversed_graph[cycle.node_id()].clone();
                Err(anyhow::anyhow!(
                    "Circular dependency detected involving table: {}",
                    cycle_node
                ))
            }
        }
    }

    pub fn get_table_order_for_data_transfer(&self) -> Result<Vec<String>> {
        // For data transfer, we need parent tables before child tables
        // This is the same as creation order
        self.get_creation_order()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{Column, ForeignKey, MySQLSchema, TableSchema};

    #[test]
    fn test_simple_dependency() {
        let mut schema = MySQLSchema::new();

        // Parent table
        schema.add_table(TableSchema {
            name: "parent".to_string(),
            columns: vec![],
            primary_keys: vec!["id".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
        });

        // Child table
        schema.add_table(TableSchema {
            name: "child".to_string(),
            columns: vec![],
            primary_keys: vec!["id".to_string()],
            foreign_keys: vec![ForeignKey {
                name: "fk_parent".to_string(),
                column_name: "parent_id".to_string(),
                referenced_table: "parent".to_string(),
                referenced_column: "id".to_string(),
            }],
            indexes: vec![],
        });

        let graph = DependencyGraph::from_schema(&schema);
        let order = graph.get_creation_order().unwrap();
        assert_eq!(order[0], "parent");
        assert_eq!(order[1], "child");
    }
}

