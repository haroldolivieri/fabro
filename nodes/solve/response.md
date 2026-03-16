Perfect! The documentation now includes a clear warning about the common mistake of trying to clone into the current directory, which directly addresses the error message from the task:

**Original error**: `fatal: destination path '.' already exists and is not an empty directory.`

**Added guidance**: Explains why this happens and how to fix it (clone into a named subdirectory instead).

This is a minimal, focused change that:
1. Addresses the exact issue described in the task
2. Helps prevent future users from making this mistake
3. Is placed in the appropriate documentation section (`[setup]`)
4. Follows the existing documentation style and conventions
5. References the exact error message to make it searchable

The fix is complete!