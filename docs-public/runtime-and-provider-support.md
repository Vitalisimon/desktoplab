# Runtime And Provider Support

Status: scoped public claims frozen, no provider or model publicly certified
Date: 2026-07-16

DesktopLab is designed to support local runtimes, cloud providers and external agent backends through explicit capability contracts.

This page describes public claims only.

Public docs must not imply that a runtime, model family, provider or bridge is ready because an adapter contract exists. A claim becomes public only after certification evidence exists for the relevant platform and account mode.

## Claim Snapshot

| Surface | Public claim state |
| --- | --- |
| Setup and first prompt | Product regression evidence exists in development, but public support waits for the platform packaging gate. |
| Local runtime and model path | Not publicly claimed as supported beyond certified dev evidence. Platform-specific runtime/model and packaging evidence is required. |
| File drawer and terminal | Product regression evidence exists in development, but public support waits for the platform packaging gate. |
| Cloud providers | Native-vault API-key boundary exists in development; live cloud execution is not publicly claimed. |
| External app bridges and protocols | Future or blocked until sandbox, trust, account ownership and audit evidence exist. |
| High-end local inference | DesktopLab is architected for high-end local workflows. No DGX/custom-rig or frontier-local support claim exists until an exact host, runtime and model combination passes live certification. |
| Complete local coding agent | `private-installed-evidence-gated`. DesktopLab owns a persistent iterative model/tool/observation loop and real local tools, but public support requires fresh installed-app live evidence for the exact signed artifact, host, runtime and model. |

## Frontier-Local Claim Boundary

High-end local readiness uses evidence labels with deliberately different meanings:

| Label | Meaning |
| --- | --- |
| `architected` | Contracts and product boundaries account for the capability, but no host support is implied. |
| `detected` | Backend-owned probes measured the relevant host facts recently. |
| `configured` | A runtime, model and storage route is connected, but quality and installed-app behavior are not yet certified. |
| `certified` | The exact host, runtime and model combination passed the live frontier-local certification scope. |
| `blocked` | Required evidence or a required dependency is missing or stale. |
| `unsupported` | Measured capability is outside the declared compatibility envelope. |

The phrase `frontier-local capable` is reserved for a certified host, runtime and model combination. It is not a default claim about DesktopLab, a hardware brand, a parameter count or every local model. Public copy may say DesktopLab is `architected` for high-end local frontier workflows while certification remains blocked.

Certification keeps four evidence surfaces separate:

- model quality from live task outcomes;
- runtime performance and health on the measured host;
- repository-context retrieval quality, freshness and provenance;
- installed-app agent behavior across reads, edits, validation and approvals.

Repository RAG can improve grounding and context selection. It does not establish frontier-model reasoning quality, and a RAG result cannot substitute for live model-quality evidence.

## Local Runtimes

| Runtime | Current public claim | Certification required before support is advertised |
| --- | --- | --- |
| Ollama | Not publicly claimed as supported. Intended first local runtime path. | Platform-specific install, detect, start, health check, model download and readiness evidence. |
| LM Studio | Not publicly claimed as supported. Intended local runtime path. | Platform-specific detection, launch/bridge, model availability and readiness evidence. |
| llama.cpp | Not publicly claimed as supported. | Runtime adapter plus platform-specific install/start/verify evidence. |
| MLX | Not publicly claimed as supported. | Apple Silicon specific runtime evidence and compatible model catalog entries. |
| vLLM | Not publicly claimed as supported. | Host/GPU specific runtime evidence and compatible model catalog entries. |
| Future runtimes | Not publicly claimed as supported. | Signed/cataloged compatibility data and runtime adapter evidence. |

Runtime lifecycle claims are intentionally narrow:

- DesktopLab can use an existing local runtime when detection and health checks prove it is available.
- An existing local runtime remains user-owned unless DesktopLab started it and wrote its own ownership marker.
- DesktopLab does not stop user-owned runtime services when the app quits.
- DesktopLab may install a DesktopLab-managed runtime only when a verified adapter exists for the current platform.
- Runtime update and uninstall actions are not advertised as supported unless they have their own end-to-end evidence.
- DesktopLab application updates are separate from local runtime updates.
- Externally managed apps such as LM Studio remain guided setup paths until DesktopLab can verify their local endpoint; update and removal stay in the external app.
- DesktopLab must never request administrator access silently.
- Replace or reinstall actions must be explicit. They are never the default path when a working local runtime already exists.

