import { GlobalWorkerOptions, TextLayer, getDocument } from "pdfjs-dist";
// oxlint-disable-next-line import/default -- Vite resolves the ?url suffix to a default export
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { useEffect, useRef, useState } from "react";

import { findQuote } from "@/shared/lib/pdf-quote";

import "pdfjs-dist/web/pdf_viewer.css";

GlobalWorkerOptions.workerSrc = workerUrl;

const quoteHighlightName = "source-quote";
const pagePadding = 32;

type PdfPageProps = {
  url: string;
  page: number;
  quote: string | null;
};

export function PdfPage({ url, page, quote }: PdfPageProps) {
  const frameRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const textRef = useRef<HTMLDivElement>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    const frame = frameRef.current;
    const canvas = canvasRef.current;
    const text = textRef.current;
    if (!frame || !canvas || !text) {
      return undefined;
    }
    const task = getDocument({ url });
    let isCancelled = false;

    const draw = async () => {
      const pdf = await task.promise;
      const pdfPage = await pdf.getPage(page);
      const base = pdfPage.getViewport({ scale: 1 });
      const available = (frame.parentElement?.clientWidth ?? base.width) - pagePadding;
      const viewport = pdfPage.getViewport({ scale: Math.max(0.5, available / base.width) });
      canvas.width = Math.floor(viewport.width * devicePixelRatio);
      canvas.height = Math.floor(viewport.height * devicePixelRatio);
      canvas.style.width = `${Math.floor(viewport.width)}px`;
      canvas.style.height = `${Math.floor(viewport.height)}px`;
      frame.style.setProperty("--total-scale-factor", String(viewport.scale));
      text.replaceChildren();
      await pdfPage.render({
        canvas,
        viewport,
        transform: [devicePixelRatio, 0, 0, devicePixelRatio, 0, 0],
      }).promise;
      const layer = new TextLayer({
        textContentSource: pdfPage.streamTextContent(),
        container: text,
        viewport,
      });
      await layer.render();
      if (quote) {
        highlightQuote(layer.textDivs, quote);
      }
    };

    const run = async () => {
      try {
        await draw();
        setProblem(null);
      } catch (error) {
        if (!isCancelled) {
          setProblem(error instanceof Error ? error.message : "Could not render this page");
        }
      }
    };
    void run();
    return () => {
      isCancelled = true;
      CSS.highlights.delete(quoteHighlightName);
      void task.destroy();
    };
  }, [url, page, quote]);

  return (
    <div className="p-4">
      <div
        ref={frameRef}
        className="relative mx-auto w-fit bg-white shadow-sm"
        data-testid="pdf-frame"
      >
        <canvas ref={canvasRef}>Page {page} of the file</canvas>
        <div ref={textRef} className="textLayer" />
      </div>
      {problem ? <p className="mt-3 text-sm text-destructive">{problem}</p> : null}
    </div>
  );
}

function highlightQuote(divs: readonly HTMLElement[], quote: string) {
  const range = findQuote(
    divs.map((div) => div.textContent ?? ""),
    quote,
  );
  if (!range) {
    return;
  }
  const first = divs[range.start.piece]?.firstChild;
  const last = divs[range.end.piece]?.firstChild;
  if (!first || !last) {
    return;
  }
  const selection = new Range();
  selection.setStart(first, range.start.offset);
  selection.setEnd(last, range.end.offset);
  CSS.highlights.set(quoteHighlightName, new Highlight(selection));
  first.parentElement?.scrollIntoView({ block: "center" });
}
