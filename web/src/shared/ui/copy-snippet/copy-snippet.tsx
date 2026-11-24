import { Check, Copy } from "lucide-react";
import { useState } from "react";

import { Button } from "@/shared/ui/button";

const copiedMs = 1500;

type CopySnippetProps = { label: string; text: string };

export function CopySnippet({ label, text }: CopySnippetProps) {
  const [isCopied, setIsCopied] = useState(false);

  async function handleCopy() {
    await navigator.clipboard.writeText(text);
    setIsCopied(true);
    setTimeout(() => setIsCopied(false), copiedMs);
  }

  return (
    <div className="flex items-start gap-2 rounded-md bg-muted py-1 pr-1 pl-3">
      <pre
        aria-label={label}
        className="min-w-0 flex-1 overflow-x-auto py-1 font-mono text-xs leading-5"
      >
        {text}
      </pre>
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label={`Copy ${label}`}
        onClick={() => void handleCopy()}
      >
        {isCopied ? <Check /> : <Copy />}
      </Button>
    </div>
  );
}
