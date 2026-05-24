use std::path::PathBuf;

use agent_thread::MentionUri;
use agent_thread::schema;
use chrono::{DateTime, Utc};
use collections::HashSet;
use gpui::{App, ClipboardItem, Context, Entity, Focusable, Window, prelude::*};
use settings::Settings;
use terminal_view::terminal_panel::TerminalPanel;
use xenomorphic_actions::agent::{
    AddSelectionToThread, ConflictContent, ResolveConflictedFilesWithAgent,
    ResolveConflictsWithAgent, ReviewBranchDiff,
};

use crate::ExpandMessageEditor;
use crate::completion_provider::AgentContextSource;
use crate::{
    AgentDiffPane, ConversationView, CopyThreadToClipboard, Follow,
    LoadThreadFromClipboard, NewThread, OpenAgentDiff, ToggleNewThreadMenu,
};
use crate::{AgentInitialContent, NewAgentThread, NewNativeAgentThreadFromSummary};
use agent_settings::AgentSettings;
use settings::TerminalDockPosition;
use terminal::terminal_settings::TerminalSettings;
use workspace::{CollaboratorId, PathList, Workspace, dock::DockPosition};

pub fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut Workspace, _window, _cx: &mut Context<Workspace>| {
            workspace
                .register_action(|workspace, _: &NewThread, window, cx| {
                    open_new_agent_session_tab(None, None, None, workspace, window, cx);
                })
                .register_action(
                    |workspace, action: &NewNativeAgentThreadFromSummary, window, cx| {
                        let initial_content = AgentInitialContent::ThreadSummary {
                            session_id: action.from_session_id.clone(),
                            title: None,
                        };
                        open_new_agent_session_tab(None, None, Some(initial_content), workspace, window, cx);
                    },
                )
                .register_action(|workspace, _: &ExpandMessageEditor, window, cx| {
                    if let Some(item) = active_agent_session_item(workspace, cx) {
                        let conversation_view = item.read(cx).conversation_view().clone();
                        if let Some(active_thread) = conversation_view.read(cx).root_thread_view() {
                            active_thread.update(cx, |thread, cx| {
                                thread.expand_message_editor(&ExpandMessageEditor, window, cx);
                                thread.focus_handle(cx).focus(window, cx);
                            });
                        }
                        item.focus_handle(cx).focus(window, cx);
                    }
                })
                .register_action(|workspace, _action: &NewAgentThread, window, cx| {
                    open_new_agent_session_tab(None, None, None, workspace, window, cx);
                })
                .register_action(|workspace, _: &Follow, window, cx| {
                    if let Some(item) = active_agent_session_item(workspace, cx) {
                        if let Some(root_thread_view) = item.read(cx).conversation_view().read(cx).root_thread_view() {
                            let agent_thread_id = root_thread_view.read(cx).session_id.0.clone();
                            workspace.follow(CollaboratorId::Agent(agent_thread_id), window, cx);
                        }
                    }
                })
                .register_action(|workspace, _: &OpenAgentDiff, window, cx| {
                    let thread = active_agent_session_item(workspace, cx)
                        .and_then(|item| {
                            item.read(cx).conversation_view().read(cx).root_thread_view()
                                .map(|r| r.read(cx).thread.clone())
                        });

                    if let Some(thread) = thread {
                        AgentDiffPane::deploy_in_workspace(thread, workspace, window, cx);
                    }
                })
                .register_action(|workspace, _: &ToggleNewThreadMenu, window, cx| {
                    open_new_agent_session_tab(None, None, None, workspace, window, cx);
                })
                .register_action(|workspace, _: &CopyThreadToClipboard, window, cx| {
                    if let Some(item) = active_agent_session_item(workspace, cx) {
                        let conversation_view = item.read(cx).conversation_view().clone();
                        if let Some(thread) = conversation_view.read(cx).as_native_thread(cx) {
                            let workspace_handle = workspace.weak_handle();
                            let load_task = thread.read(cx).to_db(cx);
                            cx.spawn_in(window, async move |_this, cx| {
                                let db_thread = load_task.await;
                                let shared_thread = agent::SharedThread::from_db_thread(&db_thread);
                                let thread_data = shared_thread.to_bytes()?;
                                let encoded = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &thread_data);

                                cx.update(|_window, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(encoded));
                                })?;

                                if let Some(workspace) = workspace_handle.upgrade() {
                                    workspace.update(cx, |workspace, cx| {
                                        struct ThreadCopiedToast;
                                        workspace.show_toast(
                                            workspace::Toast::new(
                                                workspace::notifications::NotificationId::unique::<ThreadCopiedToast>(),
                                                "Thread copied to clipboard (base64 encoded)",
                                            )
                                            .autohide(),
                                            cx,
                                        );
                                    });
                                }

                                anyhow::Ok(())
                            })
                            .detach_and_log_err(cx);
                        }
                    }
                })
                .register_action(|workspace, _: &LoadThreadFromClipboard, window, cx| {
                    open_new_agent_session_tab(None, None, None, workspace, window, cx);
                })
                .register_action(|workspace, action: &ReviewBranchDiff, window, cx| {
                    let mention_uri = MentionUri::GitDiff {
                        base_ref: action.base_ref.to_string(),
                    };
                    let diff_uri = mention_uri.to_uri().to_string();

                    let content_blocks = vec![
                        schema::ContentBlock::Text(schema::TextContent::new(
                            "Please review this branch diff carefully. Point out any issues, \
                             potential bugs, or improvement opportunities you find.\n\n"
                                .to_string(),
                        )),
                        schema::ContentBlock::Resource(schema::EmbeddedResource::new(
                            schema::EmbeddedResourceResource::TextResourceContents(
                                schema::TextResourceContents::new(
                                    action.diff_text.to_string(),
                                    diff_uri,
                                ),
                            ),
                        )),
                    ];

                    let initial_content = AgentInitialContent::ContentBlock {
                        blocks: content_blocks,
                        auto_submit: true,
                    };
                    open_new_agent_session_tab(None, None, Some(initial_content), workspace, window, cx);
                })
                .register_action(
                    |workspace, action: &ResolveConflictsWithAgent, window, cx| {
                        let content_blocks = build_conflict_resolution_prompt(&action.conflicts);

                        let initial_content = AgentInitialContent::ContentBlock {
                            blocks: content_blocks,
                            auto_submit: true,
                        };
                        open_new_agent_session_tab(None, None, Some(initial_content), workspace, window, cx);
                    },
                )
                .register_action(
                    |workspace, action: &ResolveConflictedFilesWithAgent, window, cx| {
                        let content_blocks =
                            build_conflicted_files_resolution_prompt(&action.conflicted_file_paths);

                        let initial_content = AgentInitialContent::ContentBlock {
                            blocks: content_blocks,
                            auto_submit: true,
                        };
                        open_new_agent_session_tab(None, None, Some(initial_content), workspace, window, cx);
                    },
                )
                .register_action(
                    |workspace: &mut Workspace, _: &AddSelectionToThread, window, cx| {
                        let active_editor = workspace
                            .active_item(cx)
                            .and_then(|item| item.act_as::<editor::Editor>(cx));
                        let has_editor_selection = active_editor.is_some_and(|editor| {
                            editor.update(cx, |editor, cx| {
                                editor.has_non_empty_selection(&editor.display_snapshot(cx))
                            })
                        });

                        let has_terminal_selection = workspace
                            .active_item(cx)
                            .and_then(|item| item.act_as::<terminal_view::TerminalView>(cx))
                            .is_some_and(|terminal_view| {
                                terminal_view
                                    .read(cx)
                                    .terminal()
                                    .read(cx)
                                    .last_content
                                    .selection_text
                                    .as_ref()
                                    .is_some_and(|text| !text.is_empty())
                            });

                        let has_terminal_panel_selection =
                            workspace.panel::<TerminalPanel>(cx).is_some_and(|panel| {
                                let position = match TerminalSettings::get_global(cx).dock {
                                    TerminalDockPosition::Left => DockPosition::Left,
                                    TerminalDockPosition::Bottom => DockPosition::Bottom,
                                    TerminalDockPosition::Right => DockPosition::Right,
                                };
                                let dock_is_open =
                                    workspace.dock_at_position(position).read(cx).is_open();
                                dock_is_open && !panel.read(cx).terminal_selections(cx).is_empty()
                            });

                        if !has_editor_selection
                            && !has_terminal_selection
                            && !has_terminal_panel_selection
                        {
                            return;
                        }

                        let source = AgentContextSource::from_focused(workspace, window, cx)
                            .or_else(|| AgentContextSource::from_active(workspace, cx));

                        let Some(source) = source else {
                            return;
                        };

                        let Some(selection) = source.read_selection(workspace, true, cx) else {
                            return;
                        };

                        if let Some(item) = active_agent_session_item(workspace, cx) {
                            let conversation_view = item.read(cx).conversation_view().clone();
                            conversation_view.update(cx, |cv, cx| {
                                cv.insert_selection(selection, window, cx);
                            });
                        } else {
                            let conversation_view =
                                crate::thread_finder_provider::create_conversation_view(
                                    None,
                                    None,
                                    None,
                                    None,
                                    workspace,
                                    window,
                                    cx,
                                );
                            let cv_clone = conversation_view.clone();
                            let item = cx.new(|_| {
                                crate::AgentSessionItem::new(
                                    conversation_view,
                                    workspace.weak_handle(),
                                )
                            });
                            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);

                            cx.defer_in(window, move |_this, window, cx| {
                                cv_clone.update(cx, |cv, cx| {
                                    cv.insert_selection(selection, window, cx);
                                });
                            });
                        }
                    },
                );
        },
    )
    .detach();
}

