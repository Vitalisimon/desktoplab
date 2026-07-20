import { spawnSync } from "node:child_process";

export function observeProcessMemory(pid, platform = process.platform) {
  if (platform === "darwin") {
    const output = spawnSync("vmmap", ["-summary", String(pid)], { encoding: "utf8" }).stdout;
    const match = output.match(/^Physical footprint:\s+([0-9.]+[KMG])$/m);
    return match ? { measurement: "physical_footprint", observedMemoryKb: parseMemorySizeKb(match[1]) } : null;
  }
  const output = spawnSync("ps", ["-o", "rss=", "-p", String(pid)], { encoding: "utf8" }).stdout.trim();
  const observedMemoryKb = Number.parseInt(output || "0", 10);
  return observedMemoryKb > 0 ? { measurement: "rss", observedMemoryKb } : null;
}

export function parseMemorySizeKb(value) {
  const match = String(value).match(/^([0-9]+(?:\.[0-9]+)?)([KMG])$/);
  if (!match) return 0;
  const multipliers = { K: 1, M: 1024, G: 1024 * 1024 };
  return Math.round(Number.parseFloat(match[1]) * multipliers[match[2]]);
}
