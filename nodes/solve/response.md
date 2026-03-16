The fix is applied. Both `ASCIIUsernameValidator` and `UnicodeUsernameValidator` now use `\A` and `\Z` instead of `^` and `$` to anchor the regex.

**The problem:** Python's `$` in regex matches not only at the end of the string but also just before a trailing newline. This means a username like `"validuser\n"` would incorrectly pass validation.

**The fix:** Replace `^` with `\A` (matches only at the absolute start of the string) and `$` with `\Z` (matches only at the absolute end of the string, without the trailing-newline quirk). This ensures usernames ending with a newline are properly rejected.