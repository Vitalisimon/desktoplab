import { expect } from "vitest";

type MatcherResult = {
  pass: boolean;
  message: () => string;
};

type ElementLike = Element & {
  checked?: boolean;
  disabled?: boolean;
  value?: unknown;
};

function matcher(pass: boolean, message: string): MatcherResult {
  return { pass, message: () => message };
}

function asElement(received: unknown): ElementLike | null {
  return received instanceof Element ? (received as ElementLike) : null;
}

function textOf(received: unknown) {
  return asElement(received)?.textContent ?? String(received ?? "");
}

expect.extend({
  toBeInTheDocument(received: unknown) {
    const element = asElement(received);
    return matcher(Boolean(element?.ownerDocument?.documentElement.contains(element)), "expected element to be in the document");
  },
  toBeDisabled(received: unknown) {
    const element = asElement(received);
    return matcher(Boolean(element?.disabled || element?.getAttribute("aria-disabled") === "true"), "expected element to be disabled");
  },
  toBeEnabled(received: unknown) {
    const element = asElement(received);
    return matcher(Boolean(element && !element.disabled && element.getAttribute("aria-disabled") !== "true"), "expected element to be enabled");
  },
  toBeChecked(received: unknown) {
    const element = asElement(received);
    return matcher(Boolean(element?.checked || element?.getAttribute("aria-checked") === "true"), "expected element to be checked");
  },
  toBeVisible(received: unknown) {
    const element = asElement(received);
    const style = element ? getComputedStyle(element) : null;
    return matcher(
      Boolean(element && !element.hasAttribute("hidden") && style?.display !== "none" && style?.visibility !== "hidden" && style?.opacity !== "0"),
      "expected element to be visible",
    );
  },
  toContainElement(received: unknown, expected: Element) {
    const element = asElement(received);
    return matcher(Boolean(element?.contains(expected)), "expected element to contain the provided child");
  },
  toHaveAttribute(received: unknown, name: string, value?: string) {
    const element = asElement(received);
    const actual = element?.getAttribute(name);
    const pass = value === undefined ? actual !== null : actual === value;
    return matcher(pass, `expected element to have attribute ${name}`);
  },
  toHaveClass(received: unknown, ...classes: string[]) {
    const element = asElement(received);
    const expected = classes.flatMap((item) => item.split(/\s+/).filter(Boolean));
    return matcher(Boolean(element && expected.every((className) => element.classList.contains(className))), "expected element to have class");
  },
  toHaveStyle(received: unknown, expected: Record<string, string>) {
    const element = asElement(received);
    const style = element ? getComputedStyle(element) : null;
    return matcher(Boolean(style && Object.entries(expected).every(([key, value]) => style.getPropertyValue(key) === value || style[key as keyof CSSStyleDeclaration] === value)), "expected element to have style");
  },
  toHaveTextContent(received: unknown, expected: RegExp | string) {
    const actual = textOf(received);
    const pass = expected instanceof RegExp ? expected.test(actual) : actual.includes(expected);
    return matcher(pass, "expected element to have text content");
  },
  toHaveValue(received: unknown, expected: unknown) {
    return matcher(asElement(received)?.value === expected, "expected element to have value");
  },
});
