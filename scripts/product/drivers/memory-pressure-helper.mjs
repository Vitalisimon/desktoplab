import { randomFillSync } from "node:crypto";

const requestedMb = Number.parseInt(process.argv[2] ?? "0", 10);
if (!Number.isInteger(requestedMb) || requestedMb < 1 || requestedMb > 2_048) throw new Error("memory pressure must be between 1 and 2048 MiB");
const allocation = Buffer.allocUnsafe(requestedMb * 1024 * 1024);
const chunkBytes = 16 * 1024 * 1024;
for (let offset = 0; offset < allocation.length; offset += chunkBytes) {
  randomFillSync(allocation, offset, Math.min(chunkBytes, allocation.length - offset));
}
process.stdout.write(`ready ${allocation.length}\n`);
setInterval(() => allocation[0] ^= 1, 1_000);
