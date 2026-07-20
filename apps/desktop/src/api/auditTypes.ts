export type AuditOutcome = "allowed" | "denied";

export type LocalAuditDecisionSummary = {
  sequence: number;
  action: string;
  outcome: AuditOutcome;
  redactedDetails: string;
};

export type LocalAuditTransparencySnapshot = {
  scope: "local_single_user";
  records: LocalAuditDecisionSummary[];
  redactedExport: string;
};
