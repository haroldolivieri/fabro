# Testing Human Gate with Auto-Approve

Here's a short greeting message:

---

**Hello! 👋**

Welcome to Fabro! This test demonstrates how human gates work with auto-approval enabled. The workflow will pause at the human gate stage, but since auto-approve is configured, it will automatically continue without requiring manual intervention.

This is useful for testing workflows that include human approval steps in CI/CD pipelines or automated processes where you want to validate the gate logic without blocking on actual human approval.

---

If you're looking to set up this test, you'd typically:

1. Define a workflow with a human gate stage
2. Configure auto-approve in the test setup
3. Run `fabro run <workflow-name>` to execute
4. Verify the gate is triggered and auto-approved in the output logs

Let me know if you need help setting up the workflow definition or test configuration!