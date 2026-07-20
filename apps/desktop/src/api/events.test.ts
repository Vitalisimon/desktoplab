import {
  BackendEventClient,
  eventReplayPath,
  terminalEventsFromFrames,
  terminalResponseFromFrames,
  type BackendEventFrame,
} from "./events";
import type { ApiTransport, TransportRequest } from "./transport";

test("tracks replay cursor and ignores duplicate event frames", async () => {
  const requests: TransportRequest[] = [];
  const client = new BackendEventClient(
    {
      async request(request) {
        requests.push(request);
        return {
          status: 200,
          body: {
            frames: [
              frame(1, "setup", "created"),
              frame(2, "setup", "running"),
              frame(2, "setup", "running"),
            ],
          },
        };
      },
    } satisfies ApiTransport,
    "local-test-token",
  );

  await client.replay();
  await client.replay();

  expect(client.snapshot().map((event) => event.sequence)).toEqual([1, 2]);
  expect(requests[0].path).toBe("/v1/events/replay?after_sequence=0");
  expect(requests[1].path).toBe("/v1/events/replay?after_sequence=2");
});

test("keeps event memory bounded", () => {
  const client = new BackendEventClient({ async request() { return { status: 200, body: { frames: [] } }; } }, "token", 2);

  client.ingest([frame(1, "job", "one"), frame(2, "job", "two"), frame(3, "job", "three")]);

  expect(client.snapshot().map((event) => event.sequence)).toEqual([2, 3]);
});

test("does not send a bearer header when no local api token is configured", async () => {
  const requests: TransportRequest[] = [];
  const client = new BackendEventClient(
    {
      async request(request) {
        requests.push(request);
        return { status: 200, body: { frames: [] } };
      },
    },
    "",
  );

  await client.replay();

  expect(requests[0].headers.authorization).toBeUndefined();
});

test("builds replay path from the last seen sequence", () => {
  expect(eventReplayPath(42)).toBe("/v1/events/replay?after_sequence=42");
  expect(eventReplayPath(42, "stream one")).toBe("/v1/events/replay?after_sequence=42&stream_id=stream%20one");
});

test("paginates replay and resets false continuity when the stream changes", async () => {
  const requests: TransportRequest[] = [];
  const responses = [
    replay("stream.1", [frame(1, "session", "one")], true),
    replay("stream.1", [frame(2, "session", "two")], false),
    { ...replay("stream.2", [frame(1, "session", "replacement")], false), resetRequired: true },
  ];
  const client = new BackendEventClient({
    async request(request) {
      requests.push(request);
      return { status: 200, body: responses.shift() };
    },
  }, "token");

  await client.replay();
  expect(client.snapshot().map((event) => event.payload)).toEqual(["one", "two"]);
  await client.replay();
  expect(client.snapshot().map((event) => event.payload)).toEqual(["replacement"]);
  expect(requests[1].path).toContain("stream_id=stream.1");
});

test("exposes a detected retention gap instead of claiming continuous replay", async () => {
  const client = new BackendEventClient({
    async request() {
      return { status: 200, body: { ...replay("stream.1", [frame(7, "job", "retained")], false), gapDetected: true } };
    },
  }, "token");

  await client.replay();

  expect(client.hasDetectedGap()).toBe(true);
  expect(client.snapshot().map((event) => event.sequence)).toEqual([7]);
});

test("extracts terminal events from replay frames without duplicating lines", () => {
  const frames = [
    frame(4, "terminal", JSON.stringify(terminalPayload("event.1", "terminal.output", "token=[REDACTED]"))),
    frame(5, "terminal", JSON.stringify(terminalPayload("event.1", "terminal.output", "token=[REDACTED]"))),
    frame(6, "job", JSON.stringify(terminalPayload("event.2", "terminal.output", "ignored"))),
  ];

  expect(terminalEventsFromFrames(frames, "terminal.local")).toEqual([
    {
      eventId: "event.1",
      kind: "output",
      stdout: "token=[REDACTED]",
      stderr: "",
      status: "exited",
      exitCode: 0,
      stdoutTruncated: false,
      redacted: true,
    },
  ]);
});

test("reconstructs pending terminal approval from replay frames", () => {
  const response = terminalResponseFromFrames([
    frame(
      7,
      "terminal",
      JSON.stringify({
        terminalId: "terminal.local",
        kind: "terminal.approval_required",
        workspaceId: "workspace.desktoplab",
        command: "printf pending",
        cwd: ".",
        approvalState: "pending",
        approvalId: "approval.7",
      }),
    ),
  ]);

  expect(response?.state).toBe("approval_required");
  expect(response?.approval.approvalId).toBe("approval.7");
  expect(response?.command).toBe("printf pending");
});

test("replay fallback rejects instead of masking event replay failure", async () => {
  const fallback = [frame(9, "terminal", JSON.stringify(terminalPayload("event.9", "terminal.output", "cached")))];
  const client = new BackendEventClient(
    {
      async request() {
        return { status: 503, body: {} };
      },
    },
    "token",
  );

  await expect(client.replayOrFallback(() => fallback)).rejects.toThrow("event replay failed");
  expect(client.snapshot()).toEqual([]);
});

function frame(sequence: number, scope: BackendEventFrame["scope"], payload: string): BackendEventFrame {
  return { sequence, scope, payload };
}

function replay(streamId: string, frames: BackendEventFrame[], hasMore: boolean) {
  return {
    streamId,
    oldestSequence: frames.at(0)?.sequence ?? null,
    latestSequence: frames.at(-1)?.sequence ?? 0,
    nextSequence: frames.at(-1)?.sequence ?? 0,
    hasMore,
    gapDetected: false,
    resetRequired: false,
    frames,
  };
}

function terminalPayload(eventId: string, kind: string, stdout: string) {
  return {
    terminalId: "terminal.local",
    eventId,
    kind,
    stdout,
    stderr: "",
    status: "exited",
    exitCode: 0,
    stdoutTruncated: false,
    redacted: stdout.includes("[REDACTED]"),
  };
}
