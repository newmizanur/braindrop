import type { ApiError } from "../bindings/ApiError";
import { getToken } from "../auth/token";

export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

export class ApiClientError extends Error {
  readonly status: number;
  readonly body: ApiError | null;

  constructor(status: number, body: ApiError | null) {
    super(body?.message ?? `Request failed with status ${status}`);
    this.name = "ApiClientError";
    this.status = status;
    this.body = body;
  }
}

/**
 * Accept both our typed `{ code, message }` errors and Loco's default
 * `{ error, description }` shape. Without the second shape, a 403 such as
 * "email verification required" would reach the user as a generic failure.
 */
async function parseErrorBody(res: Response): Promise<ApiError | null> {
  try {
    const data: unknown = await res.json();
    if (typeof data !== "object" || data === null) {
      return null;
    }

    const record = data as Record<string, unknown>;
    if (typeof record.code === "string" && typeof record.message === "string") {
      return {
        code: record.code,
        message: record.message,
        details: record.details ?? null,
      };
    }

    if (typeof record.error === "string") {
      return {
        code: record.error,
        message:
          typeof record.description === "string"
            ? record.description
            : record.error,
        details: record.errors ?? null,
      };
    }

    return null;
  } catch {
    return null;
  }
}

export async function request<T>(
  method: HttpMethod,
  path: string,
  body?: unknown,
): Promise<T> {
  const headers: Record<string, string> = {};
  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
  }
  const token = getToken();
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const res = await fetch(path, {
    method,
    headers,
    credentials: "same-origin",
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (res.ok) {
    return res.status === 204 ? (undefined as T) : ((await res.json()) as T);
  }

  const errorBody = await parseErrorBody(res);

  // Session recovery belongs to the route guard. A hard redirect here would
  // reload the page on a failed login attempt and lose the form state.
  throw new ApiClientError(res.status, errorBody);
}

export function get<T>(path: string): Promise<T> {
  return request<T>("GET", path);
}

export function post<T>(path: string, body: unknown): Promise<T> {
  return request<T>("POST", path, body);
}

export function put<T>(path: string, body: unknown): Promise<T> {
  return request<T>("PUT", path, body);
}

export function del(path: string): Promise<void> {
  return request<void>("DELETE", path);
}
