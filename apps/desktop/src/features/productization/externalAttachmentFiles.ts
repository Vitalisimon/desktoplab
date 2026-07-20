import type { ExternalAttachmentInput } from "../../api/types";

const MAX_ATTACHMENT_BYTES = 64 * 1024;
const MAX_ATTACHMENTS = 8;

export const TEXT_ATTACHMENT_ACCEPT = [
  "text/*",
  "application/json",
  "application/toml",
  "application/xml",
  ".md,.txt,.json,.yaml,.yml,.toml,.csv,.ts,.tsx,.js,.jsx,.rs,.py,.go,.java,.kt,.swift,.c,.cc,.cpp,.h",
].join(",");

export type AttachmentSelection = {
  attachments: ExternalAttachmentInput[];
  rejected: boolean;
};

export async function filesToAttachments(files: FileList | null): Promise<AttachmentSelection> {
  const selected = Array.from(files ?? []).slice(0, MAX_ATTACHMENTS);
  const readable = selected.filter((file) => textLikeFile(file));
  const attachments = (await Promise.all(readable.map(fileToAttachment))).filter(
    (attachment): attachment is ExternalAttachmentInput => attachment !== null,
  );
  return {
    attachments,
    rejected: selected.length !== Array.from(files ?? []).length || attachments.length !== selected.length,
  };
}

async function fileToAttachment(file: File): Promise<ExternalAttachmentInput | null> {
  const contentText = await readTextFilePreview(file);
  if (contentText === undefined) return null;
  const contentSha256 = await sha256(contentText);
  if (!contentSha256) return null;
  return {
    name: file.name,
    size: file.size,
    mediaType: file.type || "text/plain",
    contentText,
    contentSha256,
    truncated: file.size > MAX_ATTACHMENT_BYTES,
  };
}

function textLikeFile(file: File): boolean {
  return file.type.startsWith("text/") || ["application/json", "application/toml", "application/xml"].includes(file.type) || textLikeName(file.name);
}

function textLikeName(name: string): boolean {
  return /\.(md|txt|json|ya?ml|toml|csv|ts|tsx|js|jsx|rs|py|go|java|kt|swift|c|cc|cpp|h)$/i.test(name);
}

async function readTextFilePreview(file: File): Promise<string | undefined> {
  const chunk = file.slice(0, MAX_ATTACHMENT_BYTES);
  if ("text" in chunk && typeof chunk.text === "function") return chunk.text();
  if ("text" in file && typeof file.text === "function") return (await file.text()).slice(0, MAX_ATTACHMENT_BYTES);
  return readTextWithFileReader(chunk);
}

function readTextWithFileReader(blob: Blob): Promise<string | undefined> {
  if (typeof FileReader === "undefined") return Promise.resolve(undefined);
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onerror = () => resolve(undefined);
    reader.onload = () => resolve(typeof reader.result === "string" ? reader.result : undefined);
    reader.readAsText(blob);
  });
}

async function sha256(content: string): Promise<string | undefined> {
  if (!globalThis.crypto?.subtle) return undefined;
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(content));
  const hex = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `sha256:${hex}`;
}
