export function authHeaders(authToken: string): Record<string, string> {
  const token = authToken.trim();
  return token.length > 0 ? { authorization: `Bearer ${token}` } : {};
}

export function backendErrorMessage(body: unknown): string {
  const message =
    body && typeof body === "object" && "message" in body
      ? (body as { message?: unknown }).message
      : null;
  if (typeof message === "string" && message.trim()) return message;
  return "Local API returned an error";
}
