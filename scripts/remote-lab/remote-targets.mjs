#!/usr/bin/env node
import os from "node:os";
import { fileURLToPath } from "node:url";

import { RemoteTargetController, SystemTargetTransport } from "./remote-target-contract.mjs";

export function localTargetInventory() {
  return [
    {
      id: "local-mac",
      kind: "local_owned",
      platform: "macos",
      architecture: process.arch === "arm64" ? "arm64" : "x64",
      shell: "posix",
      capabilities: ["build", "package", "visual", "agent"],
      trustLevel: "local_owner",
    },
    {
      id: "n95-linux",
      kind: "static_ssh",
      platform: "linux",
      architecture: "x64",
      shell: "posix",
      capabilities: ["build", "package", "agent", "xvfb"],
      trustLevel: "trusted_physical",
      endpointRef: "DESKTOPLAB_N95_SSH_ENDPOINT",
      credentialRef: "DESKTOPLAB_N95_SSH_IDENTITY",
    },
    {
      id: "nico-windows",
      kind: "static_ssh",
      platform: "windows",
      architecture: "x64",
      shell: "powershell",
      capabilities: ["build", "package", "visual", "agent", "test_signing"],
      trustLevel: "trusted_physical",
      endpointRef: "DESKTOPLAB_WINDOWS_SSH_ENDPOINT",
      credentialRef: "DESKTOPLAB_WINDOWS_SSH_IDENTITY",
    },
  ];
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const environment = {
    ...process.env,
    DESKTOPLAB_N95_SSH_ENDPOINT: process.env.DESKTOPLAB_N95_SSH_ENDPOINT ?? `${process.env.N95_SSH_USER ?? "simone"}@${process.env.N95_SSH_HOST ?? ""}`,
    DESKTOPLAB_N95_SSH_IDENTITY: process.env.DESKTOPLAB_N95_SSH_IDENTITY ?? process.env.N95_SSH_KEY,
  };
  const controller = new RemoteTargetController(localTargetInventory(), new SystemTargetTransport(environment));
  const requested = process.argv.slice(2);
  const targets = requested.length > 0 ? requested : localTargetInventory().map((target) => target.id);
  const results = targets.map((targetId) => controller.probe(targetId));
  process.stdout.write(`${JSON.stringify({ schemaVersion: 1, controllerHost: os.hostname(), results }, null, 2)}\n`);
  process.exitCode = results.every((result) => result.state === "available") ? 0 : 2;
}
