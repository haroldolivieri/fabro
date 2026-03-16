Implemented the minimal fix for the DurationField error message.

Changes:
- Updated `django/db/models/fields/__init__.py`
  - Changed the invalid format message from:
    - `[DD] [HH:[MM:]]ss[.uuuuuu]`
  - to:
    - `[DD] [[HH:]MM:]ss[.uuuuuu]`
- Updated the matching assertion in:
  - `tests/model_fields/test_durationfield.py`

Verified with:
- `python tests/runtests.py model_fields.test_durationfield utils_tests.test_dateparse forms_tests.field_tests.test_durationfield`

All targeted tests passed.