Fixed in the Django checkout at `/home/daytona/workspace/django`.

Changes:
- `django/db/models/sql/compiler.py`
  - Updated `SQLCompiler.ordering_parts` to use a multiline + dotall regex so duplicate `ORDER BY` detection works with multiline `RawSQL`.
- `tests/expressions/tests.py`
  - Added a regression test covering multiline `RawSQL(...).desc()/asc()` in `order_by()`, including `distinct()`.

Validation:
- `python tests/runtests.py expressions.tests.BasicExpressionsTests.test_order_by_multiline_sql ordering.tests.OrderingTests.test_order_by_f_expression_duplicates`
- `python tests/runtests.py expressions`
- `python tests/runtests.py ordering`

All passed.