All 75 tests pass (12 skipped, which are the Selenium tests requiring a browser).

## Summary

**Issue**: Django ticket #32219 — When `verbose_name` is explicitly set on an `InlineModelAdmin`, the `verbose_name_plural` should default to `verbose_name + 's'` instead of always falling back to the model's `Meta.verbose_name_plural`.

**Fix**: In `django/contrib/admin/options.py`, in `InlineModelAdmin.__init__()`, the `verbose_name_plural` resolution is moved **before** the `verbose_name` fallback assignment. The logic now:

1. If `verbose_name_plural` is `None` and `verbose_name` is also `None` → use `model._meta.verbose_name_plural` (existing behavior)
2. If `verbose_name_plural` is `None` but `verbose_name` is set → derive plural as `verbose_name + 's'` using `format_lazy` (new behavior)
3. If `verbose_name_plural` is explicitly set → keep it as-is (existing behavior)
4. Then, if `verbose_name` is `None` → fall back to `model._meta.verbose_name` (existing behavior, unchanged)

This makes inline admin classes consistent with model `Meta` classes, which also auto-derive `verbose_name_plural` from `verbose_name`.