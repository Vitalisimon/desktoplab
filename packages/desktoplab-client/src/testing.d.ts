import type { ClientTransport, TransportRequest, TransportResponse } from "./index.mjs";
export class InMemoryTransport implements ClientTransport {
  constructor(options: { testOnly: true; responder(request: TransportRequest): TransportResponse | Promise<TransportResponse> });
  request(request: TransportRequest): Promise<TransportResponse>;
  requests(): TransportRequest[];
}
