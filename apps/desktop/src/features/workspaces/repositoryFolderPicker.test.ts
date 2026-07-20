import { open } from "@tauri-apps/plugin-dialog";
import { chooseRepositoryFolder } from "./repositoryFolderPicker";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

test("uses the native Tauri directory picker", async () => {
  vi.mocked(open).mockResolvedValue("/repo/desktoplab");

  await expect(chooseRepositoryFolder()).resolves.toBe("/repo/desktoplab");
  expect(open).toHaveBeenCalledWith({
    directory: true,
    multiple: false,
    title: "Choose a project folder",
  });
});

test("returns null when the native picker is cancelled or unavailable", async () => {
  vi.mocked(open).mockResolvedValueOnce(null).mockRejectedValueOnce(new Error("dialog unavailable"));

  await expect(chooseRepositoryFolder()).resolves.toBeNull();
  await expect(chooseRepositoryFolder()).resolves.toBeNull();
});
