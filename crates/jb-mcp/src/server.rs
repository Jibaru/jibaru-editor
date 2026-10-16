//! rmcp glue: maps MCP requests onto `tools::State`.

use crate::tools::{tool_defs, State, RESOURCES};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult,
    Resource, ResourceContents, ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData, ServerHandler};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

const INSTRUCTIONS: &str = "Jibaru Editor: edit video projects through an undoable command bus. \
Start with new_project or open_project, add media with import_media, then edit with execute_command \
(the exact Command schema is in its inputSchema and at jb://schema/command). Inspect with get_project, \
find_clips and evaluate_frame; undo/redo are available; call save_project to persist. \
Times in commands are integer ticks: 10_000_000 per second. Errors carry a `code` and a `hint` telling you how to fix the call.";

#[derive(Clone, Default)]
pub struct JbServer {
    state: Arc<Mutex<State>>,
}

impl JbServer {
    pub fn new() -> Self {
        Self::default()
    }
}

fn to_object(v: Value) -> serde_json::Map<String, Value> {
    match v {
        Value::Object(o) => o,
        _ => serde_json::Map::new(),
    }
}

impl ServerHandler for JbServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_resources().build())
            .with_server_info(Implementation::new("jb-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools = tool_defs()
            .into_iter()
            .map(|d| {
                Tool::new(d.name, d.description, to_object(d.input_schema)).with_title(d.title).with_annotations(
                    ToolAnnotations::new().read_only(d.read_only).destructive(false).open_world(false),
                )
            })
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let args = request.arguments.map(Value::Object).unwrap_or(Value::Null);
        let name = request.name.to_string();
        let state = self.state.clone();
        // Probing media shells out to ffprobe; keep the async runtime free.
        let result =
            tokio::task::spawn_blocking(move || state.lock().unwrap_or_else(|e| e.into_inner()).call(&name, args))
                .await
                .map_err(|e| ErrorData::internal_error(format!("tool panicked: {e}"), None))?;
        Ok(match result {
            Ok(v) => CallToolResult::structured(v).into(),
            Err(e) => {
                let mut r = CallToolResult::error(vec![ContentBlock::text(
                    serde_json::to_string_pretty(&json!({ "error": e })).unwrap_or_default(),
                )]);
                r.structured_content = Some(json!({ "error": e }));
                r.into()
            }
        })
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(
            RESOURCES
                .iter()
                .map(|(uri, name, desc)| {
                    Resource::new(*uri, *name).with_description(*desc).with_mime_type("application/json")
                })
                .collect(),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match state.read_resource(&request.uri) {
            Ok(v) => Ok(ReadResourceResult::new(vec![ResourceContents::text(
                serde_json::to_string_pretty(&v).unwrap_or_default(),
                request.uri.clone(),
            )
            .with_mime_type("application/json")])
            .into()),
            Err(e) => Err(ErrorData::resource_not_found(e.message.clone(), Some(json!({ "error": e })))),
        }
    }
}
