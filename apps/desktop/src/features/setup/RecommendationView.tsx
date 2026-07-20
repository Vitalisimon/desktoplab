import { Box, Cpu } from "../../design/icons";
import type { SetupChoice, SetupPlanPreview, SetupRecommendation } from "../../api/types";
import {
  ExpectedLimitations,
  HiddenReasonDetails,
  RecommendationCard,
  RegistryState,
  SelectableAlternatives,
  selectedRecommendation,
} from "./RecommendationDetails";
import { isHardwareHiddenReason } from "./recommendationLabels";

type RecommendationViewProps = {
  preview: SetupPlanPreview;
  selectedRuntimeId?: string;
  selectedModelId?: string;
  onSelectRuntime?: (runtimeId: string) => void;
  onSelectModel?: (modelId: string) => void;
  runtimeSetupChoice: SetupChoice;
  modelSetupChoice: SetupChoice;
  onRuntimeSetupChoiceChange: (choice: SetupChoice) => void;
  onModelSetupChoiceChange: (choice: SetupChoice) => void;
};

export function RecommendationView({
  preview,
  selectedRuntimeId,
  selectedModelId,
  onSelectRuntime,
  onSelectModel,
  runtimeSetupChoice,
  modelSetupChoice,
  onRuntimeSetupChoiceChange,
  onModelSetupChoiceChange,
}: RecommendationViewProps) {
  const runtime = selectedRecommendation(preview.runtimeRecommendations, selectedRuntimeId);
  const model = selectedRecommendation(preview.modelRecommendations, selectedModelId);
  const alternativeRuntimes = preview.runtimeRecommendations.filter((item) => item.manifestId !== runtime?.manifestId);
  const alternativeModels = preview.modelRecommendations.filter((item) => item.manifestId !== model?.manifestId);
  const notRecommendedReasons = preview.hiddenReasons.filter(isHardwareHiddenReason);
  const advancedReasons = preview.hiddenReasons.filter((reason) => !isHardwareHiddenReason(reason));

  return (
    <section aria-labelledby="recommendation-title" className="space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="recommendation-title" className="text-lg font-semibold">
            Recommended setup
          </h2>
          <p className="mt-1 text-sm leading-6 text-muted">
            DesktopLab selected a local setup that fits this computer.
          </p>
        </div>
        <RegistryState state={preview.registryState} />
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <RecommendationCard icon={<Cpu size={18} />} title="Local runner" recommendation={runtime} />
        <RecommendationCard icon={<Box size={18} />} title="Coding model" recommendation={model} />
      </div>

      <SetupChoiceControls
        runtime={runtime}
        model={model}
        runtimeSetupChoice={runtimeSetupChoice}
        modelSetupChoice={modelSetupChoice}
        onRuntimeSetupChoiceChange={onRuntimeSetupChoiceChange}
        onModelSetupChoiceChange={onModelSetupChoiceChange}
      />

      {alternativeRuntimes.length > 0 ? (
        <SelectableAlternatives title="Other compatible runners" items={alternativeRuntimes} onSelect={onSelectRuntime} />
      ) : null}

      {alternativeModels.length > 0 ? (
        <SelectableAlternatives title="Other compatible models" items={alternativeModels} onSelect={onSelectModel} />
      ) : null}

      <ExpectedLimitations limitations={preview.expectedLimitations} />
      <HiddenReasonDetails title="Not recommended on this computer" reasons={notRecommendedReasons} />
      <HiddenReasonDetails title="Advanced catalog details" reasons={advancedReasons} />
    </section>
  );
}

function SetupChoiceControls({
  runtime,
  model,
  runtimeSetupChoice,
  modelSetupChoice,
  onRuntimeSetupChoiceChange,
  onModelSetupChoiceChange,
}: {
  runtime?: SetupRecommendation;
  model?: SetupRecommendation;
  runtimeSetupChoice: SetupChoice;
  modelSetupChoice: SetupChoice;
  onRuntimeSetupChoiceChange: (choice: SetupChoice) => void;
  onModelSetupChoiceChange: (choice: SetupChoice) => void;
}) {
  const runtimeNeedsChoice = runtime?.setupChoiceRequired;
  const modelNeedsChoice = model?.setupChoiceRequired;
  if (!runtimeNeedsChoice && !modelNeedsChoice) return null;
  return (
    <div className="grid gap-3 rounded-desktop border border-line p-3 text-sm md:grid-cols-2 dl-panel">
      {runtimeNeedsChoice ? (
        <ChoiceGroup
          title="Local runner"
          value={runtimeSetupChoice}
          useExistingLabel="Keep my current local setup"
          replaceLabel="Install a fresh local setup"
          onChange={onRuntimeSetupChoiceChange}
        />
      ) : null}
      {modelNeedsChoice ? (
        <ChoiceGroup
          title="Coding model"
          value={modelSetupChoice}
          useExistingLabel="Keep my current local setup"
          replaceLabel="Install a fresh local setup"
          onChange={onModelSetupChoiceChange}
        />
      ) : null}
    </div>
  );
}

function ChoiceGroup({
  title,
  value,
  useExistingLabel,
  replaceLabel,
  onChange,
}: {
  title: string;
  value: SetupChoice;
  useExistingLabel: string;
  replaceLabel: string;
  onChange: (choice: SetupChoice) => void;
}) {
  return (
    <fieldset className="space-y-2">
      <legend className="text-xs font-semibold uppercase text-muted">{title}</legend>
      <label className="flex items-center gap-2 rounded px-3 py-2 font-medium text-ink dl-elevated">
        <input type="radio" checked={value === "use_existing"} onChange={() => onChange("use_existing")} />
        {useExistingLabel}
      </label>
      <label className="flex items-center gap-2 rounded px-3 py-2 font-medium text-ink dl-elevated">
        <input type="radio" checked={value === "replace"} onChange={() => onChange("replace")} />
        {replaceLabel}
      </label>
    </fieldset>
  );
}
