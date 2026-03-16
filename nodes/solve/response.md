All tests pass. Here's a summary of the changes:

## Changes Made

### 1. `django/core/management/commands/sqlmigrate.py` (line 58-60)

Changed the `output_transaction` assignment to also check `connection.features.can_rollback_ddl`:

**Before:**
```python
# Show begin/end around output only for atomic migrations
self.output_transaction = migration.atomic
```

**After:**
```python
# Show begin/end around output for atomic migrations on databases
# that support transactional DDL.
self.output_transaction = migration.atomic and connection.features.can_rollback_ddl
```

This aligns `sqlmigrate`'s behavior with the migration executor, which only wraps migrations in `BEGIN`/`COMMIT` if both the migration is atomic **and** the database supports transactional DDL (`can_rollback_ddl`).

### 2. `tests/migrations/test_commands.py` (after line 617)

Added a new test `test_sqlmigrate_for_non_transactional_databases` that uses `mock.patch.object` to set `connection.features.can_rollback_ddl` to `False`, verifying that transaction wrappers are not shown even for atomic migrations when the database doesn't support transactional DDL. This follows the same pattern as the existing `test_sqlmigrate_for_non_atomic_migration` test, as suggested in the issue.