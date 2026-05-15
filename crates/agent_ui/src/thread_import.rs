use collections::HashSet;
use db::kvp::Dismissable;
use db::sqlez;
use gpui::{App, Context, WeakEntity};
use notifications::status_toast::StatusToast;
use release_channel::ReleaseChannel;
use ui::{Icon, IconName, Color};
use util::ResultExt;
use workspace::Workspace;

use crate::thread_metadata_store::{ThreadId, ThreadMetadata, ThreadMetadataStore};

pub struct CrossChannelImportOnboarding;

impl CrossChannelImportOnboarding {
    pub fn dismissed(cx: &App) -> bool {
        <Self as Dismissable>::dismissed(cx)
    }

    pub fn dismiss(cx: &mut App) {
        <Self as Dismissable>::set_dismissed(true, cx);
    }
}

impl Dismissable for CrossChannelImportOnboarding {
    const KEY: &'static str = "dismissed-cross-channel-thread-import";
}

/// Returns the list of non-Dev, non-current release channels that have
/// at least one thread in their database.  The result is suitable for
/// building a user-facing message ("from Xenomorphic Preview and Nightly").
pub fn channels_with_threads(cx: &App) -> Vec<ReleaseChannel> {
    let Some(current_channel) = ReleaseChannel::try_global(cx) else {
        return Vec::new();
    };
    let database_dir = paths::database_dir();
    ReleaseChannel::ALL
        .iter()
        .filter(|channel| **channel != current_channel && **channel != ReleaseChannel::Dev)
        .filter(|channel| channel_has_threads(database_dir, **channel))
        .copied()
        .collect()
}

pub fn import_threads_from_other_channels(_workspace: &mut Workspace, cx: &mut Context<Workspace>) {
    let database_dir = paths::database_dir().clone();
    import_threads_from_other_channels_in(database_dir, cx);
}

fn import_threads_from_other_channels_in(
    database_dir: std::path::PathBuf,
    cx: &mut Context<Workspace>,
) {
    let current_channel = ReleaseChannel::global(cx);

    let existing_thread_ids: HashSet<ThreadId> = ThreadMetadataStore::global(cx)
        .read(cx)
        .entries()
        .map(|metadata| metadata.thread_id)
        .collect();

    let workspace_handle = cx.weak_entity();
    cx.spawn(async move |_this, cx| {
        let mut imported_threads = Vec::new();

        for channel in &ReleaseChannel::ALL {
            if *channel == current_channel || *channel == ReleaseChannel::Dev {
                continue;
            }

            match read_threads_from_channel(&database_dir, *channel) {
                Ok(threads) => {
                    let new_threads = threads
                        .into_iter()
                        .filter(|thread| !existing_thread_ids.contains(&thread.thread_id));
                    imported_threads.extend(new_threads);
                }
                Err(error) => {
                    log::warn!(
                        "Failed to read threads from {} channel database: {}",
                        channel.dev_name(),
                        error
                    );
                }
            }
        }

        let imported_count = imported_threads.len();

        cx.update(|cx| {
            ThreadMetadataStore::global(cx)
                .update(cx, |store, cx| store.save_all(imported_threads, cx));

            show_cross_channel_import_toast(&workspace_handle, imported_count, cx);
        })
    })
    .detach();
}

fn channel_has_threads(database_dir: &std::path::Path, channel: ReleaseChannel) -> bool {
    let db_path = db::db_path(database_dir, channel);
    if !db_path.exists() {
        return false;
    }
    let connection = sqlez::connection::Connection::open_file(&db_path.to_string_lossy());
    connection
        .select_row::<bool>("SELECT 1 FROM sidebar_threads LIMIT 1")
        .ok()
        .and_then(|mut query| query().ok().flatten())
        .unwrap_or(false)
}

fn read_threads_from_channel(
    database_dir: &std::path::Path,
    channel: ReleaseChannel,
) -> anyhow::Result<Vec<ThreadMetadata>> {
    let db_path = db::db_path(database_dir, channel);
    if !db_path.exists() {
        return Ok(Vec::new());
    }
    let connection = sqlez::connection::Connection::open_file(&db_path.to_string_lossy());
    crate::thread_metadata_store::list_thread_metadata_from_connection(&connection)
}

fn show_cross_channel_import_toast(
    workspace: &WeakEntity<Workspace>,
    imported_count: usize,
    cx: &mut App,
) {
    let status_toast = if imported_count == 0 {
        StatusToast::new("No new threads found to import.", cx, |this, _cx| {
            this.icon(Icon::new(IconName::Info).color(Color::Muted))
                .dismiss_button(true)
        })
    } else {
        let message = if imported_count == 1 {
            "Imported 1 thread from other channels.".to_string()
        } else {
            format!("Imported {imported_count} threads from other channels.")
        };
        StatusToast::new(message, cx, |this, _cx| {
            this.icon(Icon::new(IconName::Check).color(Color::Success))
                .dismiss_button(true)
        })
    };

    workspace
        .update(cx, |workspace, cx| {
            workspace.toggle_status_toast(status_toast, cx);
        })
        .log_err();
}
