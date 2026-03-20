Now I have all the data needed. Here's the full triage summary:

---

## 📋 Triage Summary — fabro-sh/fabro
**Date:** 2026-03-20

---

### 📊 Totals

| Category | Count |
|---|---|
| Open Pull Requests | **0** |
| Open Issues | **11** |

---

### 🔀 Open Pull Requests

**None.** No open PRs at this time. The repo appears to operate with a direct-to-main or fast-merge workflow — the 10 most recent commits in git log all landed directly (no PR merge commits visible).

---

### 🐛 Open Issues (11 total)

| # | Title | Author | Age | Labels | Assigned |
|---|---|---|---|---|---|
| [#126](https://github.com/fabro-sh/fabro/issues/126) | Use short hex IDs for subagents instead of UUIDs | `brynary` | <1h | ❌ none | ❌ none |
| [#125](https://github.com/fabro-sh/fabro/issues/125) | Fix stall watchdog false positives on active handlers | `brynary` | ~2h | ❌ none | ❌ none |
| [#124](https://github.com/fabro-sh/fabro/issues/124) | Externalize LLM prompts as MiniJinja .j2 templates | `brynary` | ~2h | ❌ none | ❌ none |
| [#123](https://github.com/fabro-sh/fabro/issues/123) | Publish Docker agent image to GHCR | `brynary` | ~2h | ❌ none | ❌ none |
| [#121](https://github.com/fabro-sh/fabro/issues/121) | Refactor fabro-model: OOP redesign with Catalog, LanguageModel trait, ModelRef | `brynary` | ~2h | ❌ none | ❌ none |
| [#120](https://github.com/fabro-sh/fabro/issues/120) | Support Anthropic fast mode (`speed: fast`) | `brynary` | ~3h | ❌ none | ❌ none |
| [#119](https://github.com/fabro-sh/fabro/issues/119) | Linux arm64 binary? | `mikegehard` | ~9h | ❌ none | ❌ none |
| [#118](https://github.com/fabro-sh/fabro/issues/118) | Stall watchdog kills review_fanout after 1800s despite branches running | `natea` | ~20h | ❌ none | ❌ none |
| [#107](https://github.com/fabro-sh/fabro/issues/107) | Cannot pull fabro-agent Docker image: 404 / pull access denied | `natea` | ~22h | ❌ none | ❌ none |
| [#98](https://github.com/fabro-sh/fabro/issues/98) | Installed Fabro cannot launch the documented server/web UI workflow | `zakelfassi` | 1 day | ❌ none | ❌ none |
| [#93](https://github.com/fabro-sh/fabro/issues/93) | ACP protocol for CLI agents | `zharinov` | 2 days | ❌ none | ❌ none |

---

### ⏱️ Stale PRs (>7 days, no review activity)

**None** — no open PRs.

---

### ⚠️ Issues Needing Attention

#### 🔴 All issues are unassigned and unlabeled
Every single open issue has **no labels** and **no assignee**. This makes prioritisation, filtering, and routing impossible. Labelling is the highest-leverage triage action right now.

#### 🔗 Linked Issue Clusters

**Docker / GHCR cluster (2 issues):**
- **#107** — Community bug: `fabro-agent:latest` returns a 404 on Docker Hub; users can't run any workflow using the Docker sandbox.
- **#123** — Maintainer-filed: Publish the Docker agent image to GHCR. This is the direct fix for #107.
- **Action:** Link #107 → #123. Once #123 ships, close #107 with a release note.

**Stall watchdog cluster (2 issues):**
- **#118** — Community bug: Stall watchdog fires after 1800s on a `review_fanout` fan-out node even though child branches were actively running. `brynary` has responded and asked for `progress.jsonl`.
- **#125** — Maintainer-filed: Fix stall watchdog false positives on active handlers. This appears to be the tracking issue for the fix.
- **Action:** Link #118 → #125. Close #118 when #125 ships.

---

### 🗂️ Issue Classification

| Category | Issues |
|---|---|
| 🐞 **Bugs (community-reported)** | #118 (stall watchdog), #107 (Docker 404), #119 (arm64 binary question) |
| 📐 **UX / Docs / Product clarity** | #98 (server/web UI launch story) |
| 🚀 **Feature requests (community)** | #93 (ACP protocol), #119 (arm64 binary) |
| 🛠️ **Planned work (maintainer-filed)** | #120, #121, #123, #124, #125, #126 |

---

### ✅ Recommended Actions

1. **Add labels** to all 11 issues — at minimum `bug`, `enhancement`, `question`, `documentation`. Zero labels is the biggest triage gap.
2. **Assign owners** — especially to the 3 user-reported bugs (#107, #118, #98) so reporters know someone is on it.
3. **Link related issues** — #107 ↔ #123 and #118 ↔ #125 should reference each other explicitly.
4. **Respond to #98** (`zakelfassi`) and **#93** (`zharinov`) — both are community members who have invested thought and are waiting for a product signal. `zharinov` is even volunteering to implement ACP support.
5. **Close #119** once the arm64 binary (added in recent commit `bbfea71b`) is confirmed published to releases — it may already be resolved.