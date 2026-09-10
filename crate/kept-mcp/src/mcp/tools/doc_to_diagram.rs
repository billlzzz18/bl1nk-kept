//! Document to Diagram & Agent Graph Tool
//!
//! Analyzes document semantics (headings, steps, actors, transitions) to synthesize:
//! 1. Mermaid Diagrams (Flowchart, Sequence, State, Architecture)
//! 2. Agent Graph DAGs (Multi-agent topologies with nodes, edges, and handoff contracts)
//! 3. Rendered SVG visualizations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::mcp::core::{
    invalid_args, McpResult, RequestHandlerExtra, SchemaBuilder, ToolHandler, ToolInfo,
};

#[derive(Default)]
pub struct DocumentToDiagramTool;

#[derive(Deserialize)]
struct DocToDiagramInput {
    content: String,
    diagram_type: Option<String>,
    target_format: Option<String>,
    render_svg: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub handoff_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedGraph {
    pub diagram_type: String,
    pub root_node: Option<String>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl DocumentToDiagramTool {
    /// Extract nodes, edges, and flow topology from a document
    pub fn analyze_document(content: &str, requested_type: Option<&str>) -> ExtractedGraph {
        let lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();
        let mut nodes: Vec<GraphNode> = Vec::new();
        let mut edges: Vec<GraphEdge> = Vec::new();
        let mut current_group: Option<String> = None;
        let mut previous_node_id: Option<String> = None;

        let detected_type = match requested_type.unwrap_or("auto") {
            "auto" => {
                let lower = content.to_lowercase();
                if lower.contains("sequence")
                    || lower.contains("request") && lower.contains("response")
                {
                    "sequence"
                } else if lower.contains("state")
                    || lower.contains("status") && lower.contains("transition")
                {
                    "state"
                } else if lower.contains("agent")
                    || lower.contains("handoff")
                    || lower.contains("router")
                {
                    "agent_graph"
                } else {
                    "flowchart"
                }
            },
            other => other,
        };

        for line in lines {
            if line.is_empty() {
                continue;
            }

            // Headings define groups / phases
            if line.starts_with('#') {
                let heading_title = line.trim_start_matches('#').trim().to_string();
                current_group = Some(heading_title);
                continue;
            }

            // Direct arrow notations in document (e.g. A -> B or A --> B)
            if line.contains("->") || line.contains("-->") || line.contains("=>") {
                let separator = if line.contains("-->") {
                    "-->"
                } else if line.contains("=>") {
                    "=>"
                } else {
                    "->"
                };

                let parts: Vec<&str> = line.split(separator).collect();
                if parts.len() >= 2 {
                    let from_label = parts[0].trim().trim_start_matches(['-', '*', '•']).trim();
                    let to_part = parts[1].trim();

                    let (to_label, edge_label) = if to_part.contains(':') {
                        let sub: Vec<&str> = to_part.splitn(2, ':').collect();
                        (sub[0].trim(), Some(sub[1].trim().to_string()))
                    } else if let Some(open_idx) = to_part.find('(') {
                        if to_part.ends_with(')') {
                            let dest = to_part[..open_idx].trim();
                            let payload = to_part[open_idx + 1..to_part.len() - 1].trim();
                            (dest, Some(payload.to_string()))
                        } else {
                            (to_part, None)
                        }
                    } else {
                        (to_part, None)
                    };

                    let from_id = Self::sanitize_id(from_label);
                    let to_id = Self::sanitize_id(to_label);

                    if !nodes.iter().any(|n| n.id == from_id) && !from_id.is_empty() {
                        nodes.push(GraphNode {
                            id: from_id.clone(),
                            label: from_label.to_string(),
                            node_type: "component".to_string(),
                            group: current_group.clone(),
                        });
                    }

                    if !nodes.iter().any(|n| n.id == to_id) && !to_id.is_empty() {
                        nodes.push(GraphNode {
                            id: to_id.clone(),
                            label: to_label.to_string(),
                            node_type: "component".to_string(),
                            group: current_group.clone(),
                        });
                    }

                    if !from_id.is_empty() && !to_id.is_empty() {
                        edges.push(GraphEdge {
                            from: from_id,
                            to: to_id,
                            label: edge_label.clone(),
                            handoff_context: edge_label,
                        });
                    }
                    continue;
                }
            }

            // Bullet points as sequential steps / agent tasks
            if line.starts_with('-')
                || line.starts_with('*')
                || line.starts_with('•')
                || (line.chars().next().is_some_and(|c| c.is_ascii_digit()) && line.contains('.'))
            {
                let item_text = line
                    .trim_start_matches(|c: char| {
                        c == '-'
                            || c == '*'
                            || c == '•'
                            || c.is_ascii_digit()
                            || c == '.'
                            || c == ' '
                    })
                    .trim();

                if !item_text.is_empty() {
                    let node_id = Self::sanitize_id(item_text);
                    if !nodes.iter().any(|n| n.id == node_id) {
                        nodes.push(GraphNode {
                            id: node_id.clone(),
                            label: item_text.to_string(),
                            node_type: "step".to_string(),
                            group: current_group.clone(),
                        });

                        if let Some(prev) = previous_node_id.take() {
                            edges.push(GraphEdge {
                                from: prev,
                                to: node_id.clone(),
                                label: Some("next_step".to_string()),
                                handoff_context: None,
                            });
                        }
                        previous_node_id = Some(node_id);
                    }
                }
            }
        }

        let root_node = nodes.first().map(|n| n.id.clone());

        ExtractedGraph {
            diagram_type: detected_type.to_string(),
            root_node,
            nodes,
            edges,
        }
    }

    fn sanitize_id(raw: &str) -> String {
        let mut id = String::new();
        for c in raw.chars() {
            if c.is_alphanumeric() || c == '_' {
                id.push(c);
            } else if (c.is_whitespace() || c == '-') && !id.ends_with('_') {
                id.push('_');
            }
        }
        id.trim_matches('_').to_lowercase()
    }

    /// Generate Mermaid markup from the extracted graph
    pub fn generate_mermaid(graph: &ExtractedGraph) -> String {
        match graph.diagram_type.as_str() {
            "sequence" => {
                let mut out = String::from("sequenceDiagram\n  autonumber\n");
                for node in &graph.nodes {
                    out.push_str(&format!("  participant {} as {}\n", node.id, node.label));
                }
                for edge in &graph.edges {
                    let msg = edge.label.as_deref().unwrap_or("trigger");
                    out.push_str(&format!("  {}->>{}: {}\n", edge.from, edge.to, msg));
                }
                out
            },
            "state" => {
                let mut out = String::from("stateDiagram-v2\n");
                if let Some(root) = &graph.root_node {
                    out.push_str(&format!("  [*] --> {}\n", root));
                }
                for edge in &graph.edges {
                    let label = edge
                        .label
                        .as_deref()
                        .map(|l| format!(": {}", l))
                        .unwrap_or_default();
                    out.push_str(&format!("  {} --> {}{}\n", edge.from, edge.to, label));
                }
                out
            },
            _ => {
                // Default Flowchart TD
                let mut out = String::from("graph TD\n");
                for node in &graph.nodes {
                    out.push_str(&format!("  {}[\"{}\"]\n", node.id, node.label));
                }
                for edge in &graph.edges {
                    if let Some(lbl) = &edge.label {
                        out.push_str(&format!("  {} -->|\"{}\"| {}\n", edge.from, lbl, edge.to));
                    } else {
                        out.push_str(&format!("  {} --> {}\n", edge.from, edge.to));
                    }
                }
                out
            },
        }
    }

    /// Generate Agent Graph DAG JSON
    pub fn generate_agent_graph_dag(graph: &ExtractedGraph) -> Value {
        json!({
            "key": "extracted-agent-graph",
            "name": "Extracted Agent Graph",
            "rootConfigKey": graph.root_node,
            "nodes": graph.nodes.iter().map(|n| {
                json!({
                    "key": n.id,
                    "name": n.label,
                    "group": n.group,
                    "type": n.node_type
                })
            }).collect::<Vec<_>>(),
            "edges": graph.edges.iter().enumerate().map(|(i, e)| {
                json!({
                    "key": format!("edge_{}", i),
                    "sourceConfig": e.from,
                    "targetConfig": e.to,
                    "handoff": {
                        "action": e.label,
                        "context": e.handoff_context
                    }
                })
            }).collect::<Vec<_>>()
        })
    }
}

#[async_trait]
impl ToolHandler for DocumentToDiagramTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: DocToDiagramInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;

        let graph = Self::analyze_document(&input.content, input.diagram_type.as_deref());
        let target_fmt = input.target_format.unwrap_or_else(|| "mermaid".to_string());
        let mermaid_markup = Self::generate_mermaid(&graph);

        let mut response = json!({
            "success": true,
            "diagram_type": graph.diagram_type,
            "nodes_count": graph.nodes.len(),
            "edges_count": graph.edges.len(),
            "graph_topology": graph,
            "mermaid_code": mermaid_markup,
        });

        if target_fmt == "agent_graph_dag" || target_fmt == "agent_graph" || target_fmt == "dag" {
            let dag = Self::generate_agent_graph_dag(&graph);
            response["agent_graph_dag"] = dag;
        }

        if input.render_svg.unwrap_or(false) {
            match mermaid_rs_renderer::render(&mermaid_markup) {
                Ok(svg) => {
                    response["rendered_svg"] = json!(svg);
                },
                Err(e) => {
                    response["render_warning"] = json!(format!("SVG render fallback: {}", e));
                },
            }
        }

        Ok(response)
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "document_to_diagram",
            Some("AI Semantic Document-to-Diagram & Agent Graph generator (extracts nodes, edges, flows into Mermaid / Agent Graph DAG)".to_string()),
            SchemaBuilder::new()
                .param("content", "Document content, markdown, workflow notes, or specification")
                .optional_param("diagram_type", "Diagram type: auto, flowchart, sequence, state, agent_graph")
                .optional_param("target_format", "Output format: mermaid, agent_graph_dag")
                .optional_bool_param("render_svg", "Whether to render and include full vector SVG string")
                .build(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_to_diagram_flowchart() {
        let doc = "# Architecture\nClient -> API Gateway\nAPI Gateway -> Auth Service\nAPI Gateway -> Order DB (save_order)\n";
        let graph = DocumentToDiagramTool::analyze_document(doc, Some("flowchart"));
        assert!(!graph.nodes.is_empty());
        assert!(!graph.edges.is_empty());

        let mermaid = DocumentToDiagramTool::generate_mermaid(&graph);
        assert!(mermaid.starts_with("graph TD"));
        assert!(mermaid.contains("client"));
        assert!(mermaid.contains("api_gateway"));
    }

    #[test]
    fn test_document_to_agent_graph_dag() {
        let doc = "# Support Workflow\n- Triage Agent\n- Specialist Agent\n- Summary Agent\n";
        let graph = DocumentToDiagramTool::analyze_document(doc, Some("agent_graph"));
        let dag = DocumentToDiagramTool::generate_agent_graph_dag(&graph);
        assert!(dag["rootConfigKey"].is_string());
        assert!(dag["nodes"].is_array());
        assert!(dag["edges"].is_array());
    }
}
