//! Default OpenCode agent list for environments without a running OpenCode server.

use serde_json::Value;

use crate::backend::AgentInfo;

const DEFAULT_OPENCODE_AGENTS: [&str; 10] = [
    "Sisyphus",
    "oracle",
    "librarian",
    "explore",
    "frontend-ui-ux-engineer",
    "document-writer",
    "multimodal-looker",
    "Prometheus",
    "Metis",
    "Momus",
];

pub fn default_agent_names() -> &'static [&'static str] {
    &DEFAULT_OPENCODE_AGENTS
}

pub fn default_agent_infos() -> Vec<AgentInfo> {
    DEFAULT_OPENCODE_AGENTS
        .iter()
        .map(|name| AgentInfo {
            id: (*name).to_string(),
            name: (*name).to_string(),
        })
        .collect()
}

pub fn default_agent_payload() -> Value {
    Value::Array(
        DEFAULT_OPENCODE_AGENTS
            .iter()
            .map(|name| Value::String((*name).to_string()))
            .collect(),
    )
}