fn conflict_resource_block(conflict: &ConflictContent) -> schema::ContentBlock {
    let mention_uri = MentionUri::MergeConflict {
        file_path: conflict.file_path.clone(),
    };
    schema::ContentBlock::Resource(schema::EmbeddedResource::new(
        schema::EmbeddedResourceResource::TextResourceContents(schema::TextResourceContents::new(
            conflict.conflict_text.clone(),
            mention_uri.to_uri().to_string(),
        )),
    ))
}

pub(crate) fn open_new_agent_session_tab(
    session_id_to_load: Option<schema::SessionId>,
    work_dirs: Option<PathList>,
    initial_content: Option<AgentInitialContent>,
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let conversation_view = crate::thread_finder_provider::create_conversation_view(
        session_id_to_load,
        work_dirs,
        None,
        initial_content,
        workspace,
        window,
        cx,
    );
    let item = cx.new(|_| {
        crate::AgentSessionItem::new(
            conversation_view,
            workspace.weak_handle(),
        )
    });
    workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
}

pub(crate) fn active_agent_session_item(
    workspace: &Workspace,
    cx: &App,
) -> Option<Entity<crate::AgentSessionItem>> {
    workspace
        .active_item(cx)
        .and_then(|item| item.act_as::<crate::AgentSessionItem>(cx))
}

