All 18 tests pass. Here's a summary of the changes made:

## Summary

The issue is that Django's `models.E028` check unconditionally raises an `Error` when multiple models share the same `db_table`, even when `DATABASE_ROUTERS` is configured — which is a legitimate setup where different models with the same table name can exist in different databases.

### Changes

**1. `django/core/checks/model_checks.py`** — The core fix:
- Added imports for `settings` and `Warning`
- When `settings.DATABASE_ROUTERS` is non-empty, the duplicate `db_table` check now emits a `Warning` with id `models.W035` instead of an `Error` with id `models.E028`
- When there are no database routers, the behavior remains unchanged (still an `Error` with `models.E028`)

**2. `tests/check_framework/test_model_checks.py`** — Added tests:
- `test_collision_in_same_app_database_routers` — verifies Warning is emitted (not Error) when DATABASE_ROUTERS is set
- `test_collision_across_apps_database_routers` — same for cross-app collisions

**3. `tests/check_framework/routers.py`** — New minimal test router class needed by the test overrides.