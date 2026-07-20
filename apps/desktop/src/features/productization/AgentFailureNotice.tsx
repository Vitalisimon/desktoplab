import type { AgentSessionSnapshot } from "../../api/types";
import { MessageBlock } from "./ConversationEvent";

export function AgentFailureNotice({ session }: { session: AgentSessionSnapshot }) {
  if (session.state !== "failed" || !session.failureClassification?.userMessage) return null;
  return <MessageBlock body={session.failureClassification.userMessage} />;
}
