#[derive(serde::Serialize, Clone)]
pub struct ResourceProvider {
    pub class_type: String,
    pub outputs: Vec<String>,
    pub instance_limit: Option<usize>,
}

impl crate::ir::ResourceProvider for ResourceProvider {
    fn class_type(&self) -> String {
        self.class_type.clone()
    }

    fn outputs(&self) -> Vec<String> {
        self.outputs.clone()
    }

    fn instance_limit(&self) -> Option<usize> {
        self.instance_limit.clone()
    }
}
