export async function openExternalUrl(url: string) {
  if (typeof window === "undefined") return;
  if ("__TAURI_INTERNALS__" in window) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("open_external_url", { url });
    return;
  }

  window.open(url, "_blank", "noopener,noreferrer");
}
