# NyaTerm personal cluster workspace

This branch adds right-side Kubernetes Manager, Slurm Manager, Sync Input and
Split / Layouts panels. Existing panel IDs and stored data formats are preserved.
The four new IDs are appended/migrated without moving existing entries.

## Use

Connect an SSH session first. Install `kubectl` and configure its credentials on
that remote host, or use a Slurm login node with `squeue`, `sinfo`, `scontrol` and
`sdiag`. The panels execute over that session's authenticated multiplexed SSH
connection, never by typing into its terminal and never through Sync Input.
Only explicitly requested, read-only commands are implemented. There is no
create/delete/scale/submit/cancel/exec interface. Server RBAC still applies.

Kubernetes: choose Contexts to list names; enter the context/namespace to scope
subsequent queries. Empty fields use the remote CLI default. This does not run
`config use-context` or modify kubeconfig. Nodes, Pods and Deployments include
status columns; Events are chronological. Enter a pod name to fetch its last
200 log lines across containers (64 KiB limit, no streaming).
Slurm: Jobs, Partitions, Nodes, configured GPU resources and detailed node TRES,
controller reachability and scheduler statistics are available on demand.

Sync Input shows a red ON warning if any group or all-session broadcast is on.
A new window and new groups start OFF. Select a group to rename, toggle, delete,
or edit session membership directly in the sidebar. Turn ALL sync OFF stops all
routes. The full existing group manager remains available for pause/color tools.
Split uses the application's existing split and tiling operations.

## Architecture and security

- `nyaterm-cluster-commands`: pure typed allowlist and input-validation policy.
- `examples/plugins/cluster-tools`: external official v1 SDK command-draft plugin;
  compiled/packed as `cluster-tools.zip`. Install via the existing Plugins panel.
  The draft uses POSIX `timeout` and requires that utility on the destination.
- `nyaterm-remote-ops::cluster`: native SSH execution, 20-second deadline, no
  command/result tracing or history writes. Raw stderr is not surfaced.
- `features/cluster`: native panel adapter, ephemeral results, stale-result checks.

The official WIT v1 API only exposes initialize/invoke/shutdown and command/text
results. It cannot register panels or execute SSH. The native panel directly
uses the same validation crate, while the external plugin offers supported
command-draft actions. It does not pretend to add an unsupported plugin panel API.

Do not treat read-only as data anonymization: application logs and event messages
can themselves contain secrets. They are displayed on explicit request, never
persisted or written to application logs by these modules. Do not share screenshots
of sensitive output. Input is limited to 253 ASCII characters and a narrow name
alphabet; shell metacharacters, whitespace and leading dashes are rejected.

## Windows build/download/install

`Custom Windows x64` runs on feature/upstream pushes, main PRs and manual dispatch.
It tests Rust policy/integration and desktop state, checks the workspace, compiles
SDK plugins, builds MSVC x64 with the upstream NSIS/ConPTY packaging scripts and
verifies package contents. The artifact includes:

- `NyaTerm_2.0.4-custom.1_windows-x64-setup.exe`
- `NyaTerm_2.0.4-custom.1_windows-x64_portable.zip`
- `SOURCE_COMMIT.txt` and `SHA256SUMS.txt`

The separately uploaded `NyaTerm-cluster-tools-plugin-<commit>` artifact contains
`cluster-tools.zip`.

Open the passing GitHub Actions run, download its
`NyaTerm-custom-Windows-x64-<commit>` artifact and unzip the artifact wrapper.
Run the setup EXE for installation, or extract the portable ZIP into a separate
folder and start NyaTerm.exe. Keep all packaged helpers/DLLs beside it. The builds
are not code-signed. Installer uses the upstream app identity: it may replace an
existing installed NyaTerm; use Portable in a separate folder for side-by-side use.
Back up configuration through the existing export UI before switching builds.

This fork fails closed to manual custom updates, even in builds outside CI.
Startup/manual official update checks and signed download entry points are blocked;
GitHub artifacts are the update channel. Download/install newer custom builds manually.
No official update signing keys or release secrets are required.

## Upstream release updates

After this feature PR passes review and is merged, run **Propose upstream release
merge** with an official `vX.Y.Z` tag. It checks the official release exists, merges
it into a new `upstream/<tag>-<run>` branch, creates a Draft PR to main, and dispatches
Custom Windows x64 for that branch (GitHub-token pushes alone don't trigger CI).
Enable GitHub Actions PR creation in repository settings if required. A merge
conflict stops the job without changing main; resolve on a separate upstream branch
and push it to run CI. Review update-channel guards, all new panel routes and the
shared plugin policy before merging. Set branch protection to require Custom Windows
x64 and the upstream Rust gate; this workflow never merges automatically.

Manual equivalent: fetch the official release tag, branch from the custom main,
merge the tag, resolve conflicts, push an upstream branch, open a Draft PR, and only
merge after CI and Windows smoke testing. Keep dependency pins from the reviewed
release; never change those silently to make a build pass.

## Verification limits

Live Kubernetes/Slurm integration requires reachable SSH hosts and credentials.
CI desktop tests do not establish a live cluster connection or prove every pixel
of an interactive Windows desktop. Test each panel against your real hosts before
operational use. Review the PR and Actions result for actual test/build outcomes;
the presence of this workflow is not evidence that a build has succeeded.
