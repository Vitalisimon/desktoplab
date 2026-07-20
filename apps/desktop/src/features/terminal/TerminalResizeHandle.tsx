const terminalHeight = {
  min: 112,
  max: 420,
} as const;

export function TerminalResizeHandle({
  startHeight,
  setHeight,
}: {
  startHeight: number;
  setHeight: (height: number) => void;
}) {
  return (
    <div
      role="separator"
      aria-label="Resize terminal drawer"
      aria-orientation="horizontal"
      title="Resize terminal drawer"
      tabIndex={0}
      className="h-1 shrink-0 cursor-row-resize bg-line/60 transition-colors hover:bg-accent/35 focus:outline-none focus:ring-2 focus:ring-focus"
      onKeyDown={(event) => {
        if (event.key === "ArrowUp" || event.key === "ArrowDown") {
          event.preventDefault();
          setHeight(clamp(startHeight + (event.key === "ArrowUp" ? 16 : -16), terminalHeight.min, terminalHeight.max));
        }
      }}
      onMouseDown={(event) => {
        event.preventDefault();
        startTerminalResize(event.clientY, startHeight, setHeight);
      }}
    />
  );
}

function startTerminalResize(startY: number, startHeight: number, setHeight: (height: number) => void) {
  const resize = (event: MouseEvent) => {
    const height = startHeight + startY - event.clientY;
    setHeight(clamp(height, terminalHeight.min, terminalHeight.max));
  };
  const stop = () => {
    window.removeEventListener("mousemove", resize);
    window.removeEventListener("mouseup", stop);
  };
  window.addEventListener("mousemove", resize);
  window.addEventListener("mouseup", stop);
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}
