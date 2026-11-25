import { describe, expect, it } from "vitest";

import { filePreview } from "./file-preview";

describe("filePreview", () => {
  it("renders html as a page and xml as source text", () => {
    expect(filePreview("text/html")).toBe("page");
    expect(filePreview("application/xml")).toBe("text");
    expect(filePreview("text/xml")).toBe("text");
    expect(filePreview("text/markdown")).toBe("text");
  });

  it("shows svg as an image and has no preview for documents", () => {
    expect(filePreview("image/svg+xml")).toBe("image");
    expect(filePreview("application/pdf")).toBe("pdf");
    expect(
      filePreview("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
    ).toBe("none");
  });
});