fn build_conflict_resolution_prompt(conflicts: &[ConflictContent]) -> Vec<schema::ContentBlock> {
    if conflicts.is_empty() {
        return Vec::new();
    }

    let mut blocks = Vec::new();

    if conflicts.len() == 1 {
        let conflict = &conflicts[0];

        blocks.push(schema::ContentBlock::Text(schema::TextContent::new(
            "Please resolve the following merge conflict in ",
        )));
        let mention = MentionUri::File {
            abs_path: PathBuf::from(conflict.file_path.clone()),
        };
        blocks.push(schema::ContentBlock::ResourceLink(schema::ResourceLink::new(
            mention.name(),
            mention.to_uri(),
        )));

        blocks.push(schema::ContentBlock::Text(schema::TextContent::new(
            indoc::formatdoc!(
                "\nThe conflict is between branch `{ours}` (ours) and `{theirs}` (theirs).

                Analyze both versions carefully and resolve the conflict by editing \
                the file directly. Choose the resolution that best preserves the intent \
                of both changes, or combine them if appropriate.

                ",
                ours = conflict.ours_branch_name,
                theirs = conflict.theirs_branch_name,
            ),
        )));
    } else {
        let n = conflicts.len();
        let unique_files: HashSet<&str> = conflicts.iter().map(|c| c.file_path.as_str()).collect();
        let ours = &conflicts[0].ours_branch_name;
        let theirs = &conflicts[0].theirs_branch_name;
        blocks.push(schema::ContentBlock::Text(schema::TextContent::new(
            indoc::formatdoc!(
                "Please resolve all {n} merge conflicts below.

                The conflicts are between branch `{ours}` (ours) and `{theirs}` (theirs).

                For each conflict, analyze both versions carefully and resolve them \
                by editing the file{suffix} directly. Choose resolutions that best preserve \
                the intent of both changes, or combine them if appropriate.

                ",
                suffix = if unique_files.len() > 1 { "s" } else { "" },
            ),
        )));
    }

    for conflict in conflicts {
        blocks.push(conflict_resource_block(conflict));
    }

    blocks
}

