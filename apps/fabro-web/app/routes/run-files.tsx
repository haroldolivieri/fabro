import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactElement,
} from "react";
import {
  isRouteErrorResponse,
  useNavigation,
  useParams,
  useRevalidator,
  useRouteError,
} from "react-router";
import { MultiFileDiff, PatchDiff } from "@pierre/diffs/react";
import { useTheme } from "../lib/theme";
import { apiJsonOrNull } from "../api";
import type {
  FileDiff as ApiFileDiff,
  PaginatedRunFileList,
} from "@qltysh/fabro-api-client";

export const handle = { wide: true };

export async function loader({ request, params }: any) {
  const data = await apiJsonOrNull<PaginatedRunFileList>(
    `/runs/${params.id}/files`,
    { request },
  );
  return data;
}

// Events that can change the diff. CheckpointCompleted is the canonical
// signal; the others cover terminal state transitions that also merit a
// refresh.
const REFRESH_EVENTS = new Set([
  "checkpoint.completed",
  "run.completed",
  "run.failed",
]);

const PLACEHOLDER_CLASSES =
  "flex items-center justify-between rounded-md border border-line bg-panel/60 px-4 py-3 text-sm text-fg-muted";

function DegradedBanner({ reason }: { reason?: string }) {
  return (
    <div className="rounded-md border border-amber-500/30 bg-amber-950/20 px-4 py-3 text-sm text-amber-100">
      {banner_copy_for_reason(reason)}
    </div>
  );
}

function banner_copy_for_reason(reason: string | undefined): string {
  switch (reason) {
    case "sandbox_gone":
      return "Showing final patch only. This run's sandbox has been cleaned up, so individual file contents are no longer available.";
    case "provider_unsupported":
      return "Live diff isn't supported for this sandbox provider. Showing the patch captured at the last checkpoint.";
    case "sandbox_unreachable":
    default:
      return "Couldn't reach this run's sandbox. Showing the patch captured at the last checkpoint — refresh to try again.";
  }
}

function SensitivePlaceholder({ name }: { name: string }) {
  return (
    <div className={PLACEHOLDER_CLASSES}>
      <span className="font-mono text-fg-2">{name}</span>
      <span className="rounded bg-rose-950/40 px-2 py-0.5 text-xs text-rose-200">
        sensitive — contents omitted
      </span>
    </div>
  );
}

function BinaryPlaceholder({ name }: { name: string }) {
  return (
    <div className={PLACEHOLDER_CLASSES}>
      <span className="font-mono text-fg-2">{name}</span>
      <span className="rounded bg-panel-alt/80 px-2 py-0.5 text-xs text-fg-3">
        binary — not shown inline
      </span>
    </div>
  );
}

function TruncatedPlaceholder({
  name,
  reason,
}: {
  name: string;
  reason?: string;
}) {
  const label =
    reason === "budget_exhausted"
      ? "omitted — too many files changed"
      : "too large to render inline";
  return (
    <div className={PLACEHOLDER_CLASSES}>
      <span className="font-mono text-fg-2">{name}</span>
      <span className="rounded bg-panel-alt/80 px-2 py-0.5 text-xs text-fg-3">
        {label}
      </span>
    </div>
  );
}

function SymlinkOrSubmodulePlaceholder({
  name,
  kind,
}: {
  name: string;
  kind: "symlink" | "submodule";
}) {
  return (
    <div className={PLACEHOLDER_CLASSES}>
      <span className="font-mono text-fg-2">{name}</span>
      <span className="rounded bg-panel-alt/80 px-2 py-0.5 text-xs text-fg-3">
        {kind}
      </span>
    </div>
  );
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="rounded-md border border-dashed border-line bg-panel/40 px-6 py-10 text-center text-sm text-fg-muted">
      {message}
    </div>
  );
}

function LoadingSkeleton() {
  return (
    <div className="flex flex-col gap-3" aria-label="Loading files">
      <div className="h-8 rounded-md bg-panel/60 motion-safe:animate-pulse" />
      <div className="h-32 rounded-md bg-panel/60 motion-safe:animate-pulse" />
      <div className="h-32 rounded-md bg-panel/60 motion-safe:animate-pulse" />
    </div>
  );
}

