//! Bounded, read-only probes over the existing authenticated SSH lease.
use nyaterm_cluster_commands::{Query, Selection, command};
use nyaterm_transport::{SshMultiplexHandle, SshSessionConfig, ssh_execution::SshExecService};
use std::time::Duration;

pub fn query(
    config: SshSessionConfig,
    multiplex: SshMultiplexHandle,
    query: Query,
    selection: Selection,
) -> Result<String, String> {
    let command = command(query, &selection).map_err(str::to_string)?;
    let service = SshExecService::with_multiplex(config, multiplex)
        .map_err(|_| "SSH session unavailable".to_string())?;
    // Do not include raw transport errors, command output, or stderr in telemetry/errors.
    let result = service
        .run_command(command, Duration::from_secs(20))
        .map_err(|_| "Query failed or exceeded the 20-second timeout".to_string())?;
    if result.exit_status != Some(0) {
        return Err("Query failed: check CLI installation, credentials and read permissions on this SSH host".into());
    }
    Ok(display_output(&result.stdout))
}
fn display_output(output: &str) -> String {
    // Result is ephemeral UI content, never tracing/command history/persistence.
    let mut text: String = output
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .take(131072)
        .collect();
    if output.chars().count() > 131072 {
        text.push_str("\n[output truncated]");
    }
    text
}
#[cfg(test)]
mod tests {
    #[test]
    fn output_is_bounded_and_terminal_controls_are_removed() {
        assert_eq!(super::display_output("a\u{1b}\u{0}b\nc"), "ab\nc");
        assert!(super::display_output(&"a".repeat(200000)).len() < 131100);
    }
}