## Models

DesktopLab does not hardcode model support as product truth.

Model compatibility is expected to come from catalog and compatibility data, so families such as Qwen, NVIDIA Nemotron, GLM, DeepSeek and future models can be added without redesigning the app.

For installed Ollama models, DesktopLab reads runtime-owned model metadata from
`/api/tags` and `/api/show`. The resulting profile records the model digest,
reported context window and declared capabilities. Profiles are cached by
digest and invalidated when the installed model changes. A declared `tools`
capability is necessary but not sufficient for agent routing: DesktopLab runs a
non-mutating, digest-scoped protocol canary and persists its result. The canary
distinguishes native `message.tool_calls` from exact constrained JSON returned
in assistant content. Both certified paths enter the canonical DesktopLab tool
executor; missing metadata, malformed output, prose-wrapped JSON or a failed
canary leaves the model on a chat-only route. A new digest requires a new
canary, while explicit model verification can rerun it for the same digest.

Runtime capability discovery is routing evidence, not model-quality
certification. Full coding-agent claims still require the live behavioral
evidence described below.

Public model claims require compatibility evidence for the selected runtime and platform.

Current public model claim: catalog-driven compatibility is a product direction, not a public promise that any specific model family is ready on every machine.

DesktopLab route labels must separate chat capability from agent capability:

| Capability class | Meaning | Public claim status |
| --- | --- | --- |
| `chat_capable` | The model can be used for local chat-style responses, but DesktopLab does not route it as a coding agent. | Not a coding-agent claim. |
| `limited_agent_capable` | DesktopLab can wrap the model with local tools, approvals and transcript controls, but live-local certification is incomplete or below the full threshold. | Private/dev only; UI must avoid full-agent wording. |
| `full_coding_agent_capable` | The installed app passed a real iterative read, create, patch, test, failure-repair, diff, commit-proposal and refusal suite for the exact model/host. | Currently blocked; advertisable only with measured installed-app live evidence. |

For every full-agent model claim, evidence must record model id, quantization, host class, live score and failing cases, if any. Models below threshold stay `chat_capable` or `limited_agent_capable`; they must not be described as comparable to Codex or Claude Code.

Before a model family is advertised, DesktopLab needs evidence for:

- selected runtime;
- model identifier and parameter/quantization variant;
- parameter count and approximate disk size shown to the user;
- visible license/trust state (`known`, `unknown` or `restricted`);
- host hardware class;
- download/verification behavior;
- readiness check;
- blocked reasons for unsupported hosts.

Models with unknown or restricted license state must not be promoted as the primary default recommendation. They may appear only with explicit trust copy so the user understands that compatibility and license review are separate concerns.

### Large-Model Classes

DesktopLab models 70B, 100B, 200B, 300B, 400B, 600B and 1T parameter classes as compatibility envelopes. A class is not a downloadable model and does not imply that weights exist under a usable license.

Every concrete large-model entry must record quantization, precision, context length, estimated memory and disk requirements, compatible runtime adapters, license id, commercial-use state, source and checksum provenance. It remains `blocked`, `research-needed` or `not-publicly-claimed` until those fields and fresh hardware evidence pass. Unknown-license or checksum-free weights cannot enter a recommended setup route.

## Cloud Providers And Bridges

