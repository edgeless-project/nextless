pub mod default;
pub mod default_logical;
pub mod default_physical;

pub trait TransformationPipeline<GlobalState> {
    fn apply_all(
        &mut self,
        workflow: &mut super::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &mut GlobalState,
    );
    fn apply_dynamic(
        &mut self,
        workflow: &mut super::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &mut GlobalState,
    );
}
