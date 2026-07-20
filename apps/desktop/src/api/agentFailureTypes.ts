export type AgentFailureClassification = {
  schemaVersion: 1;
  primary: string;
  findings: Array<{ code: string; message: string }>;
  originalStopReason: string | null;
  userMessage: string;
};
