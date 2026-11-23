import { useQuery } from "@tanstack/react-query";
import { keepPreviousData } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import {
  flattenResults,
  isSearchable,
  parseSearchInput,
  searchQuery,
  type SearchResult,
} from "@/entities/search";
import { describeError } from "@/shared/api";
import { useDebouncedValue } from "@/shared/lib/debounced-value";
import {
  CommandFooter,
  CommandGroup,
  CommandInput,
  CommandList,
  CommandMessage,
} from "@/shared/ui/command";

import { debounceMs, listboxId, optionId, syntaxExamples } from "../config/palette";
import { nextIndex } from "../model/next-index";
import { ResultItem } from "./result-item";

type PaletteBodyProps = {
  onClose: () => void;
};

export function PaletteBody({ onClose }: PaletteBodyProps) {
  const navigate = useNavigate();
  const [raw, setRaw] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const input = parseSearchInput(useDebouncedValue(raw, debounceMs));
  const canSearch = isSearchable(input);
  const search = useQuery({
    ...searchQuery(input),
    enabled: canSearch,
    placeholderData: keepPreviousData,
  });
  const results = canSearch && search.data ? search.data.results : { pages: [], files: [] };
  const flat = flattenResults(results);

  useEffect(() => {
    document.getElementById(optionId(activeIndex))?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  const openResult = (result: SearchResult) => {
    onClose();
    void navigate({
      to: "/p/$",
      params: { _splat: result.path },
      search: result.filePage === null ? {} : { fpage: result.filePage },
    });
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.nativeEvent.isComposing) {
      return;
    }
    const move = nextIndex(event.key, activeIndex, flat.length);
    if (move !== null) {
      event.preventDefault();
      setActiveIndex(move);
      return;
    }
    const active = flat[activeIndex];
    if (event.key === "Enter" && active) {
      event.preventDefault();
      openResult(active);
    }
  };

  const itemFor = (result: SearchResult) => {
    const index = flat.indexOf(result);
    return (
      <ResultItem
        key={result.key}
        result={result}
        index={index}
        isActive={index === activeIndex}
        onHover={() => setActiveIndex(index)}
        onOpen={() => openResult(result)}
      />
    );
  };

  return (
    <>
      <CommandInput
        aria-label="Search pages and files"
        aria-controls={listboxId}
        aria-activedescendant={flat.length > 0 ? optionId(activeIndex) : undefined}
        placeholder="Search pages, files and fields"
        value={raw}
        onChange={(event) => {
          setRaw(event.target.value);
          setActiveIndex(0);
        }}
        onKeyDown={handleKeyDown}
        action={
          <button
            type="button"
            onClick={onClose}
            className="-mr-2 hidden h-9 shrink-0 rounded-md px-2 text-[13px] text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 max-sm:block"
          >
            Cancel
          </button>
        }
      />
      <CommandList id={listboxId}>
        {results.pages.length > 0 ? (
          <CommandGroup heading="Pages">{results.pages.map(itemFor)}</CommandGroup>
        ) : null}
        {results.files.length > 0 ? (
          <CommandGroup heading="Files">{results.files.map(itemFor)}</CommandGroup>
        ) : null}
        <PaletteStatus
          isSearchable={canSearch}
          isError={search.isError}
          error={search.error}
          isPending={search.isFetching && flat.length === 0}
          noHits={search.data?.noHits ?? null}
          hasResults={flat.length > 0}
        />
      </CommandList>
      <CommandFooter className="flex gap-4 max-sm:hidden">
        <span>↑↓ move</span>
        <span>↵ open</span>
        <span>esc close</span>
      </CommandFooter>
    </>
  );
}

type PaletteStatusProps = {
  isSearchable: boolean;
  isError: boolean;
  error: unknown;
  isPending: boolean;
  noHits: { query: string; fields_in_scope: string[]; folders_out_of_scope: number } | null;
  hasResults: boolean;
};

function PaletteStatus({
  isSearchable: canSearch,
  isError,
  error,
  isPending,
  noHits,
  hasResults,
}: PaletteStatusProps) {
  if (!canSearch) {
    return (
      <CommandMessage className="text-left">
        <p>Text, a path, or a field condition:</p>
        <p className="mt-2 flex flex-wrap gap-1.5">
          {syntaxExamples.map((example) => (
            <code key={example} className="rounded border px-1.5 py-0.5 font-mono text-xs">
              {example}
            </code>
          ))}
        </p>
      </CommandMessage>
    );
  }
  if (isError) {
    return <CommandMessage role="alert">{describeError(error)}</CommandMessage>;
  }
  if (isPending) {
    return <CommandMessage>Searching</CommandMessage>;
  }
  if (hasResults || !noHits) {
    return null;
  }
  return (
    <CommandMessage className="text-left">
      <p>No results for {noHits.query}</p>
      {noHits.fields_in_scope.length > 0 ? (
        <p className="mt-1.5">Fields: {noHits.fields_in_scope.join(", ")}</p>
      ) : null}
      {noHits.folders_out_of_scope > 0 ? (
        <p className="mt-1.5">{noHits.folders_out_of_scope} folders out of scope</p>
      ) : null}
    </CommandMessage>
  );
}
