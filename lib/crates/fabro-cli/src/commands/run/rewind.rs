use anyhow::Result;
use cli_table::format::{Border, Separator};
use cli_table::{Cell, CellStruct, Color, Style, Table};
use fabro_api::types::{RewindRequest, TimelineEntryResponse};
use fabro_types::RunId;
use fabro_util::printer::Printer;
use fabro_util::terminal::Styles;
use git2::Repository;
use serde::Serialize;

use crate::args::RewindArgs;
use crate::command_context::CommandContext;
use crate::server_client::Client;
use crate::shared::repo::ensure_matching_repo_origin;
use crate::shared::{color_if, print_json_pretty};

#[derive(Serialize)]
pub(crate) struct TimelineEntryJson {
    ordinal:        usize,
    node_name:      String,
    visit:          usize,
    run_commit_sha: Option<String>,
}

pub(crate) async fn run(
    args: &RewindArgs,
    styles: &Styles,
    base_ctx: &CommandContext,
) -> Result<()> {
    let printer = base_ctx.printer();
    let ctx = base_ctx.with_target(&args.server)?;
    let client = ctx.server().await?;
    let run_id = client.resolve_run(&args.run_id).await?.run_id;
    ensure_origin_if_local(client.as_ref(), &run_id, "rewind").await?;

    if args.list || args.target.is_none() {
        let timeline = client.run_timeline(&run_id).await?;
        if ctx.json_output() {
            print_json_pretty(&timeline_entries_json(&timeline))?;
            return Ok(());
        }
        print_timeline(&timeline_entries_json(&timeline), styles, printer);
        return Ok(());
    }

    let target = args
        .target
        .clone()
        .expect("rewind target should be present unless listing");
    let result = client
        .rewind_run(&run_id, RewindRequest {
            target: Some(target),
            push:   Some(!args.no_push),
        })
        .await?;
    let response = result.response;

    if ctx.json_output() {
        print_json_pretty(&serde_json::json!({
            "source_run_id": response.source_run_id,
            "new_run_id": response.new_run_id,
            "target": response.target,
            "archived": response.archived,
            "archive_error": response.archive_error,
            "status": result.status,
        }))?;
    } else {
        fabro_util::printerr!(
            printer,
            "\nRewound {}; new run {}",
            short_id(&response.source_run_id),
            short_id(&response.new_run_id)
        );
        fabro_util::printerr!(
            printer,
            "To resume: fabro resume {}",
            short_id(&response.new_run_id)
        );
        if !response.archived {
            let archive_error = response.archive_error.as_deref().unwrap_or("unknown error");
            fabro_util::printerr!(
                printer,
                "Warning: source not archived: {archive_error}. Run `fabro archive {}` to finish.",
                short_id(&response.source_run_id)
            );
        }
    }

    Ok(())
}

pub(crate) async fn ensure_origin_if_local(
    client: &Client,
    run_id: &RunId,
    verb: &str,
) -> Result<()> {
    if Repository::discover(".").is_err() {
        return Ok(());
    }

    let state = client.get_run_state(run_id).await?;
    if let Some(run_spec) = state.spec {
        ensure_matching_repo_origin(run_spec.repo_origin_url.as_deref(), verb)?;
    }
    Ok(())
}

pub(crate) fn timeline_entries_json(entries: &[TimelineEntryResponse]) -> Vec<TimelineEntryJson> {
    entries
        .iter()
        .map(|entry| TimelineEntryJson {
            ordinal:        usize::try_from(entry.ordinal.get())
                .expect("timeline ordinal should fit in usize"),
            node_name:      entry.node_name.clone(),
            visit:          usize::try_from(entry.visit.get())
                .expect("timeline visit should fit in usize"),
            run_commit_sha: entry.run_commit_sha.clone(),
        })
        .collect()
}

pub(crate) fn short_id(run_id: &str) -> &str {
    &run_id[..8.min(run_id.len())]
}

pub(crate) fn print_timeline(entries: &[TimelineEntryJson], styles: &Styles, printer: Printer) {
    if entries.is_empty() {
        fabro_util::printerr!(printer, "No checkpoints found.");
        return;
    }

    let use_color = styles.use_color;
    let title = vec![
        "@".cell().bold(use_color),
        "Node".cell().bold(use_color),
        "Details".cell().bold(use_color),
    ];

    let rows: Vec<Vec<CellStruct>> = entries
        .iter()
        .map(|entry| {
            let ordinal_str = format!("@{}", entry.ordinal);
            let mut details = Vec::new();
            if entry.visit > 1 {
                details.push(format!("visit {}, loop", entry.visit));
            }
            if entry.run_commit_sha.is_none() {
                details.push("no run commit".to_string());
            }

            let detail_str = if details.is_empty() {
                String::new()
            } else {
                format!("({})", details.join(", "))
            };

            vec![
                ordinal_str
                    .cell()
                    .foreground_color(color_if(use_color, Color::Cyan)),
                entry.node_name.clone().cell(),
                detail_str
                    .cell()
                    .foreground_color(color_if(use_color, Color::Ansi256(8))),
            ]
        })
        .collect();

    let color_choice = if use_color {
        cli_table::ColorChoice::Auto
    } else {
        cli_table::ColorChoice::Never
    };
    let table = rows
        .table()
        .title(title)
        .color_choice(color_choice)
        .border(Border::builder().build())
        .separator(Separator::builder().build());
    #[allow(
        clippy::print_stderr,
        reason = "The rewind preview table is operator feedback, not command output."
    )]
    if let Ok(display) = table.display() {
        eprintln!("{display}");
    }
}
