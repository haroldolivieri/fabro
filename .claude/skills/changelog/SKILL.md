---
name: changelog
description: Generate and update the product changelog in Mintlify docs. Use when the user asks to update the changelog, add a changelog entry, document recent changes, or write release notes. Reads git history on main, filters to user-facing changes, and writes dated MDX files to docs/changelog/.
---

# Changelog

Generate user-facing changelog entries from git history and write them as Mintlify MDX files.

- [references/guide.md](references/guide.md) — writing conventions, title rules, and anti-patterns
- [references/format.md](references/format.md) — MDX template and formatting rules

## Workflow

### 1. Determine date range

Read filenames in `docs/changelog/` to find the most recent entry date. If no entries exist, the changelog starts from 2025-02-19 (first commit).

### 2. Gather changes

Run `git log --oneline --no-merges main` for commits **on or after** the last entry date (inclusive). This ensures re-running the skill on the same day picks up commits that landed after the previous run. Read commit messages and changed files to understand the actual user-facing impact — don't just reword commit messages.

### 3. Filter and group by date

Group commits by their commit date. Each date that has user-facing changes gets its own entry file. Dates with only internal changes get no entry.

Include only changes visible to end users:
- New features and capabilities
- Bug fixes that affected users
- Breaking changes or behavioral changes
- New integrations or provider support
- Performance improvements users would notice
- UI/UX changes

Exclude:
- Internal refactors with no behavior change
- Test-only changes
- CI/CD pipeline changes
- Dependency bumps (unless they fix a user-facing issue)
- Code style or linting changes

If there are no user-facing changes in the entire range, tell the user and stop.

### 4. Write changelog entries

Create one file per date at `docs/changelog/YYYY-MM-DD.mdx`, using the commit date (not today's date). If a file already exists for a date, regenerate it with the full set of commits for that day (not just new ones). Follow the references linked above for format, writing style, and hero vs. accordion decisions.

- **Batch related commits** into a single feature section (e.g., multiple hook-related commits become one "Lifecycle hooks" section)

### 5. Update docs/docs.json

Add all new pages to the Changelog tab's pages array in `docs/docs.json`. List entries most recent first. The page path is `changelog/YYYY-MM-DD` (no `.mdx` extension).

### 6. Clean up legacy single-file changelog

If `docs/changelog.mdx` still exists as the old single-file changelog, delete it and remove its reference from `docs/docs.json`.
