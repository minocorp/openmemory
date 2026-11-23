import { ApiError } from "./api-error";

type ApiResult<Data> =
  | { data: Data; error?: never; response: Response }
  | { data?: never; error: unknown; response: Response };

export async function unwrap<Data>(request: Promise<ApiResult<Data>>): Promise<Data> {
  const result = await request;
  if (result.error !== undefined) {
    throw new ApiError(result.response.status, result.error);
  }
  if (result.data === undefined) {
    throw new ApiError(result.response.status, null);
  }
  return result.data;
}

export async function unwrapEmpty(
  request: Promise<{ error?: unknown; response: Response }>,
): Promise<void> {
  const result = await request;
  if (result.error !== undefined) {
    throw new ApiError(result.response.status, result.error);
  }
}