| Provider or bridge | Claim state | Current public claim | Certification required before support is advertised |
| --- | --- | --- | --- |
| Local-only operation | `blocked` until packaging/platform gates pass | Not publicly claimed as release-ready yet. | Fresh setup, local API auth, setup-first shell, local workbench proof and platform package proof. |
| OpenAI API-key billing | `blocked` for cloud execution; native-vault credential boundary exists in development | Not publicly claimed as cloud-model supported. | Native vault storage, live account certification, egress policy and model/backend path proof. |
| OpenAI subscription account / Codex local bridge | `private-dev` but not public | Not publicly claimed as supported. Development builds require local bridge credential evidence, responder health and explicit repository-context egress approval. | Signed public package evidence, bridge certification, user-owned account flow evidence and capability boundary proof. |
| Anthropic API-key billing | `future` | Not publicly claimed as supported. | Live account certification for the claimed account mode and model/backend path. |
| Anthropic Agent SDK | `research-needed` | Not publicly claimed as supported. Official docs describe API-key based Python/TypeScript SDK execution with built-in file, command and edit tools; DesktopLab has not certified its auth, permissions, session or evidence boundary. | SDK auth boundary, DesktopLab-owned session mapping, tool/permission normalization, rollback evidence and live account certification. |
| Anthropic subscription or Claude app bridge | `future` | Not publicly claimed as supported. | Bridge certification, user-owned account flow evidence and capability boundary proof. |
| Gemini | `future` | Not publicly claimed as supported. | Live account certification for the claimed account mode and model/backend path. |
| OpenRouter | `future` | Not publicly claimed as supported. | Live account certification for the claimed account mode and model/backend path. |
| Codex-style external agent bridge | `private-dev` but not public | Not publicly claimed as supported. Current development evidence is scoped to the OpenAI/Codex local bridge path only. | Bridge certification, user-owned account flow evidence and capability boundary proof. |
| ACP execution backend | `future` | Not publicly claimed as supported. Current ACP docs describe editor-to-agent communication and local/remote transports, with remote support still maturing; DesktopLab has no certified ACP backend. | Plugin sandbox, trust policy, capability contract, protocol-version negotiation, transport policy and session-event proof. |
| Custom OpenAI-compatible endpoint | `blocked` until endpoint health and model execution are certified | Not publicly claimed as supported. | Endpoint configuration, auth, egress policy, model listing and failure behavior evidence. |

DesktopLab separates provider identity from execution backend.

Subscription-account access and API-billed access are different modes and must be described separately when they become public. A provider may be certified for one mode without being certified for the other. Codex-style and Claude-style bridges are account/app bridge modes, not API-token billing modes; until bridge discovery and ownership checks exist, these modes remain guided/future rather than clickable cloud execution.

No provider is advertised as ready when only adapter contracts exist.

Provider compatibility is selected by the negotiated wire protocol, not by
matching model names. Ollama chat and OpenAI-compatible chat-completions use
separate active profiles for message envelopes, tool-choice request fields and
reasoning fields. Both currently allow one canonical DesktopLab tool call per
turn. Multiple calls, missing tool names, non-object arguments and malformed or
concatenated argument JSON fail closed instead of being guessed, truncated or
shown as raw provider output.

No cloud provider is publicly certified. Provider credentials, adapter
contracts and development bridge evidence are not live account certification.

## ExecutionBackend Support Contracts

Every execution backend must declare who owns the model loop, canonical thread, tools, approvals, context, compaction and transcript mirror before DesktopLab can present it as usable.

DesktopLab owns the canonical session for all backend types. Local runtimes may own model execution, and external bridges may own the external model loop, but repository context, approval policy, transcript truth and tool execution remain DesktopLab-owned unless a later certified contract says otherwise.

Missing support-contract fields are treated as blocked implementation evidence, not as partial support.

## Plugins And Agent Protocols

| Surface | Current public claim | Trust boundary |
| --- | --- | --- |
| MCP tool bridge | Future/plugin surface, not advertised as executable. Current public surface is descriptor and policy disclosure only, not tool invocation. | Tool access must run through plugin sandbox, policy, approval and redaction. |
| ACP execution backend | Future/plugin surface, not core. | ACP remains an external agent backend plugin; DesktopLab still owns the session. |
| A2A collaboration | Future, not advertised as executable. | Agent-to-agent collaboration requires a separate trust model and audit path. |
| Community plugins | Metadata/trust surfaces only until executable plugin runtime is certified. | Unverified by default; executable trust requires sandbox and explicit approval. |

Plugin listings, protocol contracts and marketplace ideas do not make third-party code executable. Public support requires sandbox, trust, policy, invocation routing and audit evidence.

Plugin reporting separates descriptor state, cold manifest state, runtime registration, install source, integrity status and execution eligibility. Descriptor metadata alone is blocked for execution.
