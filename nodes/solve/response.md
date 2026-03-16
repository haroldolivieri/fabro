## Summary

I have successfully fixed the Django DurationField error message format issue. Here's what was done:

### **Problem**
The error message for invalid DurationField values incorrectly stated the format as:
```
[DD] [HH:[MM:]]ss[.uuuuuu]
```

But the actual parser allows:
- Seconds are **mandatory**
- Minutes are **optional**
- Hours are **optional but only allowed with minutes**

### **Solution**
Updated the format specification to the correct:
```
[DD] [[HH:]MM:]ss[.uuuuuu]
```

### **Files Modified** (2 files, 1 line each)

1. **`django/db/models/fields/__init__.py`** (line 1590)
   - Updated the `default_error_messages` dictionary for the DurationField class

2. **`tests/model_fields/test_durationfield.py`** (line 78)
   - Updated the test assertion to match the corrected error message

### **Verification**
The fix was validated by:
- Analyzing the actual regex pattern in `django/utils/dateparse.py`
- Testing valid duration formats: `14`, `14:00`, `1:14:00`, `1 1:14:00`, etc.
- All formats parse correctly with the corrected format specification

The changes are minimal, focused, and directly address the issue without altering any code behavior—only the user-facing error message text is corrected.