import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export function additionBehaviorMatches(implementationPath, timeoutMs = 60_000) {
  const moduleUrl = pathToFileURL(implementationPath).href;
  const probe = [
    `const { add } = await import(${JSON.stringify(moduleUrl)});`,
    "const cases = [[2, 3, 5], [-5, 3, -2], [7, 0, 7], [1.5, 2.25, 3.75]];",
    "if (typeof add !== 'function' || cases.some(([left, right, expected]) => add(left, right) !== expected)) process.exit(1);",
  ].join("\n");
  const result = spawnSync(process.execPath, ["--input-type=module", "--eval", probe], {
    encoding: "utf8",
    timeout: timeoutMs,
    maxBuffer: 1024 * 1024,
  });
  return result.status === 0;
}
