#[derive(Debug, Clone)]
pub struct FunctionId {
    pub node_id: uuid::Uuid,
    pub function_id: uuid::Uuid,
}

impl FunctionId {
    pub fn new(node_id: uuid::Uuid) -> Self {
        Self {
            node_id: node_id,
            function_id: uuid::Uuid::new_v4(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionClassSpecification {
    pub function_class_id: String,
    pub function_class_type: String,
    pub function_class_version: String,
    pub function_class_inlude_code: Vec<u8>,
    pub output_callback_declarations: Vec<String>,
}

#[derive(Debug)]
pub struct SpawnFunctionRequest {
    pub function_id: Option<FunctionId>,
    pub code: FunctionClassSpecification,
    pub output_callback_definitions: std::collections::HashMap<String, FunctionId>,
    pub return_continuation: FunctionId,
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Debug)]
pub struct UpdateFunctionLinksRequest {
    pub function_id: Option<FunctionId>,
    pub output_callback_definitions: std::collections::HashMap<String, FunctionId>,
    pub return_continuation: FunctionId,
}

#[async_trait::async_trait]
pub trait FunctionInstanceAPI: Sync {
    async fn start_function_instance(&mut self, spawn_request: SpawnFunctionRequest) -> anyhow::Result<FunctionId>;
    async fn stop_function_instance(&mut self, id: FunctionId) -> anyhow::Result<()>;
    async fn update_function_instance_links(&mut self, update: UpdateFunctionLinksRequest) -> anyhow::Result<()>;
}
