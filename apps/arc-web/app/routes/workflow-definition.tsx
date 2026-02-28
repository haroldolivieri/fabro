import { ChevronRightIcon } from "@heroicons/react/20/solid";
import { useEffect, useRef, useState } from "react";
import { useParams } from "react-router";
import { workflowData } from "./workflow-detail";
import { dotLanguage } from "../data/dot-grammar";

function CodeBlock({
  code,
  lang,
  filename,
  defaultOpen = true,
}: {
  code: string;
  lang: string;
  filename: string;
  defaultOpen?: boolean;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [ready, setReady] = useState(false);
  const [open, setOpen] = useState(defaultOpen);

  useEffect(() => {
    let cancelled = false;

    async function highlight() {
      const { createHighlighter } = await import("shiki");
      if (cancelled) return;

      const highlighter = await createHighlighter({
        themes: ["nord"],
        langs: lang === "dot" ? [dotLanguage] : [lang],
      });

      if (cancelled) return;

      const html = highlighter.codeToHtml(code, {
        lang,
        theme: "nord",
      });

      if (cancelled || containerRef.current == null) return;
      containerRef.current.innerHTML = html;
      setReady(true);
    }

    highlight();
    return () => {
      cancelled = true;
    };
  }, [code, lang]);

  const lines = code.split("\n");
  const lineCount = lines.length;
  const loc = lines.filter((l) => l.trim().length > 0).length;

  return (
    <div className="rounded-lg border border-white/[0.06] bg-navy-800/50 overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2 px-4 py-2.5 text-left hover:bg-white/[0.02] transition-colors"
      >
        <ChevronRightIcon
          className={`size-4 text-navy-600 transition-transform duration-150 ${open ? "rotate-90" : ""}`}
        />
        <span className="font-mono text-xs text-navy-600">{filename}</span>
        <span className="ml-auto font-mono text-xs text-navy-600/60">
          {lineCount} lines ({loc} loc)
        </span>
      </button>

      <div className={open ? "" : "hidden"}>
        <div className="border-t border-white/[0.06]" />
        <div
          ref={containerRef}
          className={`shiki-container overflow-x-auto transition-opacity duration-200 ${ready ? "opacity-100" : "opacity-0"}`}
        />

        {!ready && (
          <pre className="px-4 py-4 font-mono text-sm leading-relaxed text-navy-600">
            {code}
          </pre>
        )}
      </div>
    </div>
  );
}

export default function WorkflowDefinition() {
  const { name } = useParams();
  const workflow = workflowData[name ?? ""];

  if (workflow == null) {
    return <p className="text-sm text-navy-600">No configuration found.</p>;
  }

  return (
    <div className="flex flex-col gap-6">
      <CodeBlock code={workflow.config} lang="toml" filename="task.toml" defaultOpen={false} />
      <CodeBlock code={workflow.graph} lang="dot" filename={workflow.filename} />
    </div>
  );
}
