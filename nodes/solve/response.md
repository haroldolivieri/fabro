The fix is complete. Here's a summary of the changes:

## Changes Made

### 1. `django/conf/__init__.py` (main fix)

**Added imports:**
- `ValidationError` from `django.core.exceptions`
- `URLValidator` from `django.core.validators`

**Added `_add_script_prefix()` static method to `LazySettings`:**
- Uses `URLValidator` to detect full URLs (like `http://example.com/static/`) — these are returned unchanged
- Checks if the value starts with `/` (absolute path) — these are returned unchanged
- For relative paths (like `static/`), prepends `get_script_prefix()` (which contains the `SCRIPT_NAME` value)

**Added `STATIC_URL` and `MEDIA_URL` properties to `LazySettings`:**
- These intercept access to `settings.STATIC_URL` and `settings.MEDIA_URL`
- They call `_add_script_prefix()` on the raw setting value to prepend the script prefix when appropriate

### 2. `tests/file_storage/tests.py` (test fix)

Changed `'MEDIA_URL': 'overridden_media_url/'` to `'MEDIA_URL': '/overridden_media_url/'` so the test uses an absolute path that won't be affected by script prefix logic.

### How it works

| SCRIPT_NAME | Setting Value | Result |
|---|---|---|
| `/sub/path/` | `http://example.com/static/` | `http://example.com/static/` (full URL, unchanged) |
| `/sub/path/` | `/static/` | `/static/` (absolute path, unchanged) |
| `/sub/path/` | `static/` | `/sub/path/static/` (relative path, prefixed) |
| (none) | `static/` | `/static/` (default prefix is `/`) |

This makes `{% static %}` and media URL generation consistent with how `{% url %}` and `reverse()` already handle `SCRIPT_NAME`.