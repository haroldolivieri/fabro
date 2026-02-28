use std::path::Path;
use std::process::Command;

use crate::error::{AttractorError, Result};

fn git_error(msg: impl Into<String>) -> AttractorError {
    AttractorError::Engine(msg.into())
}

/// Assert the working directory is a clean git repo (no uncommitted changes).
pub fn ensure_clean(repo: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .map_err(|e| git_error(format!("git status failed: {e}")))?;

    if !output.status.success() {
        return Err(git_error("not a git repository"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        return Err(git_error("working directory has uncommitted changes"));
    }

    Ok(())
}

/// Return the SHA of HEAD.
pub fn head_sha(repo: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .map_err(|e| git_error(format!("git rev-parse failed: {e}")))?;

    if !output.status.success() {
        return Err(git_error("git rev-parse HEAD failed"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Create a new branch at HEAD without checking it out.
pub fn create_branch(repo: &Path, name: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["branch", name, "HEAD"])
        .current_dir(repo)
        .output()
        .map_err(|e| git_error(format!("git branch failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git_error(format!("git branch failed: {stderr}")));
    }

    Ok(())
}

/// Add a git worktree for the given branch at `path`.
pub fn add_worktree(repo: &Path, path: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "add"])
        .arg(path)
        .arg(branch)
        .current_dir(repo)
        .output()
        .map_err(|e| git_error(format!("git worktree add failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git_error(format!("git worktree add failed: {stderr}")));
    }

    Ok(())
}

/// Remove a git worktree.
pub fn remove_worktree(repo: &Path, path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(path)
        .current_dir(repo)
        .output()
        .map_err(|e| git_error(format!("git worktree remove failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git_error(format!("git worktree remove failed: {stderr}")));
    }

    Ok(())
}

/// Stage all changes and commit in `work_dir` with a structured message.
/// Returns the new commit SHA.
pub fn checkpoint_commit(
    work_dir: &Path,
    run_id: &str,
    node_id: &str,
    status: &str,
) -> Result<String> {
    // Stage everything
    let output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(work_dir)
        .output()
        .map_err(|e| git_error(format!("git add failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git_error(format!("git add failed: {stderr}")));
    }

    // Commit with arc identity (works even if user.name/email not configured)
    let message = format!("arc({run_id}): {node_id} ({status})");
    let output = Command::new("git")
        .args([
            "-c", "user.name=arc",
            "-c", "user.email=arc@local",
            "commit",
            "--allow-empty",
            "-m", &message,
        ])
        .current_dir(work_dir)
        .output()
        .map_err(|e| git_error(format!("git commit failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git_error(format!("git commit failed: {stderr}")));
    }

    head_sha(work_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary git repo with an initial commit.
    fn init_repo(dir: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@test", "commit", "--allow-empty", "-m", "init"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn ensure_clean_on_clean_repo() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        assert!(ensure_clean(dir.path()).is_ok());
    }

    #[test]
    fn ensure_clean_fails_with_dirty_file() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        fs::write(dir.path().join("dirty.txt"), "hello").unwrap();
        let err = ensure_clean(dir.path()).unwrap_err();
        assert!(err.to_string().contains("uncommitted changes"));
    }

    #[test]
    fn ensure_clean_fails_on_non_repo() {
        let dir = tempfile::tempdir().unwrap();
        let err = ensure_clean(dir.path()).unwrap_err();
        assert!(err.to_string().contains("not a git repository"));
    }

    #[test]
    fn head_sha_returns_40_char_hex() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        let sha = head_sha(dir.path()).unwrap();
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn create_branch_and_list() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        create_branch(dir.path(), "test-branch").unwrap();

        let output = Command::new("git")
            .args(["branch", "--list", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("test-branch"));
    }

    #[test]
    fn add_and_remove_worktree() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        create_branch(dir.path(), "wt-branch").unwrap();

        let wt_path = dir.path().join("my-worktree");
        add_worktree(dir.path(), &wt_path, "wt-branch").unwrap();
        assert!(wt_path.join(".git").exists());

        remove_worktree(dir.path(), &wt_path).unwrap();
        assert!(!wt_path.exists());
    }

    #[test]
    fn checkpoint_commit_creates_commit() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        create_branch(dir.path(), "run-branch").unwrap();

        let wt_path = dir.path().join("worktree");
        add_worktree(dir.path(), &wt_path, "run-branch").unwrap();

        // Write a file in the worktree
        fs::write(wt_path.join("output.txt"), "result").unwrap();

        let sha = checkpoint_commit(&wt_path, "run1", "nodeA", "success").unwrap();
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));

        // Verify commit message
        let output = Command::new("git")
            .args(["log", "--oneline", "-1"])
            .current_dir(&wt_path)
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&output.stdout);
        assert!(log.contains("arc(run1): nodeA (success)"));

        remove_worktree(dir.path(), &wt_path).unwrap();
    }

    #[test]
    fn checkpoint_commit_with_no_user_config() {
        let dir = tempfile::tempdir().unwrap();
        // Init repo without setting global user.name/email — the -c flags on commit
        // provide identity, so this should still succeed.
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@test", "commit", "--allow-empty", "-m", "init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        create_branch(dir.path(), "fallback-branch").unwrap();

        let wt_path = dir.path().join("worktree");
        add_worktree(dir.path(), &wt_path, "fallback-branch").unwrap();

        let sha = checkpoint_commit(&wt_path, "run2", "nodeB", "completed").unwrap();
        assert_eq!(sha.len(), 40);

        remove_worktree(dir.path(), &wt_path).unwrap();
    }
}
