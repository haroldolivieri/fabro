# Django Issue #30479 Resolution Summary

## Overview
This document summarizes the resolution of Django GitHub Issue #30479: "Autoreloader with StatReloader doesn't track changes in manage.py."

## Issue Details
- **Repository**: https://github.com/django/django.git
- **Issue Number**: 30479
- **Affected Versions**: Django 2.2.1 (regression from 2.1.8)
- **Severity**: Medium (impacts development experience)
- **Regression Commit**: `c8720e7696ca41f3262d5369365cc1bd72a216ca`

## Root Cause Analysis

The Django autoreloader uses the `iter_modules_and_files()` function in `django/utils/autoreload.py` to determine which files to watch for changes. The function iterates through `sys.modules` and checks each module's `__spec__` attribute.

When Django's `manage.py` is executed:
1. It becomes the `__main__` module in `sys.modules`
2. By Python design, the `__main__` module has `__spec__ = None`
3. The autoreloader code skips any module with `__spec__ = None`
4. Result: `manage.py` is never added to the file watch list
5. Changes to `manage.py` don't trigger server restart

## Solution

Add special case handling for the `__main__` module in the autoreloader's file iteration logic. Even though `__spec__` is None, we can access the module's filename via `__file__` and track it.

### Minimal Code Change Required
- **File**: `django/utils/autoreload.py`
- **Function**: `iter_modules_and_files()`
- **Changes**: Add 5 lines of code to handle `__main__` special case
- **Impact**: None to existing functionality, fixes regression

## Implementation Files

### 1. DJANGO_ISSUE_FIX.md
Comprehensive documentation including:
- Issue summary
- Root cause explanation
- Solution approach
- Code changes (before/after)
- Testing procedure
- Alternative approaches considered
- Impact analysis

### 2. django-autoreloader-30479.patch
A Git patch file that can be applied directly to the Django repository:
```bash
cd django
git apply django-autoreloader-30479.patch
```

### 3. test/django_autoreload.fabro
A Fabro workflow demonstrating the fix process:
- Clone Django repository
- Review the issue
- Examine the problematic code
- Implement the fix
- Verify with tests
- Create test cases

## Testing & Verification

### Manual Testing Steps
1. Create a Django project
2. Run `python manage.py runserver`
3. Edit `manage.py` to add a print statement
4. Verify the server restarts automatically
5. Verify the change is applied

### Automated Testing
- Run Django's autoreload test suite: `python -m pytest tests/autoreload/`
- Verify no regression in other auto-reload scenarios

## Key Improvements
- ✓ Fixes regression in Django 2.2.1
- ✓ Minimal code changes (backward compatible)
- ✓ No performance impact
- ✓ Handles edge case properly
- ✓ Well-documented solution

## Related Resources
- Django Issue Tracker: https://code.djangoproject.com/ticket/30479
- Django Repository: https://github.com/django/django
- Python Modules Documentation: https://docs.python.org/3/library/sys.html#sys.modules

## Fabro Workflow
This repository contains a complete Fabro workflow that can be used to:
1. Clone the Django repository
2. Understand the issue
3. Review the problematic code
4. Implement and test the fix
5. Verify the solution works correctly

Run the workflow with: `fabro run django_autoreload`
