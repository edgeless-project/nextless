#[derive(serde::Serialize, Clone)]
pub struct ResourceProvider {
    pub class_type: String,
    pub outputs: Vec<String>,
}

impl crate::ir::ResourceProvider for ResourceProvider {
    fn class_type(&self) -> String {
        self.class_type.clone()
    }

    fn outputs(&self) -> Vec<String> {
        self.outputs.clone()
    }
}
