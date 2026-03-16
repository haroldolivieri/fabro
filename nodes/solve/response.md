Implemented the fix in a local Django clone at `/home/daytona/workspace/django` since the provided setup failed trying to clone into the non-empty workspace root.

Changes made:
- `django/core/management/commands/sqlmigrate.py`
  - Changed `self.output_transaction` to require both:
    - `migration.atomic`
    - `connection.features.can_rollback_ddl`
- `tests/migrations/test_commands.py`
  - Added a regression test that mocks `connection.features.can_rollback_ddl = False` for an otherwise atomic migration and asserts `BEGIN/COMMIT` aren’t emitted.

Files changed:
- `/home/daytona/workspace/django/django/core/management/commands/sqlmigrate.py`
- `/home/daytona/workspace/django/tests/migrations/test_commands.py`

Validation:
- Ran:
  - `python tests/runtests.py migrations.test_commands.MigrateTests.test_sqlmigrate_forwards migrations.test_commands.MigrateTests.test_sqlmigrate_for_non_atomic_migration migrations.test_commands.MigrateTests.test_sqlmigrate_for_atomic_migration_without_transactional_ddl`
- Result: all 3 tests passed.