//! Bounded, read-only probes over the existing authenticated SSH lease.
use nyaterm_cluster_commands::{Query, Selection, command};
use nyaterm_transport::ssh_execution::streaming::{SshExecEvent, stream_ssh_command};
use nyaterm_transport::{SshMultiplexHandle, SshSessionConfig};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub fn query(
    config: SshSessionConfig,
    multiplex: SshMultiplexHandle,
    query: Query,
    selection: Selection,
) -> Result<String, String> {
    let command = command(query, &selection).map_err(str::to_string)?;
    multiplex
        .ensure_matches_config(&config)
        .map_err(|_| "SSH session unavailable".to_string())?;
    let connection = multiplex.clone();
    multiplex
        .block_on(async move {
            let (tx, mut rx) = tokio::sync::mpsc::channel(16);
            let producer = stream_ssh_command(
                config,
                Some(connection),
                command.into_bytes(),
                Duration::from_secs(20),
                CancellationToken::new(),
                tx,
            );
            let consumer = async move {
                let mut output = Vec::new();
                let mut success = false;
                let mut failed = false;
                let mut truncated = false;
                while let Some(event) = rx.recv().await {
                    match event {
                        SshExecEvent::Output {
                            data,
                            stderr: false,
                        } => {
                            let remaining = 131072usize.saturating_sub(output.len());
                            truncated |= data.len() > remaining;
                            output.extend_from_slice(&data[..data.len().min(remaining)]);
                        }
                        SshExecEvent::OutputDropped { .. } => truncated = true,
                        SshExecEvent::ExitStatus(code) => {
                            success = code == 0;
                        }
                        SshExecEvent::TimedOut
                        | SshExecEvent::Failed
                        | SshExecEvent::ExitSignal { .. }
                        | SshExecEvent::CancelledBeforeStart => failed = true,
                        _ => {}
                    }
                }
                if failed || !success {
                    anyhow::bail!("Read-only query failed or timed out");
                }
                let mut text = display_output(&String::from_utf8_lossy(&output));
                if truncated {
                    text.push_str("\n[output truncated at 128 KiB]");
                }
                Ok(text)
            };
            let (_, result) = tokio::join!(producer, consumer);
            result
        })
        .map_err(|_| {
            "Query failed: check CLI installation, permissions and the 20-second deadline"
                .to_string()
        })
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
