use nyaterm_cluster_commands::{Query, Selection, command};
use nyaterm_plugin_api::{ActionInput, ActionResult, CommandDraft, ErrorKind, Plugin, PluginError, Value};
#[derive(Default)]
struct ClusterTools;
impl Plugin for ClusterTools {
    fn invoke(&mut self, input: ActionInput) -> Result<ActionResult, PluginError> {
        let query = match input.action.as_str() {
            "contexts" => Query::Contexts, "namespaces" => Query::Namespaces,
            "nodes" => Query::Nodes, "pods" => Query::Pods, "deployments" => Query::Deployments,
            "events" => Query::Events, "logs" => Query::Logs, "jobs" => Query::Jobs,
            "partitions" => Query::Partitions, "slurm-nodes" => Query::SlurmNodes,
            "gpu" => Query::Gpu, "scheduler" => Query::Scheduler,
            _ => return Err(PluginError { kind: ErrorKind::UnsupportedAction, message: "Unknown read-only query".into() }),
        };
        let text = |key: &str| input.arguments.iter().find_map(|a| match &a.value { Value::Text(v) if a.name == key => Some(v.clone()), _ => None }).unwrap_or_default();
        let selection = Selection { context: text("context"), namespace: text("namespace"), pod: text("pod") };
        let command = command(query, &selection).map_err(|message| PluginError { kind: ErrorKind::InvalidInput, message: message.into() })?;
        Ok(ActionResult::Command(CommandDraft { title: format!("Read-only {}", query.label()), command: format!("timeout 20s sh -c '{}'", command.replace('\'', "'\"'\"'")) }))
    }
}
nyaterm_plugin_api::register_plugin!(ClusterTools);
