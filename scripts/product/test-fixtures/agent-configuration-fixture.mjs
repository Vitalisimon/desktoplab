export function agentConfiguration(overrides = {}) {
  return {
    model: { id: "qwen2.5-coder:7b", digest: sha256("1"), quantization: "Q4_K_M" },
    runtime: {
      id: "runtime.ollama",
      version: "0.6.2",
      backendId: "backend.ollama",
      backendVersion: "1",
    },
    toolSchemaDigest: sha256("a"),
    approvalMode: "require_approval",
    contextPlan: {
      digest: sha256("c"),
      budgetTokens: 32768,
      compactionPolicy: "bounded-summary-v1",
    },
    adaptivePolicy: {
      digest: sha256("9"),
      contextWindowTokens: 32768,
      requestTimeoutSeconds: 300,
      modelMaximumTokens: 262144,
    },
    plugins: [{ id: "plugin.acp", version: "1.0.0", digest: sha256("d") }],
    harnessVersion: "2.0.0",
    hostCapabilities: {
      os: "darwin",
      arch: "arm64",
      memoryClassGb: 64,
      acceleratorClass: "apple-m-series",
      gpuMemoryClassGb: null,
      unifiedMemory: true,
    },
    ...overrides,
  };
}

export function sha256(character) {
  return `sha256:${character.repeat(64)}`;
}
