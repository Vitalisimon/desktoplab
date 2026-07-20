import { expect, type APIRequestContext } from "@playwright/test";
import { localApi } from "./localApiTestClient";

export type SetupModelSelection = {
  modelId: string;
  runtimeId: string;
  displayName: string;
};

export async function selectSetup(request: APIRequestContext) {
  const selection = await recommendedSetupModel(request);
  await localApi(request, "POST", "/v1/setup/accept", {
    runtimeId: selection.runtimeId,
    modelId: selection.modelId,
  });
  return selection;
}

export async function markSetupReady(request: APIRequestContext) {
  const selection = await selectSetup(request);
  await localApi(request, "POST", `/v1/runtimes/${selection.runtimeId}/verify`, {});
  await localApi(request, "POST", `/v1/models/${selection.modelId}/download`, {
    setupAccepted: true,
    setupChoice: "use_existing",
    networkAvailable: true,
    diskAvailableMb: 100_000,
  });
  await localApi(request, "POST", `/v1/models/${selection.modelId}/verify`, {});
  await localApi(request, "POST", "/v1/test/model-protocol", { modelId: selection.modelId });
  await localApi(request, "POST", "/v1/setup/complete", {});
  return selection;
}

export async function recommendedSetupModel(request: APIRequestContext): Promise<SetupModelSelection> {
  const preview = await localApi(request, "GET", "/v1/setup/preview");
  const candidates = preview.modelRecommendations.filter(
    (model: { runtimeId?: string }) => model.runtimeId === "runtime.ollama",
  );
  const selected = candidates.find(
    (model: { hostInstallState?: string }) => model.hostInstallState === "installed",
  ) ?? candidates.find(
    (model: { role?: string }) => model.role === "recommended",
  ) ?? candidates[0];
  expect(selected, "setup preview should expose an Ollama model for this host").toBeTruthy();
  expect(selected.manifestId).toMatch(/^model\./);
  return {
    modelId: selected.manifestId,
    runtimeId: selected.runtimeId,
    displayName: selected.displayName,
  };
}
