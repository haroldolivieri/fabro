The regex confirms:
- Line 33: Hours are captured only when lookahead confirms `\d+:\d+` follows (meaning both MM and ss must be present after HH)
- Line 34: Minutes are optional
- Line 35: Seconds are mandatory

So the correct format notation is `[[HH:]MM:]ss` — seconds are always required, minutes are optional, and hours are optional but require minutes. This matches the fix: `[DD] [[HH:]MM:]ss[.uuuuuu]`.

**Summary of changes:**

1. **`django/db/models/fields/__init__.py`** (line 1590): Changed the DurationField error message format from `[DD] [HH:[MM:]]ss[.uuuuuu]` to `[DD] [[HH:]MM:]ss[.uuuuuu]` to correctly describe that seconds are mandatory, minutes are optional, and hours are optional only when minutes are provided.

2. **`tests/model_fields/test_durationfield.py`** (line 78): Updated the test to match the corrected error message format.