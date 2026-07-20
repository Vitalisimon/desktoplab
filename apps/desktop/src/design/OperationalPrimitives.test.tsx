// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { render, screen } from "@testing-library/react";
import {
  CapabilityList,
  EvidenceDisclosure,
  ProgressTimeline,
  RepairActionRow,
  RouteExplanation,
  StatusRow,
  TrustBadge,
} from "./OperationalPrimitives";

test("renders operational status capability repair progress evidence and route primitives", () => {
  render(
    <div>
      <StatusRow label="Local runner" status="ready" detail="Ready for coding work" />
      <StatusRow label="Cloud account" status="blocked" detail="Credential missing" />
      <CapabilityList capabilities={["Chat", "Tool use"]} />
      <RepairActionRow label="Repair local runner" description="Restart the local service" disabled={false} />
      <ProgressTimeline
        items={[
          { id: "download", label: "Download", status: "completed" },
          { id: "verify", label: "Verify", status: "running" },
        ]}
      />
      <EvidenceDisclosure title="Command output" body="short redacted output" />
      <TrustBadge trust="verified" />
      <RouteExplanation kind="local" summary="Runs on this machine" reasons={["No provider access required"]} />
    </div>,
  );

  expect(screen.getByText("Local runner")).toBeInTheDocument();
  expect(screen.getByText("Credential missing")).toBeInTheDocument();
  expect(screen.getByText("Tool use")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Repair local runner" })).toBeEnabled();
  expect(screen.getByText("Verify")).toBeInTheDocument();
  expect(screen.getByText("Command output")).toBeInTheDocument();
  expect(screen.getByText("Verified")).toBeInTheDocument();
  expect(screen.getByText("Runs on this machine")).toBeInTheDocument();
});

test("defines semantic visual tokens for light and dark themes", () => {
  const styles = readFileSync("src/styles.css", "utf8");

  expect(styles).toContain("[data-theme=\"dark\"]");
  expect(styles).toContain("--dl-color-focus");
  expect(styles).toContain("--dl-color-overlay");
  expect(styles).toContain("--dl-shadow-panel");
  expect(styles).toContain("--dl-color-local");
  expect(styles).toContain("--dl-color-provider");
});
