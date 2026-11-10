//! Read-only command policy shared by the native panel and sandboxed SDK plugin.
//! No credentials, kubeconfig contents, shell fragments, or mutation verbs are accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Query {
    Contexts,
    Namespaces,
    Nodes,
    Pods,
    Deployments,
    Events,
    Logs,
    Jobs,
    Partitions,
    SlurmNodes,
    Gpu,
    Scheduler,
}
impl Query {
    pub const KUBERNETES: [Self; 7] = [
        Self::Contexts,
        Self::Namespaces,
        Self::Nodes,
        Self::Pods,
        Self::Deployments,
        Self::Events,
        Self::Logs,
    ];
    pub const SLURM: [Self; 5] = [
        Self::Jobs,
        Self::Partitions,
        Self::SlurmNodes,
        Self::Gpu,
        Self::Scheduler,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Contexts => "Contexts",
            Self::Namespaces => "Namespaces",
            Self::Nodes | Self::SlurmNodes => "Nodes",
            Self::Pods => "Pods",
            Self::Deployments => "Deployments",
            Self::Events => "Events",
            Self::Logs => "Logs",
            Self::Jobs => "Jobs",
            Self::Partitions => "Partitions",
            Self::Gpu => "GPU resources",
            Self::Scheduler => "Scheduler status",
        }
    }
}
#[derive(Clone, Default)]
pub struct Selection {
    pub context: String,
    pub namespace: String,
    pub pod: String,
}
fn valid(value: &str, context: bool) -> bool {
    value.len() <= 253
        && !value.starts_with('-')
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric() || b"-_.".contains(&b) || (context && b"/:@".contains(&b))
        })
}
pub fn command(query: Query, selection: &Selection) -> Result<String, &'static str> {
    if !valid(&selection.context, true)
        || !valid(&selection.namespace, false)
        || !valid(&selection.pod, false)
    {
        return Err("Invalid context, namespace or pod name (maximum 253 ASCII characters)");
    }
    let slurm = match query {
        Query::Jobs => Some("squeue --format='%.18i %.12P %.32j %.12u %.2t %.12M %.6D %R'"),
        Query::Partitions => Some("sinfo --format='%P %a %l %D %t %N'"),
        Query::SlurmNodes => Some("sinfo -N --format='%N %P %t %c %m %e %G'"),
        Query::Gpu => Some("sinfo -N --format='%N %t %G %b' && scontrol show nodes"),
        Query::Scheduler => Some("scontrol ping && sdiag"),
        _ => None,
    };
    if let Some(cmd) = slurm {
        return Ok(cmd.into());
    }
    let mut cmd = String::from("kubectl --request-timeout=15s");
    if !selection.context.is_empty() {
        cmd.push_str(&format!(" --context='{}'", selection.context));
    }
    if !selection.namespace.is_empty() {
        cmd.push_str(&format!(" --namespace='{}'", selection.namespace));
    }
    cmd.push_str(match query {
        Query::Contexts => " config get-contexts",
        Query::Namespaces => " get namespaces",
        Query::Nodes => " get nodes -o wide",
        Query::Pods => " get pods -o wide",
        Query::Deployments => " get deployments -o wide",
        Query::Events => " get events --sort-by=.metadata.creationTimestamp",
        Query::Logs => {
            if selection.pod.is_empty() { return Err("Select a pod name before reading logs"); }
            return Ok(format!("{cmd} logs '{}' --all-containers=true --tail=200 --limit-bytes=65536 --timestamps=true", selection.pod));
        }
        _ => unreachable!(),
    });
    Ok(cmd)
}
#[cfg(test)]
mod tests {
    use super::{Query, Selection, command};
    #[test]
    fn rejects_shell_injection_and_option_injection() {
        for value in ["a;id", "$(id)", "a'b", "a\nb", "--token=x", "a b", "`id`"] {
            assert!(
                command(
                    Query::Pods,
                    &Selection {
                        context: value.into(),
                        ..Default::default()
                    }
                )
                .is_err()
            );
        }
    }
    #[test]
    fn logs_are_bounded_and_require_pod() {
        assert!(command(Query::Logs, &Selection::default()).is_err());
        let cmd = command(
            Query::Logs,
            &Selection {
                pod: "pod-1".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(cmd.contains("--limit-bytes=65536"));
        assert!(!cmd.contains("--follow"));
    }
    #[test]
    fn context_selection_does_not_change_remote_kubeconfig() {
        let cmd = command(
            Query::Pods,
            &Selection {
                context: "arn:aws:eks/us-east-1@cluster".into(),
                namespace: "team-a".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(cmd.contains("--context='arn:aws:eks/us-east-1@cluster'"));
        assert!(!cmd.contains("use-context"));
    }
    #[test]
    fn every_view_has_a_read_only_command() {
        for query in Query::KUBERNETES.into_iter().chain(Query::SLURM) {
            let cmd = command(
                query,
                &Selection {
                    pod: "pod-1".into(),
                    ..Default::default()
                },
            )
            .unwrap();
            for forbidden in [
                " delete ",
                " apply ",
                " exec ",
                "scancel",
                "sbatch",
                "sudo",
                "config view",
            ] {
                assert!(!cmd.contains(forbidden));
            }
        }
    }
}
