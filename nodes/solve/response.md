Implemented the Django fix for this issue.

What changed:
- Added the model class to each `model_dict` in `AdminSite._build_app_dict()` as `model`.
- Added regression coverage in `tests/admin_views/test_adminsite.py`.
- Updated admin docs for `AdminSite.each_context()`.
- Added a 4.0 release note entry.

Files changed:
- `django/contrib/admin/sites.py`
- `tests/admin_views/test_adminsite.py`
- `docs/ref/contrib/admin/index.txt`
- `docs/releases/4.0.txt`

Validation:
- `python tests/runtests.py admin_views.test_adminsite`

This matches the minimal upstream-style fix for the issue.