fn build_conflicted_files_resolution_prompt(
    conflicted_file_paths: &[String],
) -> Vec<schema::ContentBlock> {
    if conflicted_file_paths.is_empty() {
        return Vec::new();
    }

    let instruction = indoc::indoc!(
        "The following files have unresolved merge conflicts. Please open each \
         file, find the conflict markers (`<<<<<<<` / `=======` / `>>>>>>>`), \
         and resolve every conflict by editing the files directly.

         Choose resolutions that best preserve the intent of both changes, \
         or combine them if appropriate.

         Files with conflicts:
         ",
    );

    let mut content = vec![schema::ContentBlock::Text(schema::TextContent::new(instruction))];
    for path in conflicted_file_paths {
        let mention = MentionUri::File {
            abs_path: PathBuf::from(path),
        };
        content.push(schema::ContentBlock::ResourceLink(schema::ResourceLink::new(
            mention.name(),
            mention.to_uri(),
        )));
        content.push(schema::ContentBlock::Text(schema::TextContent::new("\n")));
    }
    content
}

fn format_timestamp_human(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    let relative = if duration.num_seconds() < 0 {
        "in the future".to_string()
    } else if duration.num_seconds() < 60 {
        let seconds = duration.num_seconds();
        format!("{seconds} seconds ago")
    } else if duration.num_minutes() < 60 {
        let minutes = duration.num_minutes();
        format!("{minutes} minutes ago")
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{hours} hours ago")
    } else {
        let days = duration.num_days();
        format!("{days} days ago")
    };

    format!("{} ({})", dt.to_rfc3339(), relative)
}

