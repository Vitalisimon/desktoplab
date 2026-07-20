import { authHeaders } from "./auth";
import type { ApiTransport } from "./transport";
export { terminalEventsFromFrames, terminalResponseFromFrames } from "./terminalEvents";
export { agentConversationEventsFromFrames } from "./agentEvents";
export type { AgentConversationEvent } from "./agentEvents";

export type BackendEventScope = "job" | "session" | "approval" | "setup" | "terminal";

export type BackendEventFrame = {
  sequence: number;
  scope: BackendEventScope;
  payload: string;
};

export type EventReplayResponse = {
  streamId: string;
  oldestSequence: number | null;
  latestSequence: number;
  nextSequence: number;
  hasMore: boolean;
  gapDetected: boolean;
  resetRequired: boolean;
  frames: BackendEventFrame[];
};

export class BackendEventClient {
  private readonly seenSequences = new Set<number>();
  private readonly frames: BackendEventFrame[] = [];
  private streamId = "";
  private gapDetected = false;

  constructor(
    private readonly transport: ApiTransport,
    private readonly authToken: string,
    private readonly maxFrames = 256,
  ) {}

  lastSequence() {
    return this.frames.at(-1)?.sequence ?? 0;
  }

  snapshot() {
    return [...this.frames];
  }

  ingest(frames: BackendEventFrame[]) {
    for (const frame of frames) {
      if (this.seenSequences.has(frame.sequence)) continue;
      this.seenSequences.add(frame.sequence);
      this.frames.push(frame);
    }
    while (this.frames.length > this.maxFrames) {
      const removed = this.frames.shift();
      if (removed) this.seenSequences.delete(removed.sequence);
    }
  }

  async replay() {
    for (let page = 0; page < 16; page += 1) {
      const cursor = this.lastSequence();
      const response = await this.transport.request({
        method: "GET",
        path: eventReplayPath(cursor, this.streamId),
        headers: authHeaders(this.authToken),
      });
      if (response.status >= 400) {
        throw new Error(`event replay failed with status ${response.status}`);
      }
      const replay = normalizedReplay(response.body);
      const streamChanged = this.streamId.length > 0
        && replay.streamId.length > 0
        && replay.streamId !== this.streamId;
      if (streamChanged || replay.resetRequired || replay.gapDetected) {
        this.clear();
      }
      this.gapDetected ||= replay.gapDetected;
      if (replay.streamId.length > 0) this.streamId = replay.streamId;
      this.ingest(replay.frames);
      if (!replay.hasMore || replay.frames.length === 0) break;
    }
    return this.snapshot();
  }

  hasDetectedGap() {
    return this.gapDetected;
  }

  async replayOrFallback(fallback: () => BackendEventFrame[] | Promise<BackendEventFrame[]>) {
    try {
      return await this.replay();
    } catch (error) {
      await fallback();
      throw error;
    }
  }

  private clear() {
    this.seenSequences.clear();
    this.frames.splice(0, this.frames.length);
  }
}

export function eventReplayPath(afterSequence: number, streamId = "") {
  const stream = streamId ? `&stream_id=${encodeURIComponent(streamId)}` : "";
  return `/v1/events/replay?after_sequence=${afterSequence}${stream}`;
}

function normalizedReplay(body: unknown): EventReplayResponse {
  const value = body && typeof body === "object" ? body as Partial<EventReplayResponse> : {};
  return {
    streamId: typeof value.streamId === "string" ? value.streamId : "",
    oldestSequence: typeof value.oldestSequence === "number" ? value.oldestSequence : null,
    latestSequence: typeof value.latestSequence === "number" ? value.latestSequence : 0,
    nextSequence: typeof value.nextSequence === "number" ? value.nextSequence : 0,
    hasMore: value.hasMore === true,
    gapDetected: value.gapDetected === true,
    resetRequired: value.resetRequired === true,
    frames: Array.isArray(value.frames) ? value.frames : [],
  };
}
