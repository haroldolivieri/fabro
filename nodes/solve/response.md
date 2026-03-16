Implemented the fix in the Django checkout at:

- `.task/django/django/core/checks/model_checks.py`
- `.task/django/tests/check_framework/test_model_checks.py`

What changed:
- Kept `models.E028` for duplicate `db_table` names by default.
- If `settings.DATABASE_ROUTERS` is non-empty, the check now emits a warning instead:
  - `models.W035`
  - with a hint telling users to verify routing to separate databases.

This matches the issue’s requested behavior and keeps the change minimal.

Validation:
- Installed the checkout: `python -m pip install -e .`
- Ran targeted tests successfully:
  - `python tests/runtests.py check_framework.test_model_checks`

Result:
- 18 tests passed.