function Toolbar({
  onRefresh,
  refreshing,
  freshness,
  refreshButtonRef,
}: {
  onRefresh: () => void;
  refreshing: boolean;
  freshness: string | null;
  refreshButtonRef?: React.Ref<HTMLButtonElement>;
}) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border border-line bg-panel/40 px-3 py-2 text-xs text-fg-muted">
      <span aria-live="polite">{freshness ?? "\u00A0"}</span>
      <button
        ref={refreshButtonRef}
        type="button"
        onClick={onRefresh}
        disabled={refreshing}
        className="min-h-[44px] min-w-[44px] rounded-md border border-line bg-panel px-3 py-1 text-xs font-medium text-fg-2 transition-colors hover:bg-overlay disabled:opacity-60"
      >
        {refreshing ? "Refreshing…" : "Refresh"}
      </button>
    </div>
  );
}

function pick_placeholder(file: ApiFileDiff): ReactElement | null {
  const display_name = file.new_file.name || file.old_file.name;
  // Priority: sensitive > binary > symlink/submodule > truncated. Security
  // flags must never be hidden by a lesser placeholder.
  if (file.sensitive) {
    return <SensitivePlaceholder name={display_name} />;
  }
  if (file.binary) {
    return <BinaryPlaceholder name={display_name} />;
  }
  if (file.change_kind === "symlink") {
    return <SymlinkOrSubmodulePlaceholder name={display_name} kind="symlink" />;
  }
  if (file.change_kind === "submodule") {
    return (
      <SymlinkOrSubmodulePlaceholder name={display_name} kind="submodule" />
    );
  }
  if (file.truncated) {
    return (
      <TruncatedPlaceholder
        name={display_name}
        reason={file.truncation_reason}
      />
    );
  }
  return null;
}

function formatRelative(iso: string | null, now: number): string {
  if (!iso) return "";
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return "";
  const diff = Math.max(0, Math.floor((now - then) / 1000));
  if (diff < 5) return "just now";
  if (diff < 60) return `${diff}s ago`;
  const m = Math.floor(diff / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}

function useSseRevalidation(runId: string | undefined) {
  const revalidator = useRevalidator();
  useEffect(() => {
    if (!runId) return;
    const source = new EventSource(`/api/v1/runs/${runId}/attach?since_seq=1`);
    let debounce: ReturnType<typeof setTimeout> | undefined;
    source.onmessage = (msg) => {
      try {
        const payload = JSON.parse(msg.data);
        if (REFRESH_EVENTS.has(payload.event)) {
          clearTimeout(debounce);
          debounce = setTimeout(() => revalidator.revalidate(), 500);
        }
      } catch {
        // ignore malformed payloads
      }
    };
    return () => {
      clearTimeout(debounce);
      source.close();
    };
    // revalidator is stable across renders per react-router; omitting it
    // keeps the effect from reattaching on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [runId]);
}

/// Tailwind `md` breakpoint. Below this, split diffs collapse to unified
/// without updating any persisted preference.
const MD_BREAKPOINT_PX = 768;

function useNarrowViewport(): boolean {
  const [narrow, setNarrow] = useState(false);
  useEffect(() => {
    if (typeof window === "undefined") return;
    const mql = window.matchMedia(`(max-width: ${MD_BREAKPOINT_PX - 1}px)`);
    const apply = () => setNarrow(mql.matches);
    apply();
    mql.addEventListener("change", apply);
    return () => mql.removeEventListener("change", apply);
  }, []);
  return narrow;
}

function isEditableElement(el: Element | null): boolean {
  if (!el) return false;
  const tag = el.tagName.toLowerCase();
  if (tag === "input" || tag === "textarea" || tag === "select") return true;
  return (el as HTMLElement).isContentEditable === true;
}

function useFileKeyboardNav(
  containerRef: React.RefObject<HTMLDivElement | null>,
  fileCount: number,
) {
  useEffect(() => {
    if (!containerRef.current) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "j" && event.key !== "k") return;
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (isEditableElement(document.activeElement)) return;
      const container = containerRef.current;
      if (!container) return;
      const rows = Array.from(
        container.querySelectorAll<HTMLElement>('[data-run-file-row="true"]'),
      );
      if (rows.length === 0) return;
      const active = document.activeElement as HTMLElement | null;
      const currentIdx = rows.findIndex((row) => row.contains(active));
      let nextIdx: number;
      if (currentIdx < 0) {
        nextIdx = 0;
      } else {
        nextIdx = event.key === "j" ? currentIdx + 1 : currentIdx - 1;
      }
      if (nextIdx < 0 || nextIdx >= rows.length) return;
      event.preventDefault();
      const target = rows[nextIdx];
      target.focus({ preventScroll: false });
      target.scrollIntoView({ block: "nearest", behavior: "smooth" });
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
    // fileCount drives re-attachment so rows picked up after data changes are
    // addressable without ghost references.
  }, [containerRef, fileCount]);
}

function useFreshness(
  meta: PaginatedRunFileList["meta"] | null,
  lastFetchedAt: number | null,
): string | null {
  // Tick every 10 seconds so relative timestamps stay current.
  const [, setTick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 10_000);
    return () => clearInterval(id);
  }, []);

  if (!meta) return null;
  const now = Date.now();
  const captured = meta.to_sha_committed_at
    ? `Captured ${formatRelative(meta.to_sha_committed_at, now)}`
    : null;
  const fetched = lastFetchedAt
    ? `Fetched ${formatRelative(new Date(lastFetchedAt).toISOString(), now)}`
    : null;
  if (meta.degraded && captured && fetched) {
    return `${captured} · ${fetched}`;
  }
  return captured ?? fetched ?? null;
}

