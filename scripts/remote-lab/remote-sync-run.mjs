#!/usr/bin/env node
import { resolve } from "node:path";

import { SystemTargetTransport } from "./remote-target-contract.mjs";
import { syncSourceSnapshot, createSourceSnapshot } from "./remote-sync.mjs";
import { localTargetInventory } from "./remote-targets.mjs";

const [targetId, runId] = process.argv.slice(2);
if (!targetId || !runId) throw new Error("usage: remote-sync-run.mjs <target-id> <run-id>");
const target = localTargetInventory().find((candidate) => candidate.id === targetId);
if (!target) throw new Error("unknown remote target");
const environment = {
  ...process.env,
  DESKTOPLAB_N95_SSH_ENDPOINT: process.env.DESKTOPLAB_N95_SSH_ENDPOINT ?? `${process.env.N95_SSH_USER ?? "simone"}@${process.env.N95_SSH_HOST ?? ""}`,
  DESKTOPLAB_N95_SSH_IDENTITY: process.env.DESKTOPLAB_N95_SSH_IDENTITY ?? process.env.N95_SSH_KEY,
};
const snapshot = createSourceSnapshot(resolve("."));
const result = syncSourceSnapshot(snapshot, target, runId, new SystemTargetTransport(environment));
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
process.exitCode = result.status === "complete" ? 0 : 2;
