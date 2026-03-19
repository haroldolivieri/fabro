Perfect! The file `/tmp/scenario_flag.txt` contains:

```
SCENARIO_FLAG_42
```

This confirms that the sandbox filesystem is shared across nodes. The setup stage successfully created the file with the content `SCENARIO_FLAG_42`, and I was able to read it from the current context. This demonstrates that the filesystem is properly shared and accessible across different stages/nodes in the workflow.