export function ErrorBoundary() {
  const error = useRouteError();
  if (isRouteErrorResponse(error)) {
    if (error.status === 401 || error.status === 403) {
      return (
        <EmptyState message="You don't have access to this run's files." />
      );
    }
    if (error.status === 503 || error.status === 429) {
      return (
        <EmptyState message="The diff service is temporarily unavailable. Please retry in a moment." />
      );
    }
    return (
      <EmptyState
        message={`Something went wrong (${error.status}). Please contact support if this persists.`}
      />
    );
  }
  return (
    <EmptyState message="Something went wrong loading this run's files." />
  );
}

function fileRowId(name: string): string {
  return `run-file:${name}`;
}

function decodeDeepLinkFile(hash: string): string | null {
  if (!hash) return null;
  const withoutHash = hash.startsWith("#") ? hash.slice(1) : hash;
  const prefix = "file=";
  if (!withoutHash.startsWith(prefix)) return null;
  try {
    return decodeURIComponent(withoutHash.slice(prefix.length));
  } catch {
    return null;
  }
}

function Toast({ children }: { children: React.ReactNode }) {
  return (
    <div
      role="status"
      aria-live="polite"
      className="pointer-events-none fixed bottom-6 right-6 z-50 rounded-md border border-line bg-panel/95 px-3 py-2 text-xs text-fg-2 shadow-lg"
    >
      {children}
    </div>
  );
}

