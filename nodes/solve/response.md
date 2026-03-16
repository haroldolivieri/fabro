The fix is a single-line change on line 35 of `/home/daytona/django-repo/django/db/models/sql/compiler.py`:

**Before:**
```python
self.ordering_parts = re.compile(r'(.*)\s(ASC|DESC)(.*)')
```

**After:**
```python
self.ordering_parts = re.compile(r'(.*)\s(ASC|DESC)(.*)', re.MULTILINE | re.DOTALL)
```

**Why this fixes the issue:** The `ordering_parts` regex is used to strip the `ASC`/`DESC` suffix from ORDER BY clauses so that duplicate detection works correctly. Without `re.DOTALL`, the `.*` in the regex doesn't match newline characters, so for multiline RawSQL expressions, `group(1)` only captures the last line before `ASC`/`DESC` — not the entire expression. This causes different multiline RawSQL expressions that happen to share the same last line (e.g., `else null end`) to be incorrectly identified as duplicates, silently dropping ORDER BY clauses. With `re.DOTALL`, `.*` matches across newlines, so the full SQL expression is captured and compared, correctly distinguishing different expressions.