Goal: Check open PRs and issues using the gh CLI and produce a triage summary


Use the gh CLI to check the current repo for open pull requests and issues. Run these commands:

1. gh pr list --state open --limit 20
2. gh issue list --state open --limit 20

Then produce a triage summary that includes:
- Total count of open PRs and issues
- For each PR: number, title, author, age, review status
- For each issue: number, title, labels, age
- Any PRs that look stale (older than 7 days with no review activity)
- Any issues that are unassigned or unlabeled