export default function RunFiles({ loaderData }: any) {
  const theme = useTheme();
  const params = useParams();
  const navigation = useNavigation();
  const revalidator = useRevalidator();
  const data = loaderData as PaginatedRunFileList | null;
  const narrow = useNarrowViewport();

  const lastFetchedAtRef = useRef<number | null>(null);
  useEffect(() => {
    // Refresh the "Fetched N seconds ago" reference whenever a new loader
    // response lands.
    lastFetchedAtRef.current = Date.now();
  }, [data]);

  useSseRevalidation(params.id);

  const isInitialLoading = navigation.state === "loading" && !loaderData;
  const isRevalidating = revalidator.state === "loading";
  const freshness = useFreshness(data?.meta ?? null, lastFetchedAtRef.current);
  const diffStyle: "split" | "unified" = narrow ? "unified" : "split";
  const pierreTheme =
    theme.theme === "dark" ? "pierre-dark" : "pierre-light";

  const refreshButtonRef = useRef<HTMLButtonElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const refreshingPrev = useRef(false);
  useEffect(() => {
    // When a revalidation completes, return focus to the Refresh button so
    // keyboard-first users stay oriented.
    if (refreshingPrev.current && !isRevalidating) {
      refreshButtonRef.current?.focus({ preventScroll: true });
    }
    refreshingPrev.current = isRevalidating;
  }, [isRevalidating]);

  const fileCount = data?.data.length ?? 0;
  useFileKeyboardNav(containerRef, fileCount);

  // Deep-link handling
  const [deepLinkToast, setDeepLinkToast] = useState<string | null>(null);
  const [hashFile, setHashFile] = useState<string | null>(() => {
    if (typeof window === "undefined") return null;
    return decodeDeepLinkFile(window.location.hash);
  });
  useEffect(() => {
    if (typeof window === "undefined") return;
    const onHashChange = () =>
      setHashFile(decodeDeepLinkFile(window.location.hash));
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  useEffect(() => {
    if (!hashFile || !data) return;
    if (data.meta.degraded && data.meta.patch) {
      setDeepLinkToast(
        "File-level navigation isn't available in the patch-only view.",
      );
      const id = setTimeout(() => setDeepLinkToast(null), 5000);
      return () => clearTimeout(id);
    }
    const exists = data.data.some(
      (f) =>
        f.new_file.name === hashFile || f.old_file.name === hashFile,
    );
    if (!exists) {
      setDeepLinkToast(`File ${hashFile} is not in this run.`);
      const id = setTimeout(() => setDeepLinkToast(null), 5000);
      return () => clearTimeout(id);
    }
    // Scroll the matching row into view and focus it.
    const el = document.getElementById(fileRowId(hashFile));
    if (el) {
      el.scrollIntoView({ block: "start", behavior: "smooth" });
      el.focus({ preventScroll: true });
    }
  }, [hashFile, data]);

  const renderFiles = useCallback(
    (files: ApiFileDiff[]): ReactElement[] =>
      files.map((file, idx) => {
        const display = file.new_file.name || file.old_file.name;
        const placeholder = pick_placeholder(file);
        if (placeholder) {
          return (
            <div
              key={`${display}-${idx}`}
              id={fileRowId(display)}
              tabIndex={-1}
              data-run-file-row="true"
              role="region"
              aria-label={`${file.change_kind ?? "changed"}: ${display}`}
              className="focus:outline-2 focus:outline-focus focus:outline-offset-2 rounded-md"
            >
              {placeholder}
            </div>
          );
        }
        return (
          <div
            key={`${file.new_file.name}-${idx}`}
            id={fileRowId(display)}
            tabIndex={-1}
            data-run-file-row="true"
            role="region"
            aria-label={`${file.change_kind ?? "modified"}: ${file.new_file.name}`}
            className="focus:outline-2 focus:outline-focus focus:outline-offset-2 rounded-md"
          >
            <MultiFileDiff
              oldFile={file.old_file}
              newFile={file.new_file}
              options={{
                diffStyle,
                theme: pierreTheme,
              }}
            />
          </div>
        );
      }),
    [diffStyle, pierreTheme],
  );

  if (isInitialLoading) {
    return <LoadingSkeleton />;
  }

  if (!data) {
    return (
      <EmptyState message="The diff for this run is not available right now." />
    );
  }

  const { data: files, meta } = data;

  const toolbar = (
    <Toolbar
      onRefresh={() => revalidator.revalidate()}
      refreshing={isRevalidating}
      freshness={freshness}
      refreshButtonRef={refreshButtonRef}
    />
  );

  // Degraded: render the unified patch string directly. Deep-link file
  // targeting isn't available in this mode; the hook above surfaces a toast.
  if (meta.degraded && meta.patch) {
    return (
      <div ref={containerRef} className="flex flex-col gap-4">
        {toolbar}
        <DegradedBanner reason={meta.degraded_reason} />
        <PatchDiff
          patch={meta.patch}
          options={{
            diffStyle,
            theme: pierreTheme,
          }}
        />
        {deepLinkToast && <Toast>{deepLinkToast}</Toast>}
      </div>
    );
  }

  if (files.length === 0) {
    return (
      <div ref={containerRef} className="flex flex-col gap-4">
        {toolbar}
        <EmptyState
          message={
            meta.total_changed === 0
              ? "This run didn't change any files."
              : "No recoverable diff is available for this run."
          }
        />
        {deepLinkToast && <Toast>{deepLinkToast}</Toast>}
      </div>
    );
  }

  return (
    <div ref={containerRef} className="flex flex-col gap-4">
      {toolbar}
      {renderFiles(files)}
      {deepLinkToast && <Toast>{deepLinkToast}</Toast>}
    </div>
  );
}
