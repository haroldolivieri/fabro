# fabro

> AI-powered workflow orchestration platform. Workflows are defined as Graphviz graphs and executed by the Fabro engine.

I want you to install Fabro for me. Execute the steps below autonomously.

OBJECTIVE: Install the Fabro CLI binary and verify it works.

DONE WHEN: `fabro --version` prints a version string.

## Choose install method

Check whether Homebrew is available:

```bash
command -v brew
```

If `brew` is present, install via Homebrew (preferred — brew manages updates automatically):

```bash
brew install fabro-sh/tap/fabro-nightly
```

Otherwise, use the install script:

```bash
curl -fsSL https://fabro.sh/install.sh | bash
```

## Verify

```bash
fabro --version
```

This must print a version string like `fabro 0.x.y`. If it does, installation is complete.

Tell the user to run `fabro install` in their terminal to complete setup.

EXECUTE NOW: Complete the above steps to achieve: `fabro --version` prints a version string.
