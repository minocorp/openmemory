import { useSuspenseQuery } from "@tanstack/react-query";

import { pathTargetQuery } from "../api/path-target-query";
import type { PageSearch } from "../model/search";
import { FileScreen } from "./file-screen";
import { MissingPath } from "./missing-path";
import { PageScreen } from "./page-screen";

type PathScreenProps = {
  path: string;
  search: PageSearch;
  onSearchChange: (next: PageSearch) => void;
};

export function PathScreen({ path, search, onSearchChange }: PathScreenProps) {
  const { data: target } = useSuspenseQuery(pathTargetQuery(path));
  if (target.kind === "page") {
    return (
      <PageScreen key={target.id} id={target.id} search={search} onSearchChange={onSearchChange} />
    );
  }
  if (target.kind === "file") {
    return <FileScreen id={target.id} search={search} onSearchChange={onSearchChange} />;
  }
  return <MissingPath path={path} />;
}
