import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  type ChangeEvent,
} from "react";
import {
  FileTree,
  useFileTree,
  useFileTreeSearch,
  useFileTreeSelection,
} from "@pierre/trees/react";
import { themeToTreeStyles, type GitStatus, type GitStatusEntry } from "@pierre/trees";
import pierreDark from "@pierre/theme/pierre-dark";
import type { FileDiff } from "@qltysh/fabro-api-client";

const CHANGE_KIND_TO_GIT_STATUS: Record<NonNullable<FileDiff["change_kind"]>, GitStatus> = {
  added:     "added",
  modified:  "modified",
  deleted:   "deleted",
  renamed:   "renamed",
  symlink:   "modified",
  submodule: "modified",
};

function filePath(file: FileDiff): string {
  return file.new_file.name || file.old_file.name;
}

function ancestorPaths(paths: readonly string[]): string[] {
  const set = new Set<string>();
  for (const p of paths) {
    let i = p.indexOf("/");
    while (i !== -1) {
      set.add(p.slice(0, i));
      i = p.indexOf("/", i + 1);
    }
  }
  return [...set];
}

interface FileTreeSidebarProps {
  files: readonly FileDiff[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
}

export function FileTreeSidebar({
  files,
  selectedPath,
  onSelect,
}: FileTreeSidebarProps) {
  const paths = useMemo(() => files.map(filePath), [files]);

  const gitStatus = useMemo<GitStatusEntry[]>(
    () =>
      files.map((file) => ({
        path:   filePath(file),
        status: file.change_kind
          ? CHANGE_KIND_TO_GIT_STATUS[file.change_kind]
          : "modified",
      })),
    [files],
  );

  const initialExpandedPaths = useMemo(() => ancestorPaths(paths), [paths]);

  const { model } = useFileTree({
    paths,
    flattenEmptyDirectories: true,
    initialExpandedPaths,
    initialSelectedPaths:    selectedPath ? [selectedPath] : undefined,
    gitStatus,
    icons:                   "standard",
    density:                 "default",
    search:                  true,
    fileTreeSearchMode:      "hide-non-matches",
    onSelectionChange:       (selected) => {
      const first = selected[0];
      if (first) onSelect(first);
    },
  });

  // Keep the long-lived model in sync when the underlying files change
  // (e.g. a Refresh fetched a new commit).
  useEffect(() => {
    model.resetPaths(paths);
  }, [model, paths]);

  useEffect(() => {
    model.setGitStatus(gitStatus);
  }, [model, gitStatus]);

  // Drive selection from the URL hash (back/forward navigation, deep links).
  const selection = useFileTreeSelection(model);
  useEffect(() => {
    if (!selectedPath) return;
    if (selection.length === 1 && selection[0] === selectedPath) return;
    const item = model.getItem(selectedPath);
    if (item) item.select();
  }, [model, selectedPath, selection]);

  const search = useFileTreeSearch(model);
  const handleSearchChange = (event: ChangeEvent<HTMLInputElement>) =>
    search.setValue(event.target.value);

  // `themeToTreeStyles` returns a Record<string, string> of CSS custom
  // properties (--trees-theme-*). React's CSSProperties is closed and
  // doesn't accept arbitrary string keys, so apply them imperatively to a
  // wrapper element — they cascade into the tree's shadow DOM as expected.
  const themeStyles = useMemo(() => themeToTreeStyles(pierreDark), []);
  const themeRef = useRef<HTMLDivElement | null>(null);
  useLayoutEffect(() => {
    const el = themeRef.current;
    if (!el) return;
    for (const [key, value] of Object.entries(themeStyles)) {
      if (key.startsWith("--")) el.style.setProperty(key, value);
    }
  }, [themeStyles]);

  return (
    <aside
      ref={themeRef}
      aria-label="Changed files"
      className="hidden md:flex sticky top-4 self-start w-72 shrink-0 flex-col gap-2 max-h-[calc(100vh-6rem)]"
    >
      <input
        type="search"
        value={search.value}
        onChange={handleSearchChange}
        placeholder="Filter changed files…"
        aria-label="Filter changed files"
        className="w-full rounded-md border border-line bg-panel px-2 py-1.5 text-sm text-fg placeholder:text-fg-muted focus:outline-2 focus:outline-focus focus:outline-offset-2"
      />
      <FileTree
        model={model}
        className="min-h-0 flex-1 overflow-hidden rounded-md border border-line bg-panel"
      />
    </aside>
  );
}
