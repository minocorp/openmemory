import { describe, expect, it } from "vitest";

import { ApiError } from "@/shared/api";

import { describeUploadError, fileMime } from "./upload-problem";

describe("describeUploadError", () => {
  it("names the secret pattern of a rejected text", () => {
    const body = { error: "credential", pattern: "private_key" };
    expect(describeUploadError(new ApiError(422, body), body)).toContain("private_key");
  });

  it("states the size limit", () => {
    const body = { error: "too_large", limit: 52_428_800 };
    expect(describeUploadError(new ApiError(413, body), body)).toBe(
      "The file is larger than 50 MB.",
    );
  });

  it("explains a taken path and an unsupported type", () => {
    expect(describeUploadError(new ApiError(409, { error: "path_taken" }), null)).toContain(
      "already exists",
    );
    expect(describeUploadError(new ApiError(415, { error: "unsupported_type" }), null)).toContain(
      "not accepted",
    );
  });
});

describe("fileMime", () => {
  it("falls back to octet-stream when the browser does not know the type", () => {
    expect(fileMime({ type: "" })).toBe("application/octet-stream");
    expect(fileMime({ type: "application/pdf" })).toBe("application/pdf");
  });
});
