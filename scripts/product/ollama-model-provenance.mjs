import { readFileSync, realpathSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { join, relative } from "node:path";
import { DatabaseSync } from "node:sqlite";

export function localModelProvenance(statePath, modelsRoot = process.env.OLLAMA_MODELS ?? join(homedir(), ".ollama/models")) {
  const database = new DatabaseSync(statePath, { readOnly: true });
  let readiness;
  try {
    const row = database.prepare("select payload from productization_state where kind = ? and subject_id = ?").get("backend_readiness", "local");
    readiness = row?.payload ? JSON.parse(row.payload) : null;
  } finally { database.close(); }
  const modelId = readiness?.modelCapabilities?.modelId;
  if (!modelId) throw new Error("verified local model identity missing");
  const manifest = JSON.parse(readFileSync(ollamaManifestPath(modelId, modelsRoot), "utf8"));
  const modelLayer = manifest.layers?.find((layer) => layer.mediaType === "application/vnd.ollama.image.model");
  const configDigest = manifest.config?.digest;
  if (!validDigest(modelLayer?.digest) || !validDigest(configDigest)) throw new Error("local model manifest is invalid");
  const modelBlob = blobPath(modelsRoot, modelLayer.digest);
  if (statSync(modelBlob).size !== modelLayer.size) throw new Error("local model blob size differs from its manifest");
  const quantization = JSON.parse(readFileSync(blobPath(modelsRoot, configDigest), "utf8")).file_type;
  if (!quantization) throw new Error("verified local model quantization missing");
  return { modelId, quantization, digest: modelLayer.digest.toLowerCase() };
}

function ollamaManifestPath(modelId, modelsRoot) {
  if (!/^[a-zA-Z0-9._:/-]+$/.test(modelId) || modelId.includes("..")) throw new Error("verified local model id is invalid");
  const lastSlash = modelId.lastIndexOf("/");
  const tagSeparator = modelId.lastIndexOf(":");
  const tagged = tagSeparator > lastSlash;
  const name = tagged ? modelId.slice(0, tagSeparator) : modelId;
  const tag = tagged ? modelId.slice(tagSeparator + 1) : "latest";
  const parts = name.split("/");
  const registry = parts.length > 1 && parts[0]?.includes(".") ? parts.shift() : "registry.ollama.ai";
  if (parts.length === 1) parts.unshift("library");
  const root = realpathSync(join(modelsRoot, "manifests"));
  const path = realpathSync(join(root, registry, ...parts, tag));
  if (relative(root, path).startsWith("..")) throw new Error("local model manifest escapes Ollama storage");
  return path;
}

function blobPath(modelsRoot, value) {
  return realpathSync(join(modelsRoot, "blobs", value.replace(":", "-")));
}

function validDigest(value) { return /^sha256:[a-f0-9]{64}$/i.test(value ?? ""); }
