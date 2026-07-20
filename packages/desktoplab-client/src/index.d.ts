export const SDK_VERSION: "1";

export interface TransportRequest { method: "GET" | "POST"; path: string; headers: Record<string, string>; body?: unknown }
export interface TransportResponse { status: number; body: any }
export interface ClientTransport { request(request: TransportRequest): Promise<TransportResponse> }
export interface ExternalAttachmentInput {
  name: string;
  size: number;
  mediaType: string;
  contentText?: string;
  truncated?: boolean;
}
export interface AgentRunRequest {
  workspaceId: string;
  executionBackendId: string;
  prompt: string;
  sessionId?: string;
  contextPaths?: string[];
  externalAttachments?: ExternalAttachmentInput[];
  approvalId?: string;
  newChat?: boolean;
}
export interface AgentRunResult<T = any> {
  kind: "desktoplab.agent-run-result";
  schemaVersion: 1;
  sessionId: string | null;
  workspaceId: string | null;
  state: string;
  session: T;
}
export interface WaitRequest { sessionId: string; workspaceId: string; timeoutMs?: number; pollIntervalMs?: number }
export interface DesktopLabAgentClientOptions { authToken: string; transport: ClientTransport }

export class DesktopLabAgentClient {
  constructor(options: DesktopLabAgentClientOptions);
  run<T = any>(request: AgentRunRequest): Promise<AgentRunResult<T>>;
  stream<T = any>(request: AgentRunRequest): Promise<AgentRunResult<T>>;
  wait<T = any>(request: WaitRequest): Promise<AgentRunResult<T>>;
  cancel<T = any>(sessionId: string): Promise<AgentRunResult<T>>;
  modelStatus<T = any>(): Promise<T & { kind: "desktoplab.model-status"; schemaVersion: 1 }>;
}
export class FetchTransport implements ClientTransport {
  constructor(baseUrl: string);
  request(request: TransportRequest): Promise<TransportResponse>;
}
