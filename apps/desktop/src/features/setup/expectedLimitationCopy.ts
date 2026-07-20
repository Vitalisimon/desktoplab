const USER_HIDDEN_LIMITATIONS = new Set(["accelerator confidence requires v2 driver/runtime probing"]);

const expectedLimitationCopy: Record<string, string> = {};

export function friendlyExpectedLimitation(limitation: string): string | null {
  if (USER_HIDDEN_LIMITATIONS.has(limitation)) return null;
  return expectedLimitationCopy[limitation] ?? limitation;
}

export function visibleExpectedLimitations(limitations: string[]) {
  return limitations.map(friendlyExpectedLimitation).filter((limitation): limitation is string => Boolean(limitation));
}
