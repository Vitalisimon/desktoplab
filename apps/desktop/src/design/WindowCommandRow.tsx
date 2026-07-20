import { GitPullRequestArrow, PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen, TerminalSquare } from "./icons";
import type { WorkspaceSnapshot } from "../api/types";
import type { RepositoryOpenTarget } from "./repositoryOpen";
import { RepositoryOpenMenu } from "./RepositoryOpenMenu";
import { ChromeIconButton } from "./ChromeIconButton";
import { startWindowDrag, toggleWindowMaximized } from "./windowDrag";
import { WindowStatusDot } from "./WindowStatusDot";
import { detectDesktopPlatform, type DesktopPlatform } from "./desktopPlatform";

type WindowCommandRowProps = {
  leftOpen: boolean;
  rightOpen: boolean;
  terminalOpen: boolean;
  changesOpen: boolean;
  hasWorkspace: boolean;
  workspace: WorkspaceSnapshot | null;
  onToggleLeft: () => void;
  onToggleRight: () => void;
  onToggleTerminal: () => void;
  onToggleChanges: () => void;
  openTargets: RepositoryOpenTarget[];
  onOpenTarget: (targetId: string) => void;
  platform?: DesktopPlatform;
};

export function WindowCommandRow({
  leftOpen,
  rightOpen,
  terminalOpen,
  changesOpen,
  hasWorkspace,
  workspace,
  onToggleLeft,
  onToggleRight,
  onToggleTerminal,
  onToggleChanges,
  openTargets,
  onOpenTarget,
  platform = detectDesktopPlatform(),
}: WindowCommandRowProps) {
  const usesOverlayChrome = platform === "macos";

  return (
    <div
      data-platform={platform}
      data-tauri-drag-region={usesOverlayChrome ? true : undefined}
      data-testid="window-command-row"
      className={`grid h-9 shrink-0 items-center gap-3 border-b border-line pr-3 dl-ambient-bar ${
        usesOverlayChrome ? "grid-cols-[104px_minmax(0,1fr)_auto]" : "grid-cols-[40px_minmax(0,1fr)_auto]"
      }`}
      onMouseDown={usesOverlayChrome ? startWindowDrag : undefined}
    >
      <div data-testid="window-chrome-left-cluster" className={`flex h-full items-center gap-1.5 ${usesOverlayChrome ? "pl-[92px]" : "pl-3"}`}>
        <ChromeIconButton label={leftOpen ? "Collapse left drawer" : "Expand left drawer"} pressed={leftOpen} onClick={onToggleLeft}>
          {leftOpen ? <PanelLeftClose size={14} /> : <PanelLeftOpen size={14} />}
        </ChromeIconButton>
      </div>
      <div className="min-w-0 text-center" onDoubleClick={usesOverlayChrome ? toggleWindowMaximized : undefined}>
        {workspace || usesOverlayChrome ? (
          <>
            <div className="flex items-center justify-center gap-2 truncate text-sm font-medium text-ink">
              <WindowStatusDot hasWorkspace={hasWorkspace} />
              <span className="truncate">{workspace?.displayName ?? "Desktop client"}</span>
            </div>
            <div className="truncate text-[11px] leading-3 text-muted">
              {workspace ? workspace.rootPath : "Open a repository to start working"}
            </div>
          </>
        ) : (
          <div className="truncate text-sm text-muted">Open a repository</div>
        )}
      </div>
      {hasWorkspace ? (
        <div className="flex items-center justify-end gap-1.5">
          <ChromeIconButton label={changesOpen ? "Hide changes panel" : "Show changes panel"} pressed={changesOpen} onClick={onToggleChanges}>
            <GitPullRequestArrow size={14} />
          </ChromeIconButton>
          <RepositoryOpenMenu targets={openTargets} onOpen={onOpenTarget} />
          <ChromeIconButton label={terminalOpen ? "Hide terminal" : "Show terminal"} pressed={terminalOpen} onClick={onToggleTerminal}>
            <TerminalSquare size={14} />
          </ChromeIconButton>
          <ChromeIconButton label={rightOpen ? "Hide inspector" : "Show inspector"} pressed={rightOpen} onClick={onToggleRight}>
            {rightOpen ? <PanelRightClose size={14} /> : <PanelRightOpen size={14} />}
          </ChromeIconButton>
        </div>
      ) : (
        <div />
      )}
    </div>
  );
}
