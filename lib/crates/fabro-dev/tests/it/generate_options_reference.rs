use std::path::Path;

use super::{fabro_dev, output_text, read_file, write_file};

fn options_reference(root: &Path) -> assert_cmd::Command {
    let mut cmd = fabro_dev();
    cmd.args(["generate-options-reference", "--root"]).arg(root);
    cmd
}

#[test]
fn write_updates_only_generated_region() {
    let fixture = tempfile::tempdir().expect("creating fixture");
    write_file(
        fixture.path(),
        "docs/reference/user-configuration.mdx",
        r"---
title: Settings
---

Intro copy.

<!-- generated:options -->
stale
<!-- /generated:options -->

Tail copy.
",
    );

    options_reference(fixture.path()).assert().success();

    let contents = read_file(fixture.path(), "docs/reference/user-configuration.mdx");
    assert!(
        contents.contains("Intro copy."),
        "manual intro should be preserved:\n{contents}"
    );
    assert!(
        contents.contains("Tail copy."),
        "manual tail should be preserved:\n{contents}"
    );
    assert!(
        contents.contains("## `[cli.output]`"),
        "generated output should include cli output settings:\n{contents}"
    );
    assert!(
        contents.contains("| `format` |"),
        "generated output should include option fields:\n{contents}"
    );
    assert!(
        contents.contains("## `[run.model]`"),
        "generated output should include run model settings:\n{contents}"
    );
    assert!(
        !contents.contains("stale"),
        "stale generated content should be replaced:\n{contents}"
    );
}

#[test]
fn check_passes_after_write() {
    let fixture = tempfile::tempdir().expect("creating fixture");
    write_file(
        fixture.path(),
        "docs/reference/user-configuration.mdx",
        r"<!-- generated:options -->
stale
<!-- /generated:options -->
",
    );

    options_reference(fixture.path()).assert().success();

    options_reference(fixture.path())
        .arg("--check")
        .assert()
        .success();
}

#[test]
fn check_fails_when_generated_region_is_stale() {
    let fixture = tempfile::tempdir().expect("creating fixture");
    write_file(
        fixture.path(),
        "docs/reference/user-configuration.mdx",
        r"<!-- generated:options -->
stale
<!-- /generated:options -->
",
    );

    let output = options_reference(fixture.path())
        .arg("--check")
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let stderr = output_text(&output.stderr);

    assert!(
        stderr.contains(
            "docs/reference/user-configuration.mdx is stale; run `cargo dev generate-options-reference`"
        ),
        "check failure should explain how to regenerate:\n{stderr}"
    );
}

#[test]
fn generated_reference_is_deterministic() {
    let fixture = tempfile::tempdir().expect("creating fixture");
    write_file(
        fixture.path(),
        "docs/reference/user-configuration.mdx",
        r"<!-- generated:options -->
stale
<!-- /generated:options -->
",
    );

    options_reference(fixture.path()).assert().success();
    let first = read_file(fixture.path(), "docs/reference/user-configuration.mdx");

    options_reference(fixture.path()).assert().success();
    let second = read_file(fixture.path(), "docs/reference/user-configuration.mdx");

    assert_eq!(first, second);
}
