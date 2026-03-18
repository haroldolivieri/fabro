use std::path::Path;

use fabro_util::terminal::Styles;
use fabro_validate::{Diagnostic, Severity};

pub fn read_workflow_file(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {e}", path.display()))
}

pub fn print_diagnostics(diagnostics: &[Diagnostic], styles: &Styles) {
    for d in diagnostics {
        let location = match (&d.node_id, &d.edge) {
            (Some(node), _) => format!(" [node: {node}]"),
            (_, Some((from, to))) => format!(" [edge: {from} -> {to}]"),
            _ => String::new(),
        };
        match d.severity {
            Severity::Error => eprintln!(
                "{}{location}: {} ({})",
                styles.red.apply_to("error"),
                d.message,
                styles.dim.apply_to(&d.rule),
            ),
            Severity::Warning => eprintln!(
                "{}{location}: {} ({})",
                styles.yellow.apply_to("warning"),
                d.message,
                styles.dim.apply_to(&d.rule),
            ),
            Severity::Info => eprintln!(
                "{}",
                styles
                    .dim
                    .apply_to(format!("info{location}: {} ({})", d.message, d.rule)),
            ),
        }
    }
}

pub fn relative_path(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = path.strip_prefix(&cwd) {
            return rel.display().to_string();
        }
    }
    tilde_path(path)
}

pub fn format_tokens_human(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}m", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1000 {
        format!("{:.1}k", tokens as f64 / 1000.0)
    } else {
        tokens.to_string()
    }
}

pub fn tilde_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(suffix) = path.strip_prefix(&home) {
            return format!("~/{}", suffix.display());
        }
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::format_tokens_human;

    #[test]
    fn format_tokens_human_zero() {
        assert_eq!(format_tokens_human(0), "0");
    }

    #[test]
    fn format_tokens_human_small() {
        assert_eq!(format_tokens_human(999), "999");
    }

    #[test]
    fn format_tokens_human_thousands() {
        assert_eq!(format_tokens_human(1000), "1.0k");
    }

    #[test]
    fn format_tokens_human_mid_thousands() {
        assert_eq!(format_tokens_human(15234), "15.2k");
    }

    #[test]
    fn format_tokens_human_millions() {
        assert_eq!(format_tokens_human(1_000_000), "1.0m");
    }

    #[test]
    fn format_tokens_human_mid_millions() {
        assert_eq!(format_tokens_human(3_456_789), "3.5m");
    }
}
