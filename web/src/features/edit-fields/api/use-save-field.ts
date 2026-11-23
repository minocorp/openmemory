import { useMutation, useQueryClient } from "@tanstack/react-query";

import {
  applySavedPage,
  getPage,
  overwritePage,
  removeField,
  setField,
  splitContent,
  withHead,
  type Page,
} from "@/entities/page";
import { describeError, isApiError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

import { sameValues, valuesOf } from "../model/field-values";

type FieldEdit = { page: Page; key: string; values: string[] };

class FieldConflictError extends Error {
  readonly current: Page;

  constructor(current: Page) {
    super("field changed elsewhere");
    this.name = "FieldConflictError";
    this.current = current;
  }
}

function write(base: Page, key: string, values: string[]) {
  const { head } = splitContent(base);
  const nextHead = values.length === 0 ? removeField(head, key) : setField(head, key, values);
  return overwritePage({
    id: base.id,
    version: base.version,
    content: withHead(base, nextHead),
  });
}

async function saveField({ page, key, values }: FieldEdit): Promise<Page> {
  try {
    return await write(page, key, values);
  } catch (error) {
    if (!isApiError(error, 412)) {
      throw error;
    }
    const current = await getPage(page.id);
    if (sameValues(valuesOf(current, key), valuesOf(page, key))) {
      return write(current, key, values);
    }
    throw new FieldConflictError(current);
  }
}

export function useSaveField() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: saveField,
    onSuccess: (saved) => applySavedPage(queryClient, saved),
    onError: (error, edit) => {
      if (error instanceof FieldConflictError) {
        applySavedPage(queryClient, error.current);
        toast.error(`"${edit.key}" was changed elsewhere. Showing the latest value.`);
        return;
      }
      toast.error(describeError(error));
    },
  });
}
