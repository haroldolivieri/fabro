import { describe, expect, test } from "bun:test";
import TestRenderer from "react-test-renderer";

import {
  deriveEmptyKind,
  emptyStateCopy,
  EmptyState,
  InlineErrorBanner,
  LoadingSkeleton,
  Toast,
} from "./states";

function renderToJson(element: React.ReactElement): any {
  return TestRenderer.create(element).toJSON();
}

describe("deriveEmptyKind", () => {
  test("submitted / starting / queued map to R4(a) 'starting'", () => {
    for (const status of ["submitted", "Submitted", "starting", "queued"]) {
      expect(
        deriveEmptyKind({
          runStatus: status,
          totalChanged: 0,
          degraded: false,
        }),
      ).toBe("starting");
    }
  });

  test("failed run without degraded fallback is R4(c1)", () => {
    expect(
      deriveEmptyKind({
        runStatus: "failed",
        totalChanged: 0,
        degraded: false,
      }),
    ).toBe("failed_before_checkpoint");
  });

  test("succeeded run with changes but no data is R4(c2) 'diff_lost'", () => {
    expect(
      deriveEmptyKind({
        runStatus: "succeeded",
        totalChanged: 3,
        degraded: false,
      }),
    ).toBe("diff_lost");
  });

  test("succeeded run with no changes is R4(b)", () => {
    expect(
      deriveEmptyKind({
        runStatus: "succeeded",
        totalChanged: 0,
        degraded: false,
      }),
    ).toBe("no_changes");
  });

  test("running run with no changes is R4(b)", () => {
    expect(
      deriveEmptyKind({
        runStatus: "running",
        totalChanged: 0,
        degraded: false,
      }),
    ).toBe("no_changes");
  });

  test("missing runStatus collapses to 'unknown'", () => {
    expect(
      deriveEmptyKind({
        runStatus: undefined,
        totalChanged: 0,
        degraded: false,
      }),
    ).toBe("unknown");
  });
});

describe("emptyStateCopy", () => {
  test("every kind resolves to distinct non-empty copy", () => {
    const seen = new Set<string>();
    for (const kind of [
      "starting",
      "no_changes",
      "failed_before_checkpoint",
      "diff_lost",
      "unknown",
    ] as const) {
      const c = emptyStateCopy(kind);
      expect(c.length).toBeGreaterThan(0);
      expect(seen.has(c)).toBe(false);
      seen.add(c);
    }
  });
});

describe("component rendering", () => {
  test("EmptyState wraps message in role=status", () => {
    let tree: TestRenderer.ReactTestRenderer | undefined;
    TestRenderer.act(() => {
      tree = TestRenderer.create(<EmptyState kind="starting" />);
    });
    const statusEl = tree!.root.findAll(
      (node) => node.type === "div" && node.props?.role === "status",
    );
    expect(statusEl.length).toBeGreaterThan(0);
  });

  test("LoadingSkeleton has aria-label", () => {
    let tree: TestRenderer.ReactTestRenderer | undefined;
    TestRenderer.act(() => {
      tree = TestRenderer.create(<LoadingSkeleton />);
    });
    const labeled = tree!.root.findAll(
      (node) => node.props?.["aria-label"] === "Loading files",
    );
    expect(labeled.length).toBeGreaterThan(0);
  });

  test("InlineErrorBanner fires onRetry when clicked", () => {
    let clicked = 0;
    let tree: TestRenderer.ReactTestRenderer | undefined;
    TestRenderer.act(() => {
      tree = TestRenderer.create(
        <InlineErrorBanner message="503" onRetry={() => (clicked += 1)} />,
      );
    });
    const button = tree!.root.findByType("button");
    TestRenderer.act(() => {
      button.props.onClick();
    });
    expect(clicked).toBe(1);
  });

  test("Toast renders its children in an aria-live region", () => {
    let tree: TestRenderer.ReactTestRenderer | undefined;
    TestRenderer.act(() => {
      tree = TestRenderer.create(<Toast>hello</Toast>);
    });
    const live = tree!.root.findAll(
      (node) => node.props?.["aria-live"] === "polite",
    );
    expect(live.length).toBeGreaterThan(0);
  });
});
