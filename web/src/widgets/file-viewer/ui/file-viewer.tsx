import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { ChevronLeft, ChevronRight, Download, MoreHorizontal, X } from "lucide-react";
import { Suspense, useState } from "react";

import { fileContentUrl, filePreview, fileTextQuery, type FileInfo } from "@/entities/file";
import { meQuery } from "@/entities/user";
import { PurgeDialog } from "@/features/purge";
import { baseName } from "@/shared/lib/page-path";
import { Button, buttonVariants } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { Skeleton } from "@/shared/ui/skeleton";

import { PdfPage } from "./pdf-page";

type FileViewerProps = {
  file: FileInfo;
  page: number;
  quote: string | null;
  onPageChange: (page: number) => void;
  onClose?: () => void;
};

export function FileViewer({ file, page, quote, onPageChange, onClose }: FileViewerProps) {
  const preview = filePreview(file.mime);
  const isPdf = preview === "pdf";
  const pageCount = Math.max(1, file.page_count);
  const current = Math.min(Math.max(1, page), pageCount);
  return (
    <section aria-label={`File ${file.path}`} className="flex h-full min-h-0 flex-col">
      <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b px-4">
        <h2 className="min-w-0 truncate text-sm font-medium" title={file.path}>
          {baseName(file.path)}
        </h2>
        <div className="flex items-center gap-1">
          {isPdf && pageCount > 1 ? (
            <>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Previous page"
                disabled={current <= 1}
                onClick={() => onPageChange(current - 1)}
              >
                <ChevronLeft />
              </Button>
              <span className="text-xs text-muted-foreground tabular-nums">
                {current} / {pageCount}
              </span>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Next page"
                disabled={current >= pageCount}
                onClick={() => onPageChange(current + 1)}
              >
                <ChevronRight />
              </Button>
            </>
          ) : null}
          <a
            href={fileContentUrl(file.id, { download: true })}
            download
            aria-label="Download file"
            className={buttonVariants({ variant: "ghost", size: "icon-sm" })}
          >
            <Download />
          </a>
          <FileMenu file={file} />
          {onClose ? (
            <Button variant="ghost" size="icon-sm" aria-label="Close file" onClick={onClose}>
              <X />
            </Button>
          ) : null}
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-auto bg-muted">
        {isPdf ? <PdfPage url={fileContentUrl(file.id)} page={current} quote={quote} /> : null}
        {preview === "image" ? (
          <img
            src={fileContentUrl(file.id)}
            alt={baseName(file.path)}
            className="mx-auto max-w-full p-4"
          />
        ) : null}
        {preview === "page" ? (
          <iframe
            src={fileContentUrl(file.id)}
            title={baseName(file.path)}
            sandbox="allow-scripts"
            className="block size-full bg-white"
          />
        ) : null}
        {preview === "text" ? (
          <Suspense fallback={<Skeleton className="m-4 h-40" />}>
            <TextContent id={file.id} quote={quote} />
          </Suspense>
        ) : null}
        {preview === "none" ? (
          <p className="p-6 text-sm text-muted-foreground">
            This file type has no preview. Use download to open it.
          </p>
        ) : null}
      </div>
    </section>
  );
}

function TextContent({ id, quote }: { id: number; quote: string | null }) {
  const { data: text } = useSuspenseQuery(fileTextQuery(id));
  const start = quote ? text.indexOf(quote) : -1;
  return (
    <pre className="m-4 rounded-md border bg-background p-4 font-mono text-xs leading-6 break-words whitespace-pre-wrap">
      {start === -1 || !quote ? (
        text
      ) : (
        <>
          {text.slice(0, start)}
          <mark className="rounded-sm bg-highlight">{quote}</mark>
          {text.slice(start + quote.length)}
        </>
      )}
    </pre>
  );
}

function FileMenu({ file }: { file: FileInfo }) {
  const { data: me } = useQuery(meQuery());
  const navigate = useNavigate();
  const [isPurging, setIsPurging] = useState(false);
  if (!me?.is_admin) {
    return null;
  }
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={<Button variant="ghost" size="icon-sm" />}
          aria-label="File actions"
        >
          <MoreHorizontal className="size-4" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem variant="destructive" onClick={() => setIsPurging(true)}>
            Purge permanently
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      {isPurging ? (
        <PurgeDialog
          kind="file"
          id={file.id}
          path={file.path}
          onClose={() => setIsPurging(false)}
          onDone={() => void navigate({ to: "/" })}
        />
      ) : null}
    </>
  );
}
