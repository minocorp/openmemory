import { describe, expect, it } from "vitest";

import {
  ancestorFolders,
  baseName,
  findPathProblem,
  joinPath,
  normalizePath,
  pagePathFromPathname,
  parentFolder,
  pathnameOfPagePath,
} from "./page-path";

describe("page path", () => {
  it("normalizes to NFC and trims slashes", () => {
    const decomposed = "customers/사";
    expect(normalizePath(`/${decomposed}/`)).toBe("customers/사");
  });

  it("splits a path into folder and name", () => {
    expect(baseName("customers/acme")).toBe("acme");
    expect(parentFolder("customers/acme")).toBe("customers");
    expect(parentFolder("start")).toBe("");
    expect(joinPath("", "start")).toBe("start");
    expect(joinPath("a/b", "c")).toBe("a/b/c");
  });

  it("lists every ancestor folder, outermost first", () => {
    expect(ancestorFolders("a/b/c")).toEqual(["a", "a/b"]);
    expect(ancestorFolders("a")).toEqual([]);
  });

  it("rejects empty segments, dot segments and control characters", () => {
    expect(findPathProblem("")).toBe("empty");
    expect(findPathProblem("a//b")).toBe("segment");
    expect(findPathProblem("a/../b")).toBe("segment");
    expect(findPathProblem("a/\u0007b")).toBe("character");
    expect(findPathProblem("customers/acme")).toBeNull();
  });

  it("treats a trailing slash as a missing name", () => {
    expect(findPathProblem("customers/")).toBe("segment");
  });
});

describe("page urls", () => {
  it("reads the page path from a browser pathname", () => {
    expect(pagePathFromPathname("/p/customers/OO%EC%83%81%EC%82%AC")).toBe("customers/OO상사");
    expect(pagePathFromPathname("/p/a%20b/c")).toBe("a b/c");
  });

  it("ignores pathnames outside the page routes", () => {
    expect(pagePathFromPathname("/login")).toBeNull();
    expect(pagePathFromPathname("/p/%E0%A4%A")).toBeNull();
  });

  it("round-trips through the encoded pathname", () => {
    const path = "customers/a b/100%/OO상사.pdf";
    expect(pagePathFromPathname(pathnameOfPagePath(path))).toBe(path);
  });
});
