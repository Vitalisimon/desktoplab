import { clamp, drawerWidth } from "./paneSizing";

type ResizeHandleProps = {
  label: string;
  onStart: (clientX: number) => void;
  onKeyStep: (delta: number) => void;
};

export function ResizeHandle({ label, onStart, onKeyStep }: ResizeHandleProps) {
  return (
    <div
      role="separator"
      aria-label={label}
      aria-orientation="vertical"
      tabIndex={0}
      className="h-full cursor-col-resize bg-line/60 transition-colors hover:bg-accent/35 focus:outline-none focus:ring-2 focus:ring-focus"
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
          event.preventDefault();
          onKeyStep(event.key === "ArrowRight" ? 16 : -16);
        }
      }}
      onMouseDown={(event) => {
        event.preventDefault();
        onStart(event.clientX);
      }}
    />
  );
}

export function startResize(side: "left" | "right", startX: number, startWidth: number, setWidth: (width: number) => void) {
  const limits = side === "left" ? [drawerWidth.leftMin, drawerWidth.leftMax] : [drawerWidth.rightMin, drawerWidth.rightMax];
  const resize = (event: MouseEvent) => {
    const delta = event.clientX - startX;
    const width = side === "left" ? startWidth + delta : startWidth - delta;
    setWidth(clamp(width, limits[0], limits[1]));
  };
  const stop = () => {
    window.removeEventListener("mousemove", resize);
    window.removeEventListener("mouseup", stop);
  };
  window.addEventListener("mousemove", resize);
  window.addEventListener("mouseup", stop);
}
