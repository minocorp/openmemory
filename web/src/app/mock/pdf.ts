const encoder = new TextEncoder();

export function simplePdf(pages: string[][]): Uint8Array<ArrayBuffer> {
  const fontNumber = 3 + pages.length * 2;
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    `<< /Type /Pages /Kids [${pages.map((_, index) => `${3 + index * 2} 0 R`).join(" ")}] /Count ${pages.length} >>`,
    ...pages.flatMap((lines, index) => {
      const content = `BT /F1 14 Tf 72 720 Td ${lines.map((line) => `(${line}) Tj 0 -24 Td`).join(" ")} ET`;
      return [
        `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents ${4 + index * 2} 0 R /Resources << /Font << /F1 ${fontNumber} 0 R >> >> >>`,
        `<< /Length ${content.length} >>\nstream\n${content}\nendstream`,
      ];
    }),
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  const offsets: number[] = [];
  const body = objects.reduce((text, object, index) => {
    offsets.push(text.length);
    return `${text}${index + 1} 0 obj\n${object}\nendobj\n`;
  }, "%PDF-1.4\n");
  const xref = [
    `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`,
    ...offsets.map((offset) => `${String(offset).padStart(10, "0")} 00000 n \n`),
    `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${body.length}\n%%EOF\n`,
  ];
  return encoder.encode(body + xref.join(""));
}
