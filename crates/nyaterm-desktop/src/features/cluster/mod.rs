use crate::features::{NyaTermApp, runtime_jobs::await_blocking_job, text_inputs::TextInputSetup};
use crate::models::{NavItem, SmartSplitMode, WorkspaceSplitDirection};
use crate::widgets::small_button;
use gpui::{Context, IntoElement, div, prelude::*, px, rgb};
use nyaterm_cluster_commands::{Query, Selection};
use nyaterm_ui::{NyaInputShell, NyaScrollable};

#[derive(Default)]
pub(in crate::features) struct ClusterFeatureState {
    selection: Selection,
    output: String,
    session_id: Option<String>,
    query: Option<Query>,
    pending: bool,
    generation: u64,
}

impl NyaTermApp {
    pub(in crate::features) fn apply_cluster_input(
        &mut self,
        field: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        match field {
            "context" => self.cluster.selection.context = text,
            "namespace" => self.cluster.selection.namespace = text,
            "pod" => self.cluster.selection.pod = text,
            _ => return,
        }
        // Invalidate in-flight results when the scope changes.
        self.cluster.generation = self.cluster.generation.wrapping_add(1);
        self.cluster.output.clear();
        cx.notify();
    }
    fn refresh_cluster(&mut self, query: Query, cx: &mut Context<Self>) {
        if self.cluster.pending {
            return;
        }
        let context = match self.active_ssh_runtime_context("reading cluster resources") {
            Ok(context) => context,
            Err(message) => {
                self.cluster.output = message;
                self.cluster.session_id = self.session.active_id_owned();
                cx.notify();
                return;
            }
        };
        let selection = self.cluster.selection.clone();
        if let Err(message) = nyaterm_cluster_commands::command(query, &selection) {
            self.cluster.output = message.into();
            cx.notify();
            return;
        }
        self.cluster.generation = self.cluster.generation.wrapping_add(1);
        let generation = self.cluster.generation;
        let session_id = context.session_id;
        self.cluster.session_id = Some(session_id.clone());
        self.cluster.query = Some(query);
        self.cluster.output.clear();
        self.cluster.pending = true;
        let task = self
            .blocking_jobs
            .submit_task("cluster-read-only", move |_| {
                nyaterm_remote_ops::cluster::query(
                    context.config,
                    context.multiplex,
                    query,
                    selection,
                )
            });
        cx.spawn(async move |this, cx| {
            let result = await_blocking_job(task).await.and_then(|result| result);
            let _ = this.update(cx, |this, cx| {
                this.cluster.pending = false;
                if this.cluster.generation == generation && this.session.active_id_owned().as_deref() == Some(session_id.as_str()) {
                    this.cluster.output = result.unwrap_or_else(|_| "Query failed. Check SSH access, CLI installation and permissions; timeout is 20 seconds.".into());
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    pub(in crate::features) fn cluster_panel(
        &mut self,
        kubernetes: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let mut body = div()
            .id("cluster-panel")
            .size_full()
            .overflow_y_scrollbar()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .text_color(rgb(palette.text));
        body = body.child("READ ONLY · active SSH host · 20s timeout");
        if kubernetes {
            for (key, label, value) in [
                (
                    "context",
                    "Context (blank = current)",
                    self.cluster.selection.context.clone(),
                ),
                (
                    "namespace",
                    "Namespace (blank = context default)",
                    self.cluster.selection.namespace.clone(),
                ),
                (
                    "pod",
                    "Pod name for logs",
                    self.cluster.selection.pod.clone(),
                ),
            ] {
                let input = self.text_input(
                    format!("cluster.{key}"),
                    &value,
                    TextInputSetup::placeholder(label),
                    cx,
                );
                body = body.child(label).child(
                    div()
                        .w_full()
                        .h(px(32.))
                        .child(NyaInputShell::new(format!("cluster-{key}"), &input)),
                );
            }
        }
        let queries = if kubernetes {
            Query::KUBERNETES.as_slice()
        } else {
            Query::SLURM.as_slice()
        };
        let mut buttons = div().flex().flex_wrap().gap_1();
        for &query in queries {
            buttons = buttons.child(small_button(
                palette,
                format!("cluster-query-{query:?}"),
                query.label(),
                cx.listener(move |this, _, _, cx| this.refresh_cluster(query, cx)),
            ));
        }
        let matches_host = self.cluster.session_id == self.session.active_id_owned();
        let matches_panel = self.cluster.query.is_none_or(|q| queries.contains(&q));
        let text = if self.cluster.pending {
            "Loading…".to_string()
        } else if matches_host && matches_panel && !self.cluster.output.is_empty() {
            self.cluster.output.clone()
        } else {
            "Choose a view to query the active SSH host. Results are not saved. Logs may contain application-sensitive data.".into()
        };
        body.child(buttons).child(div().text_sm().child(text))
    }
    pub(in crate::features) fn workspace_tools_panel(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let mut body = div()
            .id("workspace-tools")
            .size_full()
            .overflow_y_scrollbar()
            .flex()
            .flex_col()
            .p_2()
            .gap_2();
        if panel == NavItem::Layouts {
            for (id, label, direction) in [
                (
                    "horizontal",
                    "Horizontal",
                    WorkspaceSplitDirection::Horizontal,
                ),
                ("vertical", "Vertical", WorkspaceSplitDirection::Vertical),
            ] {
                body = body.child(small_button(
                    palette,
                    id,
                    label,
                    cx.listener(move |this, _, window, cx| {
                        this.split_workspace_with_duplicate(direction, window, cx)
                    }),
                ));
            }
            for (id, label, mode) in [
                ("auto", "Auto Tile All", SmartSplitMode::Auto),
                (
                    "top-bottom",
                    "Tile Top to Bottom",
                    SmartSplitMode::Horizontal,
                ),
                ("left-right", "Tile Left to Right", SmartSplitMode::Vertical),
            ] {
                body = body.child(small_button(
                    palette,
                    id,
                    label,
                    cx.listener(move |this, _, _, cx| this.apply_smart_split(mode, cx)),
                ));
            }
            return body.child(small_button(
                palette,
                "unsplit",
                "Unsplit",
                cx.listener(|this, _, _, cx| this.unsplit_workspace(cx)),
            ));
        }
        let enabled = self.sync_input.input_selection_active();
        body = body.child(
            div()
                .p_2()
                .bg(rgb(if enabled { 0x9b2525 } else { palette.surface }))
                .child(if enabled {
                    "SYNC INPUT ON — commands reach multiple sessions"
                } else {
                    "SYNC INPUT OFF"
                }),
        );
        body = body.child(small_button(
            palette,
            "sync-stop",
            "Turn ALL sync OFF",
            cx.listener(|this, _, _, cx| {
                this.sync_input.disable_all();
                cx.notify();
            }),
        ));
        let broadcast = self.sync_input.broadcast_to_all();
        body = body.child(small_button(
            palette,
            "sync-broadcast",
            if broadcast {
                "All sessions: ON (click to disable)"
            } else {
                "All sessions: OFF (click to enable)"
            },
            cx.listener(|this, _, _, cx| this.toggle_broadcast_to_all(cx)),
        ));
        body = body.child(small_button(
            palette,
            "sync-new",
            "New group (OFF)",
            cx.listener(|this, _, _, cx| this.create_sync_group(cx)),
        ));
        for group in self.sync_input.groups().to_vec() {
            let id = group.id.clone();
            body = body.child(small_button(
                palette,
                format!("sync-select-{id}"),
                format!(
                    "{} · {} · {} sessions",
                    group.name,
                    if group.enabled { "ON" } else { "OFF" },
                    group.session_ids.len()
                ),
                cx.listener(move |this, _, _, cx| this.select_sync_group(id.clone(), cx)),
            ));
        }
        if let Some(group) = self.sync_input.selected_group().cloned() {
            let input = self.text_input(
                format!("sync.group-name.{}", group.id),
                &group.name,
                TextInputSetup::placeholder("Group name"),
                cx,
            );
            body = body.child(
                div()
                    .w_full()
                    .h(px(32.))
                    .child(NyaInputShell::new("sidebar-sync-name", &input)),
            );
            body = body.child(small_button(
                palette,
                "sync-toggle-group",
                if group.enabled {
                    "Selected group: ON"
                } else {
                    "Selected group: OFF"
                },
                cx.listener(|this, _, _, cx| this.toggle_selected_sync_group_enabled(cx)),
            ));
            body = body.child(small_button(
                palette,
                "sync-delete",
                "Delete selected group",
                cx.listener(|this, _, _, cx| {
                    this.sync_input.delete_selected();
                    cx.notify();
                }),
            ));
            for session in self.session.ordered_sessions() {
                let id = session.id.clone();
                let included = group.session_ids.contains(&id);
                body = body.child(small_button(
                    palette,
                    format!("sync-member-{id}"),
                    format!("{} {}", if included { "[x]" } else { "[ ]" }, session.name),
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_session_in_selected_sync_group(id.clone(), cx)
                    }),
                ));
            }
        }
        body.child(small_button(
            palette,
            "sync-full",
            "Open full group manager",
            cx.listener(|this, _, window, cx| this.open_sync_groups(window, cx)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::features::{NyaTermApp, test_support::app_with_visible_local_session};
    use crate::models::NavItem;
    use gpui::{Context, Entity, IntoElement, Render, TestAppContext, Window, div, prelude::*};
    use nyaterm_core::test_support::TestTempDir;
    struct PanelFixture {
        app: Entity<NyaTermApp>,
        panel: NavItem,
    }
    impl Render for PanelFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let panel = self.panel;
            let content = self
                .app
                .update(cx, |app, cx| app.panel_body(panel, window, cx));
            div().size_full().child(content)
        }
    }
    #[test]
    fn all_custom_panels_render_with_sync_off() {
        let root = TestTempDir::new("cluster-panel-render");
        let mut cx = TestAppContext::single();
        let _cleanup = crate::test_support::TestAppCleanup::new(&cx);
        let app = app_with_visible_local_session(&mut cx, root.path(), "cluster-panel-session");
        app.update(&mut cx, |app, cx| {
            app.sync_component_theme(cx);
            assert!(!app.sync_input.input_selection_active());
            app.create_sync_group(cx);
            assert!(!app.sync_input.input_selection_active());
        });
        for panel in [
            NavItem::Kubernetes,
            NavItem::Slurm,
            NavItem::SyncInput,
            NavItem::Layouts,
        ] {
            let app = app.clone();
            let (_, visual) = cx.add_window_view(move |_, _| PanelFixture { app, panel });
            visual.run_until_parked();
            visual.update(|window, cx| {
                let _ = window.draw(cx);
            });
        }
    }
    #[test]
    fn changing_scope_invalidates_pending_result_and_clears_output() {
        let root = TestTempDir::new("cluster-scope");
        let mut cx = TestAppContext::single();
        let _cleanup = crate::test_support::TestAppCleanup::new(&cx);
        let app = app_with_visible_local_session(&mut cx, root.path(), "cluster-scope-session");
        app.update(&mut cx, |app, cx| {
            app.cluster.output = "old host data".into();
            app.cluster.pending = true;
            let generation = app.cluster.generation;
            app.apply_cluster_input("namespace", "team-a".into(), cx);
            assert!(app.cluster.output.is_empty());
            assert_ne!(app.cluster.generation, generation);
            assert!(app.cluster.pending);
        });
    }
}
