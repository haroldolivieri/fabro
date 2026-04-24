use super::{fabro_dev, output_text, write_file};

#[test]
fn refresh_spa_mirrors_dist_and_removes_source_maps() {
    let fixture = tempfile::tempdir().expect("creating fixture");
    write_file(fixture.path(), "apps/fabro-web/dist/index.html", b"index");
    write_file(fixture.path(), "apps/fabro-web/dist/assets/app.js", b"app");
    write_file(
        fixture.path(),
        "apps/fabro-web/dist/assets/app.js.map",
        b"map",
    );
    write_file(
        fixture.path(),
        "lib/crates/fabro-spa/assets/stale.txt",
        b"stale",
    );

    let output = fabro_dev()
        .args([
            "refresh-spa",
            "--root",
            fixture
                .path()
                .to_str()
                .expect("fixture path should be utf-8"),
            "--skip-build",
        ])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = output_text(&output.stdout);

    assert!(
        stdout.contains("Refreshed lib/crates/fabro-spa/assets"),
        "refresh-spa should report refreshed assets:\n{stdout}"
    );
    assert!(
        fixture
            .path()
            .join("lib/crates/fabro-spa/assets/index.html")
            .is_file()
    );
    assert!(
        fixture
            .path()
            .join("lib/crates/fabro-spa/assets/assets/app.js")
            .is_file()
    );
    assert!(
        !fixture
            .path()
            .join("lib/crates/fabro-spa/assets/assets/app.js.map")
            .exists()
    );
    assert!(
        !fixture
            .path()
            .join("lib/crates/fabro-spa/assets/stale.txt")
            .exists()
    );
}

#[test]
fn refresh_spa_missing_dist_errors_cleanly() {
    let fixture = tempfile::tempdir().expect("creating fixture");

    let output = fabro_dev()
        .args([
            "refresh-spa",
            "--root",
            fixture
                .path()
                .to_str()
                .expect("fixture path should be utf-8"),
            "--skip-build",
        ])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let stderr = output_text(&output.stderr);

    assert!(
        stderr.contains("apps/fabro-web/dist is missing; run `bun run build`"),
        "missing dist should explain how to recover:\n{stderr}"
    );
}

#[test]
fn check_spa_budgets_passes_fixture_assets() {
    let fixture = tempfile::tempdir().expect("creating fixture");
    write_file(
        fixture.path(),
        "lib/crates/fabro-spa/assets/index.html",
        b"hello",
    );

    let output = fabro_dev()
        .args([
            "check-spa-budgets",
            "--root",
            fixture
                .path()
                .to_str()
                .expect("fixture path should be utf-8"),
            "--asset-budget-bytes",
            "100",
            "--payload-budget-bytes",
            "100",
        ])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = output_text(&output.stdout);

    assert!(
        stdout.contains("fabro-spa asset bytes: 5"),
        "budget check should print raw bytes:\n{stdout}"
    );
    assert!(
        stdout.contains("fabro-spa estimated compressed payload bytes:"),
        "budget check should print compressed payload bytes:\n{stdout}"
    );
}

#[test]
fn check_spa_budgets_fails_when_assets_exceed_budget() {
    let fixture = tempfile::tempdir().expect("creating fixture");
    write_file(
        fixture.path(),
        "lib/crates/fabro-spa/assets/index.html",
        b"hello",
    );

    let output = fabro_dev()
        .args([
            "check-spa-budgets",
            "--root",
            fixture
                .path()
                .to_str()
                .expect("fixture path should be utf-8"),
            "--asset-budget-bytes",
            "4",
            "--payload-budget-bytes",
            "100",
        ])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let stderr = output_text(&output.stderr);

    assert!(
        stderr.contains("fabro-spa committed assets exceed budget: 5 > 4"),
        "budget failure should report raw byte overage:\n{stderr}"
    );
}

#[test]
fn check_spa_budgets_fails_when_source_map_is_present() {
    let fixture = tempfile::tempdir().expect("creating fixture");
    write_file(
        fixture.path(),
        "lib/crates/fabro-spa/assets/assets/app.js.map",
        b"map",
    );

    let output = fabro_dev()
        .args([
            "check-spa-budgets",
            "--root",
            fixture
                .path()
                .to_str()
                .expect("fixture path should be utf-8"),
        ])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let stderr = output_text(&output.stderr);

    assert!(
        stderr.contains("source map files are not allowed in fabro-spa assets"),
        "source maps should fail the budget check:\n{stderr}"
    );
}

#[test]
fn check_spa_budgets_missing_assets_errors_cleanly() {
    let fixture = tempfile::tempdir().expect("creating fixture");

    let output = fabro_dev()
        .args([
            "check-spa-budgets",
            "--root",
            fixture
                .path()
                .to_str()
                .expect("fixture path should be utf-8"),
        ])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let stderr = output_text(&output.stderr);

    assert!(
        stderr.contains("fabro-spa assets directory is missing"),
        "missing assets should be reported clearly:\n{stderr}"
    );
}
