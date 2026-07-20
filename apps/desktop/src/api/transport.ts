export type TransportRequest = {
  method: "GET" | "POST";
  path: string;
  headers: Record<string, string>;
  body?: unknown;
};

export type TransportResponse = {
  status: number;
  body: unknown;
};

export type ApiTransport = {
  request(request: TransportRequest): Promise<TransportResponse>;
};

export class FetchTransport implements ApiTransport {
  constructor(private readonly baseUrl: string) {}

  async request(request: TransportRequest): Promise<TransportResponse> {
    const response = await fetch(`${this.baseUrl}${request.path}`, {
      method: request.method,
      headers: {
        "content-type": "application/json",
        ...request.headers,
      },
      body: request.body === undefined ? undefined : JSON.stringify(request.body),
    });

    const text = await response.text();
    const body = text.length > 0 ? JSON.parse(text) : {};
    return { status: response.status, body };
  }
}
