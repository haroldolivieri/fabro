use std::path::PathBuf;

use anyhow::Result;
use fabro_config::RuntimeDirectory;
use fabro_server::bind::BindRequest;
use fabro_server::daemon::ServerDaemon;
use fabro_server::serve;
use fabro_server::serve::ServeArgs;
use fabro_util::printer::Printer;
use fabro_util::terminal::Styles;

pub(crate) async fn execute(
    mut serve_args: ServeArgs,
    bind: BindRequest,
    storage_dir: PathBuf,
    styles: &'static Styles,
    printer: Printer,
) -> Result<()> {
    let _ = printer;
    serve_args.bind = Some(bind.to_string());

    let runtime_directory = RuntimeDirectory::new(&storage_dir);
    let _record_guard = scopeguard::guard(runtime_directory.clone(), |dir| {
        ServerDaemon::remove(&dir);
    });

    let _socket_guard = if let BindRequest::Unix(ref path) = bind {
        let path = path.clone();
        Some(scopeguard::guard(path, |p| {
            let _ = std::fs::remove_file(p);
        }))
    } else {
        None
    };

    let log_path = runtime_directory.log_path();
    let pid = std::process::id();
    let daemon_dir = runtime_directory.clone();

    Box::pin(serve::serve_command(
        serve_args,
        styles,
        Some(storage_dir),
        move |resolved_bind| {
            ServerDaemon::new(pid, resolved_bind.clone(), log_path.clone()).write(&daemon_dir)
        },
    ))
    .await
}
