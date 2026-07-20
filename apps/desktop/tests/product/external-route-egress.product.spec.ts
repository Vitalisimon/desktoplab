import { expect, test } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { desktopOnly, localApi, openWorkspaceThroughUi } from "./auditHelpers";

test("24.9 product: external route cannot send attached context before explicit approval", async ({
  page,
  request,
}, testInfo) => {
  desktopOnly(testInfo);
  const workspaceRoot = await openWorkspaceThroughUi(page, request);
  const workspaceId = `workspace.${path.basename(workspaceRoot)}`;
  const started = await localApi(request, "POST", "/v1/provider-bridges/openai-codex/pairing/start", {
    accountMode: "subscription_account",
    stateSeed: "external-route-egress-product",
  });
  writeCodexCredential(started.pairingId);
  await localApi(request, "POST", "/v1/provider-bridges/openai-codex/pairing/complete", {
    pairingId: started.pairingId,
    pairingCode: started.pairingCode,
    bridgeInstanceId: "desktoplab-product-smoke",
    providerAccountLabel: "Product smoke Codex",
    localCredentialRef: `vault://desktoplab/external-backend/openai-codex/${started.pairingId}`,
    responderUrl: "http://127.0.0.1:1421",
  });
  await localApi(request, "POST", "/v1/routing/options/selection", { routeId: "route.external.codex" });

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Selected model Codex" })).toBeVisible();

  const attachment = path.join(workspaceRoot, "external-notes.txt");
  writeFileSync(attachment, "external context should require approval");
  await page.getByLabel("Choose external files").setInputFiles(attachment);
  await expect(page.getByRole("button", { name: "Attach external files, 1 attached" })).toBeVisible();
  await page.getByRole("textbox", { name: "Prompt" }).fill("Use the attached context with Codex");
  await page.getByRole("button", { name: "Send prompt" }).click();

  await expect(page.getByRole("group", { name: "External route approval" })).toContainText(
    "Send attached context to the external route?",
  );
  await expect(page.getByText("agent loop completed")).toHaveCount(0);
  const approvals = await localApi(request, "GET", "/v1/approvals");
  expect(approvals.approvals).toContainEqual(
    expect.objectContaining({
      action: "provider.egress",
      state: "pending",
      operationId: `provider.openai:route.external.codex:${workspaceId}`,
    }),
  );
});

function writeCodexCredential(pairingId: string) {
  const dir = path.join(tmpdir(), "desktoplab-local-provider-bridge", "openai-codex");
  mkdirSync(dir, { recursive: true });
  writeFileSync(path.join(dir, `${pairingId}.json`), JSON.stringify({ refresh_token: "test-redacted" }));
}
