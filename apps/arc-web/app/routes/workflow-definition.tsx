import { ChevronRightIcon } from "@heroicons/react/20/solid";
import { useEffect, useState } from "react";
import { useParams } from "react-router";
import type { BundledLanguage, FileContents } from "@pierre/diffs";
import { File } from "@pierre/diffs/react";
import { registerDotLanguage } from "../data/register-dot-language";
import { workflowData } from "./workflow-detail";

function CollapsibleFile({
  file,
  defaultOpen = true,
}: {
  file: FileContents;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);

  const lines = file.contents.split("\n");
  const lineCount = lines.length;
  const loc = lines.filter((l) => l.trim().length > 0).length;

  return (
    <div className="rounded-md border border-white/[0.06] bg-navy-800/50 overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2 px-4 py-2.5 text-left hover:bg-white/[0.02] transition-colors"
      >
        <ChevronRightIcon
          className={`size-4 text-navy-600 transition-transform duration-150 ${open ? "rotate-90" : ""}`}
        />
        <span className="font-mono text-xs text-navy-600">{file.name}</span>
        <span className="ml-auto font-mono text-xs text-navy-600/60">
          {lineCount} lines ({loc} loc)
        </span>
      </button>

      <div className={open ? "" : "hidden"}>
        <div className="border-t border-white/[0.06]" />
        <File
          file={file}
          options={{ theme: "pierre-dark", disableFileHeader: true }}
        />
      </div>
    </div>
  );
}

export default function WorkflowDefinition() {
  const { name } = useParams();
  const workflow = workflowData[name ?? ""];
  const [dotReady, setDotReady] = useState(false);

  useEffect(() => {
    let cancelled = false;
    registerDotLanguage().then(() => {
      if (!cancelled) setDotReady(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (workflow == null) {
    return <p className="text-sm text-navy-600">No configuration found.</p>;
  }

  return (
    <div className="flex flex-col gap-6">
      <CollapsibleFile
        file={{ name: "task.toml", contents: workflow.config, lang: "toml" }}
        defaultOpen={false}
      />
      {dotReady && (
        <CollapsibleFile
          file={{
            name: workflow.filename,
            contents: workflow.graph,
            lang: "dot" as BundledLanguage,
          }}
        />
      )}
    </div>
  );
}
