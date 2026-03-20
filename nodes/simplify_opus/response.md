Clean. Here's a summary of what was fixed:

## Summary

**Issue found and fixed: Copy-paste duplication between `StylesheetModelKnownRule` and `NodeModelKnownRule`**

The new `NodeModelKnownRule` had nearly identical model and provider validation logic copied from `StylesheetModelKnownRule`, with only the context string and `node_id` differing.

**Fix:** Extracted two shared helper functions in `rules.rs`:
- `check_model_known(rule_name, model, context, node_id) -> Option<Diagnostic>` — validates a model name against the catalog
- `check_provider_known(rule_name, provider, context, node_id) -> Option<Diagnostic>` — validates a provider name against `Provider::ALL`

Both `StylesheetModelKnownRule` and `NodeModelKnownRule` now call these helpers, passing their respective context strings (`"in stylesheet rule '#foo'"` vs `"on node 'bar'"`). This eliminates ~30 lines of duplicated logic.

**Other findings (no action needed):**
- **Missing trailing newline in catalog.json**: Already resolved (file has a trailing newline).
- **Efficiency**: No issues — validation runs on a cold CLI path, all operations are proportional to graph size.
- **Diagnostic builder pattern**: Pre-existing pattern across all ~20 rules, out of scope for this change.