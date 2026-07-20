import { describe, expect, test } from "vitest";
import { filesToAttachments } from "./externalAttachmentFiles";

describe("external attachment files", () => {
  test("binds readable text to its SHA-256 digest", async () => {
    const selection = await select(new File(["notes"], "brief.txt", { type: "text/plain" }));

    expect(selection.rejected).toBe(false);
    expect(selection.attachments[0]).toMatchObject({
      contentText: "notes",
      contentSha256: "sha256:ab5aa97074c454a0632057e704220d9a6678fbf773a0a5806fc09b8173b07309",
    });
  });

  test("keeps an empty text file as real attached content", async () => {
    const selection = await select(new File([], "empty.md", { type: "text/markdown" }));

    expect(selection.rejected).toBe(false);
    expect(selection.attachments[0]).toMatchObject({
      contentText: "",
      contentSha256: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    });
  });

  test("rejects binary files", async () => {
    const selection = await select(new File([new Uint8Array([137, 80, 78, 71])], "screen.png", { type: "image/png" }));

    expect(selection).toEqual({ attachments: [], rejected: true });
  });
});

function select(...files: File[]) {
  return filesToAttachments(files as unknown as FileList);
}
