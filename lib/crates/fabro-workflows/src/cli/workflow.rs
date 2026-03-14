use anyhow::bail;
use clap::Args;

use super::project_config::{
    discover_project_config, list_available_workflows, resolve_fabro_root,
};

#[derive(Args)]
pub struct WorkflowListArgs {}

pub fn workflow_list_command(_args: &WorkflowListArgs) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;

    let (config_path, config) = match discover_project_config(&cwd)? {
        Some(found) => found,
        None => bail!(
            "No fabro.toml found in {cwd} or any parent directory",
            cwd = cwd.display()
        ),
    };

    let fabro_root = resolve_fabro_root(&config_path, &config);
    let project_wf_dir = fabro_root.join("workflows");
    let user_wf_dir = dirs::home_dir().map(|h| h.join(".fabro").join("workflows"));

    let workflows = list_available_workflows(Some(&project_wf_dir), user_wf_dir.as_deref());

    if workflows.is_empty() {
        eprintln!("No workflows found");
    } else {
        for name in &workflows {
            println!("{name}");
        }
    }

    Ok(())
}
