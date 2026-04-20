use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use fabro_server::bind::Bind;
use fabro_util::printer::Printer;
use tokio::time;

use super::record;

pub(crate) async fn stop_server(storage_dir: &Path, timeout: Duration) -> Result<bool> {
    let Some(active) = record::active_server_record_details(storage_dir)? else {
        return Ok(false);
    };
    let record = active.record;

    fabro_proc::sigterm(record.pid);

    // Use the zombie-aware predicate here: this loop is commonly driven
    // against a child of the calling process (tests, install/uninstall
    // in-process shutdowns, a foreground-launching shell). A zombie
    // child would otherwise satisfy `process_running` until its parent
    // waits, causing us to burn the whole `timeout` on an already-dead
    // process. The `ps` cost (~2 ms per poll) is trivial compared to
    // the 10 s timeout and is only paid while the process still exists.
    let poll_interval = Duration::from_millis(100);
    let mut elapsed = Duration::ZERO;
    while elapsed < timeout {
        if !fabro_proc::process_running_strict(record.pid) {
            break;
        }
        time::sleep(poll_interval).await;
        elapsed += poll_interval;
    }

    if fabro_proc::process_running_strict(record.pid) {
        fabro_proc::sigkill(record.pid);
        time::sleep(Duration::from_millis(100)).await;
    }

    record::remove_server_record(&active.record_path);

    if let Bind::Unix(ref path) = record.bind {
        let _ = std::fs::remove_file(path);
    }

    Ok(true)
}

pub(crate) async fn execute(storage_dir: &Path, timeout: Duration, printer: Printer) -> Result<()> {
    if !stop_server(storage_dir, timeout).await? {
        fabro_util::printerr!(printer, "Server is not running");
        std::process::exit(1);
    }

    fabro_util::printerr!(printer, "Server stopped");
    Ok(())
}
