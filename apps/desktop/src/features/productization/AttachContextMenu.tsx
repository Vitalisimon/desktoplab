import { Paperclip } from "../../design/icons";
import { useRef, useState } from "react";
import type { ExternalAttachmentInput } from "../../api/types";
import { filesToAttachments, TEXT_ATTACHMENT_ACCEPT } from "./externalAttachmentFiles";

type ExternalAttachmentButtonProps = {
  attachments: ExternalAttachmentInput[];
  disabled?: boolean;
  onChange: (attachments: ExternalAttachmentInput[]) => void;
};

export function ExternalAttachmentButton({ attachments, disabled = false, onChange }: ExternalAttachmentButtonProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [rejected, setRejected] = useState(false);
  const selectedCount = attachments.length;
  const buttonLabel = selectedCount > 0 ? `Attach external files, ${selectedCount} attached` : "Attach external files";

  return (
    <div className="relative">
      <input
        ref={inputRef}
        aria-label="Choose external files"
        className="sr-only"
        type="file"
        multiple
        accept={TEXT_ATTACHMENT_ACCEPT}
        onChange={(event) => {
          const input = event.currentTarget;
          void filesToAttachments(input.files).then((selection) => {
            setRejected(selection.rejected);
            onChange(selection.attachments);
            input.value = "";
          });
        }}
      />
      <button
        type="button"
        aria-label={buttonLabel}
        title={buttonLabel}
        className="inline-flex h-8 max-w-[150px] shrink-0 items-center gap-2 rounded-full border border-line px-2.5 text-xs font-medium text-muted hover:text-ink disabled:opacity-50"
        disabled={disabled}
        onClick={() => inputRef.current?.click()}
      >
        <Paperclip size={14} className="shrink-0" />
        {selectedCount > 0 ? <span className="truncate">{selectedCount} attached</span> : null}
      </button>
      {rejected ? <p role="status" className="sr-only">Only text and source files can be attached.</p> : null}
    </div>
  );
}
