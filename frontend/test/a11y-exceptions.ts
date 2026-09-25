import type { A11yException } from "./a11y";

// Issue #169: the accessibility suite fails CI on new violations, so every
// exemption has to be written down here with a reason instead of quietly
// turning a rule off in code.

/**
 * Rule-scope exceptions passed to `checkA11y()` by the component tests.
 *
 * `selector: "*"` means the rule is not asserted for anything rendered in
 * this suite; the entry still documents *why*, and rules without an entry are
 * always enforced.
 */
export const A11Y_EXCEPTIONS: readonly A11yException[] = [
  {
    rule: "region",
    selector: "*",
    reason:
      "Component tests render a single component without the surrounding page, so " +
      "'all content must live inside a landmark' can only be judged on a full page " +
      "render (app/**/page.tsx), never on an isolated fragment.",
  },
];

/** A rule the suite deliberately does not implement, and why. */
export interface UntestableRule {
  rule: string;
  reason: string;
}

/**
 * Rules axe-core would run that jsdom cannot answer: they need layout, paint
 * or a document head. Listed explicitly so the gap is a decision rather than
 * an oversight — a browser-based (Playwright + axe) run should cover them.
 */
export const UNTESTABLE_RULES: readonly UntestableRule[] = [
  {
    rule: "color-contrast",
    reason: "needs computed styles for foreground and background; jsdom has no layout or paint.",
  },
  {
    rule: "target-size",
    reason: "needs bounding boxes to measure pointer target area.",
  },
  {
    rule: "scrollable-region-focusable",
    reason: "needs layout to know whether the region actually overflows.",
  },
  {
    rule: "meta-viewport",
    reason: "page-level rule over <head>; components never render a viewport meta tag.",
  },
];