fn thread_metadata_to_debug_json(
    metadata: &crate::thread_metadata_store::ThreadMetadata,
) -> serde_json::Value {
    serde_json::json!({
        "thread_id": metadata.thread_id,
        "session_id": metadata.session_id.as_ref().map(|s| s.0.to_string()),
        "agent_id": metadata.agent_id.0.to_string(),
        "title": metadata.title.as_ref().map(|t| t.to_string()),
        "updated_at": format_timestamp_human(&metadata.updated_at),
        "created_at": metadata.created_at.as_ref().map(format_timestamp_human),
        "interacted_at": metadata.interacted_at.as_ref().map(format_timestamp_human),
        "worktree_paths": format!("{:?}", metadata.worktree_paths),
        "archived": metadata.archived,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_text_block(blocks: &[schema::ContentBlock], index: usize, expected: &str) {
        let block = &blocks[index];
        match block {
            schema::ContentBlock::Text(text) => {
                assert!(
                    text.text.contains(expected),
                    "expected text block at index {index} to contain {expected:?}, got {:?}",
                    text.text,
                );
            }
            _ => panic!("expected text block at index {index}, got {block:?}"),
        }
    }

    fn expect_resource_block(
        blocks: &[schema::ContentBlock],
        index: usize,
        expected_path: &str,
    ) {
        let block = &blocks[index];
        match block {
            schema::ContentBlock::Resource(res) => {
                match &res.resource {
                    schema::EmbeddedResourceResource::TextResourceContents(text) => {
                        assert!(
                            text.uri.contains(expected_path),
                            "expected resource block URI at index {index} to contain {expected_path:?}, got {:?}",
                            text.uri,
                        );
                    }
                    _ => panic!("expected text resource contents at index {index}"),
                }
            }
            _ => panic!("expected resource block at index {index}, got {block:?}"),
        }
    }

    #[test]
    fn test_build_conflict_resolution_prompt_single_conflict() {
        let conflicts = vec![ConflictContent {
            file_path: "src/main.rs".to_string(),
            ours_branch_name: "feature".to_string(),
            theirs_branch_name: "main".to_string(),
            conflict_text: "<<<<<<<\nours\n=======\ntheirs\n>>>>>>>".to_string(),
        }];

        let blocks = build_conflict_resolution_prompt(&conflicts);

        assert_eq!(blocks.len(), 4);
        expect_text_block(&blocks, 0, "Please resolve the following merge conflict in ");
        expect_resource_block(&blocks, 2, "src/main.rs");
        expect_text_block(&blocks, 3, "between branch `feature` (ours) and `main` (theirs)");
    }

    #[test]
    fn test_build_conflict_resolution_prompt_multiple_conflicts_same_file() {
        let conflicts = vec![
            ConflictContent {
                file_path: "src/main.rs".to_string(),
                ours_branch_name: "feature".to_string(),
                theirs_branch_name: "main".to_string(),
                conflict_text: "<<<<<<<\nours1\n=======\ntheirs1\n>>>>>>>".to_string(),
            },
            ConflictContent {
                file_path: "src/main.rs".to_string(),
                ours_branch_name: "feature".to_string(),
                theirs_branch_name: "main".to_string(),
                conflict_text: "<<<<<<<\nours2\n=======\ntheirs2\n>>>>>>>".to_string(),
            },
        ];

        let blocks = build_conflict_resolution_prompt(&conflicts);

        assert!(blocks.len() >= 3);
        expect_text_block(&blocks, 0, "Please resolve all 2 merge conflicts below");
        expect_resource_block(&blocks, blocks.len() - 2, "src/main.rs");
    }

    #[test]
    fn test_build_conflict_resolution_prompt_multiple_conflicts_different_files() {
        let conflicts = vec![
            ConflictContent {
                file_path: "src/main.rs".to_string(),
                ours_branch_name: "feature".to_string(),
                theirs_branch_name: "main".to_string(),
                conflict_text: "<<<<<<<\nours1\n=======\ntheirs1\n>>>>>>>".to_string(),
            },
            ConflictContent {
                file_path: "src/lib.rs".to_string(),
                ours_branch_name: "feature".to_string(),
                theirs_branch_name: "main".to_string(),
                conflict_text: "<<<<<<<\nours2\n=======\ntheirs2\n>>>>>>>".to_string(),
            },
        ];

        let blocks = build_conflict_resolution_prompt(&conflicts);

        assert!(blocks.len() >= 3);
        expect_text_block(&blocks, 0, "Please resolve all 2 merge conflicts below");
        let last_index = blocks.len() - 1;
        expect_resource_block(&blocks, last_index, "src/lib.rs");
    }

    #[test]
    fn test_build_conflicted_files_resolution_prompt_file_paths_only() {
        let paths = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];

        let blocks = build_conflicted_files_resolution_prompt(&paths);

        assert_eq!(blocks.len(), 5);
        expect_text_block(&blocks, 0, "The following files have unresolved merge conflicts");
        expect_text_block(&blocks, 2, "\n");
        expect_text_block(&blocks, 4, "\n");
    }

    #[test]
    fn test_build_conflict_resolution_prompt_empty_conflicts() {
        let blocks = build_conflict_resolution_prompt(&[]);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_build_conflicted_files_resolution_prompt_empty_paths() {
        let blocks = build_conflicted_files_resolution_prompt(&[]);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_conflict_resource_block_structure() {
        let conflict = ConflictContent {
            file_path: "src/foo.rs".to_string(),
            ours_branch_name: "ours".to_string(),
            theirs_branch_name: "theirs".to_string(),
            conflict_text: "conflict text".to_string(),
        };

        let block = conflict_resource_block(&conflict);

        match block {
            schema::ContentBlock::Resource(res) => {
                match &res.resource {
                    schema::EmbeddedResourceResource::TextResourceContents(text) => {
                        assert!(text.uri.contains("src/foo.rs"));
                        assert_eq!(text.text, "conflict text");
                    }
                    _ => panic!("expected text resource contents"),
                }
            }
            _ => panic!("expected resource block"),
        }
    }
}
