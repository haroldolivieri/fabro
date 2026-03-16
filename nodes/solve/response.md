Implemented in a Django checkout at `/home/daytona/workspace/django`.

What changed
- `django/conf/__init__.py`
  - Added `LazySettings._add_script_prefix()`.
  - Made `settings.STATIC_URL` and `settings.MEDIA_URL` prepend the current `SCRIPT_NAME` only for relative paths.
  - Left absolute paths (`/static/`) and full URLs unchanged for compatibility.
- `tests/settings_tests/tests.py`
  - Added regression tests covering:
    - relative `STATIC_URL` / `MEDIA_URL` getting the script prefix,
    - absolute paths and full URLs not being prefixed,
    - blank values and invalid URLs.
- `tests/file_storage/tests.py`
  - Updated one expectation affected by the new default-prefix behavior for relative `MEDIA_URL`.

Validation
- `python3 tests/runtests.py settings_tests.tests.MediaURLStaticURLPrefixTest file_storage.tests.FileStorageTests.test_setting_changed`
- `python3 tests/runtests.py template_tests.syntax_tests.test_static file_storage.tests.FileStorageTests`
- `python3 tests/runtests.py staticfiles_tests.test_storage`

All passed.