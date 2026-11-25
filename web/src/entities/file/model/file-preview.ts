export type FilePreview = "pdf" | "image" | "page" | "markdown" | "text" | "none";

const sourceTypes = new Set(["application/xml", "text/xml"]);

export function filePreview(mime: string): FilePreview {
  if (mime === "application/pdf") {
    return "pdf";
  }
  if (mime === "text/html") {
    return "page";
  }
  if (mime === "text/markdown") {
    return "markdown";
  }
  if (mime.startsWith("image/")) {
    return "image";
  }
  if (mime.startsWith("text/") || sourceTypes.has(mime)) {
    return "text";
  }
  return "none";
}
