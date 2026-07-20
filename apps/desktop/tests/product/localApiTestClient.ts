import { expect, type APIRequestContext } from "@playwright/test";

export const apiBase = "http://127.0.0.1:1421";

export async function localApi(
  request: APIRequestContext,
  method: "GET" | "POST",
  route: string,
  body?: unknown,
  expectedStatus = 200,
) {
  const response = await fetchWithRetry(request, method, route, body);
  const failureBody = response.status() === expectedStatus ? "" : await response.text().catch(() => "<unreadable body>");
  expect(response.status(), `${method} ${route} status: ${failureBody}`).toBe(expectedStatus);
  return response.json();
}

export async function resetProductState(request: APIRequestContext) {
  const response = await fetchWithRetry(request, "POST", "/v1/test/reset", {});
  if (response.status() === 200) return response.json();
  const body = await response.json().catch(() => ({}));
  if (response.status() === 400 || response.status() === 404) return body;
  expect(response.status(), `POST /v1/test/reset status: ${JSON.stringify(body)}`).toBe(200);
  return body;
}

async function fetchWithRetry(request: APIRequestContext, method: "GET" | "POST", route: string, body?: unknown) {
  let lastError: unknown = null;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const response = await request.fetch(`${apiBase}${route}`, { method, data: body });
      if (response.status() === 400 && attempt < 2 && isTransientMalformedHttpRequest(await response.text())) {
        await new Promise((resolve) => setTimeout(resolve, 150));
        continue;
      }
      return response;
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 150));
    }
  }
  throw lastError;
}

function isTransientMalformedHttpRequest(body: string) {
  try {
    const parsed = JSON.parse(body);
    return parsed?.code === "BAD_REQUEST" && parsed?.message === "malformed http request";
  } catch {
    return false;
  }
}
