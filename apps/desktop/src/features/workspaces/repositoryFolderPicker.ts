import { open } from "@tauri-apps/plugin-dialog";

export async function chooseRepositoryFolder(): Promise<string | null> {
  try {
    const path = await open({
      directory: true,
      multiple: false,
      title: "Choose a project folder",
    });
    return typeof path === "string" ? path : null;
  } catch {
    return null;
  }
}
