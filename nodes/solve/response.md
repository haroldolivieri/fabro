Excellent! Now let me document the changes:

## Summary

I've successfully fixed the Django issue where named tuples used as arguments to `__range` queryset filters would raise a `TypeError`.

### The Problem
In Django 3.0, the `Query.resolve_lookup_value()` method in `django/db/models/sql/query.py` processes list and tuple arguments by iterating through their elements to resolve any lookup expressions. When it tries to reconstruct the tuple/list with the resolved elements, it was passing a generator directly to the type constructor:

```python
return type(value)(
    self.resolve_lookup_value(sub_value, can_reuse, allow_joins)
    for sub_value in value
)
```

This works fine for regular tuples and lists, which accept an iterable as a single argument. However, namedtuples have a different constructor signature - they expect positional arguments matching their fields. When passed a generator, the namedtuple constructor tries to unpack it as the first argument and fails with: `TypeError: __new__() missing 1 required positional argument: 'far'`.

### The Solution
The fix detects namedtuples (by checking for the `_fields` attribute on the class) and uses argument unpacking (`*`) only for them:

```python
resolved_items = (
    self.resolve_lookup_value(sub_value, can_reuse, allow_joins)
    for sub_value in value
)
# For namedtuples (which are subclasses of tuple with _fields attribute),
# we need to expand the items as separate arguments using *.
# For regular tuples and lists, passing a generator works fine.
if hasattr(type(value), '_fields'):
    return type(value)(*resolved_items)
else:
    return type(value)(resolved_items)
```

### Testing
The fix has been verified to work correctly with:
- Named tuples with 2+ fields ✓
- Regular tuples of various sizes ✓
- Lists of various sizes ✓
- Empty containers ✓
- Nested structures (namedtuples containing namedtuples, lists of namedtuples, etc.) ✓

The file `/tmp/django-repo/django/db/models/sql/query.py` now contains the complete fix.