// Issue #169: accessibility assertions for the component suite — an
// "axe or equivalent" checker that runs on jsdom markup alone, so CI can fail
// on a new violation without pulling a browser-sized dependency into the repo.
//
// The rules mirror axe-core's WCAG A/AA rules that are decidable from markup
// without layout or paint (names, labels, ARIA validity, focus management,
// heading order). Rules that need a real renderer, and rules that only make
// sense for whole pages, are listed with their reasons in a11y-exceptions.ts.

export interface A11yException {
  /** Rule the exemption applies to, e.g. "region". */
  rule: string;
  /** CSS selector matched against the offending element; `*` matches everything. */
  selector: string;
  /** Why the exemption exists — required so exceptions stay reviewable. */
  reason: string;
}

export interface A11yViolation {
  /** Rule id, named after the axe-core rule it mirrors where one exists. */
  rule: string;
  /** What is wrong with this element. */
  message: string;
  /** Short, stable description of the offending element for the failure output. */
  target: string;
  /** The offending element, used to match exceptions. */
  element: Element;
}

export interface A11yOptions {
  /** Known exceptions; see `A11Y_EXCEPTIONS` in a11y-exceptions.ts. */
  exceptions?: readonly A11yException[];
}

export type A11yScope = Element | Document | DocumentFragment;

const TEXT_NODE = 3;
const ELEMENT_NODE = 1;

/** Roles that must expose an accessible name to be usable. */
const BUTTON_LIKE_ROLES = ["button", "checkbox", "radio", "switch", "tab"];
const LINK_LIKE_ROLES = ["link"];

const LANDMARK_SELECTOR =
  'header, footer, main, nav, aside, [role="banner"], [role="navigation"], ' +
  '[role="main"], [role="contentinfo"], [role="complementary"], [role="search"], [role="region"]';

/** Non-interactive elements whose text must live inside a landmark (axe `region`). */
const NON_LANDMARK_EXEMPT = 'script, style, template, option, optgroup, [role="presentation"], [role="none"]';

/** Every attribute defined by ARIA 1.2 — anything else is a typo. */
const ARIA_ATTRIBUTES = new Set([
  "aria-activedescendant",
  "aria-atomic",
  "aria-autocomplete",
  "aria-braillelabel",
  "aria-brailleroledescription",
  "aria-busy",
  "aria-checked",
  "aria-colcount",
  "aria-colindex",
  "aria-colindextext",
  "aria-colspan",
  "aria-controls",
  "aria-current",
  "aria-describedby",
  "aria-description",
  "aria-details",
  "aria-disabled",
  "aria-errormessage",
  "aria-expanded",
  "aria-flowto",
  "aria-haspopup",
  "aria-hidden",
  "aria-invalid",
  "aria-keyshortcuts",
  "aria-label",
  "aria-labelledby",
  "aria-level",
  "aria-live",
  "aria-modal",
  "aria-multiline",
  "aria-multiselectable",
  "aria-orientation",
  "aria-owns",
  "aria-placeholder",
  "aria-posinset",
  "aria-pressed",
  "aria-readonly",
  "aria-relevant",
  "aria-required",
  "aria-roledescription",
  "aria-rowcount",
  "aria-rowindex",
  "aria-rowindextext",
  "aria-rowspan",
  "aria-selected",
  "aria-setsize",
  "aria-sort",
  "aria-valuemax",
  "aria-valuemin",
  "aria-valuenow",
  "aria-valuetext",
]);

/** Attributes whose value is a whitespace-separated list of element ids. */
const IDREF_ATTRIBUTES = new Set([
  "aria-activedescendant",
  "aria-describedby",
  "aria-details",
  "aria-errormessage",
  "aria-flowto",
  "aria-labelledby",
  "aria-owns",
]);

/** Attributes whose value must come from a fixed token list. */
const TOKEN_VALUES: Record<string, readonly string[]> = {
  "aria-autocomplete": ["none", "list", "inline", "both"],
  "aria-busy": ["true", "false"],
  "aria-checked": ["true", "false", "mixed", "undefined"],
  "aria-current": ["true", "false", "page", "step", "location", "date", "time"],
  "aria-disabled": ["true", "false"],
  "aria-expanded": ["true", "false", "undefined"],
  "aria-haspopup": ["true", "false", "menu", "listbox", "tree", "grid", "dialog"],
  "aria-hidden": ["true", "false", "undefined"],
  "aria-invalid": ["true", "false", "grammar", "spelling"],
  "aria-live": ["off", "polite", "assertive"],
  "aria-modal": ["true", "false"],
  "aria-multiline": ["true", "false"],
  "aria-multiselectable": ["true", "false"],
  "aria-orientation": ["horizontal", "vertical", "undefined"],
  "aria-pressed": ["true", "false", "mixed", "undefined"],
  "aria-readonly": ["true", "false"],
  "aria-relevant": ["additions", "removals", "text", "all"],
  "aria-required": ["true", "false"],
  "aria-selected": ["true", "false", "undefined"],
  "aria-sort": ["none", "ascending", "descending", "other"],
};

/** Attributes that must parse as a number. */
const NUMERIC_ARIA_ATTRIBUTES = new Set([
  "aria-colcount",
  "aria-colindex",
  "aria-colspan",
  "aria-level",
  "aria-posinset",
  "aria-rowcount",
  "aria-rowindex",
  "aria-rowspan",
  "aria-setsize",
  "aria-valuemax",
  "aria-valuemin",
  "aria-valuenow",
]);

/**
 * Roles that cannot work without their state attributes (ARIA 1.2). Only the
 * roles whose required properties are unambiguous are listed; components in
 * this repo do not use them today, but a new `role="checkbox"` without
 * `aria-checked` should fail CI.
 */
const REQUIRED_ARIA_BY_ROLE: Record<string, readonly string[]> = {
  checkbox: ["aria-checked"],
  radio: ["aria-checked"],
  switch: ["aria-checked"],
  slider: ["aria-valuemax", "aria-valuemin", "aria-valuenow"],
  spinbutton: ["aria-valuemax", "aria-valuemin", "aria-valuenow"],
};

function isElement(node: Node): node is Element {
  return node.nodeType === ELEMENT_NODE;
}

function normalise(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

function trimmedAttribute(element: Element, name: string): string {
  return (element.getAttribute(name) ?? "").trim();
}

function hasRole(element: Element, role: string): boolean {
  const value = element.getAttribute("role");
  if (value === null) {
    return false;
  }
  return value.trim().split(/\s+/).includes(role);
}

/** True when the element, or any ancestor, hides itself from assistive tech. */
function hiddenFromA11y(element: Element): boolean {
  let current: Element | null = element;
  while (current) {
    if (current.getAttribute("aria-hidden") === "true") {
      return true;
    }
    current = current.parentElement;
  }
  return false;
}

function closestAncestor(
  element: Element,
  predicate: (candidate: Element) => boolean,
  stopAt: Element | null,
): Element | null {
  let current = element.parentElement;
  while (current) {
    if (predicate(current)) {
      return current;
    }
    if (current === stopAt) {
      return null;
    }
    current = current.parentElement;
  }
  return null;
}

/**
 * Programmatic focusability, following the same shape as axe's `focusable`
 * check: positive and zero tabindex, `tabindex="-1"`, enabled native widgets
 * and anchors that actually carry an href.
 */
function isFocusable(element: Element): boolean {
  const tabindex = element.getAttribute("tabindex");
  if (tabindex !== null) {
    const parsed = Number.parseInt(tabindex, 10);
    return !Number.isNaN(parsed) && parsed >= -1;
  }
  const tag = element.tagName;
  if (tag === "A") {
    return element.hasAttribute("href");
  }
  if (element.hasAttribute("disabled")) {
    return false;
  }
  if (tag === "BUTTON" || tag === "SELECT" || tag === "TEXTAREA" || tag === "SUMMARY") {
    return true;
  }
  if (tag === "INPUT") {
    return trimmedAttribute(element, "type").toLowerCase() !== "hidden";
  }
  return false;
}

function descendants(element: Element): Element[] {
  return Array.from(element.querySelectorAll("*"));
}

/** Text this element owns directly — used by the `region` rule. */
function directText(element: Element): string {
  let text = "";
  for (const child of Array.from(element.childNodes)) {
    if (child.nodeType === TEXT_NODE) {
      text += child.textContent ?? "";
    }
  }
  return normalise(text);
}

/** Visible text, skipping subtrees that assistive tech cannot see. */
function visibleText(element: Element): string {
  let text = "";
  const walk = (node: Node): void => {
    if (node.nodeType === TEXT_NODE) {
      text += node.textContent ?? "";
      return;
    }
    if (!isElement(node)) {
      return;
    }
    if (node.getAttribute("aria-hidden") === "true") {
      return;
    }
    for (const child of Array.from(node.childNodes)) {
      walk(child);
    }
  };
  walk(element);
  return normalise(text);
}

function ownerDocumentOf(scope: A11yScope): Document | null {
  return scope.nodeType === 9 ? (scope as Document) : scope.ownerDocument;
}

function findById(scope: A11yScope, id: string): Element | null {
  const parent: ParentNode = scope;
  if (isElement(scope) && scope.id === id) {
    return scope;
  }
  for (const element of Array.from(parent.querySelectorAll("[id]"))) {
    if (element.id === id) {
      return element;
    }
  }
  const ownerDoc = ownerDocumentOf(scope);
  return ownerDoc ? ownerDoc.getElementById(id) : null;
}

function idReferenceText(scope: A11yScope, element: Element, attribute: string): string {
  const value = trimmedAttribute(element, attribute);
  if (!value) {
    return "";
  }
  const parts = value.split(/\s+/).map((id) => {
    const target = findById(scope, id);
    return target ? visibleText(target) : "";
  });
  return normalise(parts.join(" "));
}

function wrappingLabel(element: Element): Element | null {
  const label = element.closest("label");
  return label && label !== element ? label : null;
}

function explicitLabel(scope: A11yScope, element: Element): Element | null {
  if (!element.id) {
    return null;
  }
  const parent: ParentNode = scope;
  for (const label of Array.from(parent.querySelectorAll("label"))) {
    if (label.htmlFor === element.id) {
      return label;
    }
  }
  const ownerDoc = ownerDocumentOf(scope);
  if (ownerDoc) {
    for (const label of Array.from(ownerDoc.querySelectorAll("label"))) {
      if (label.htmlFor === element.id) {
        return label;
      }
    }
  }
  return null;
}

function isFormControl(element: Element): boolean {
  const tag = element.tagName;
  if (tag === "TEXTAREA" || tag === "SELECT") {
    return true;
  }
  if (tag !== "INPUT") {
    return false;
  }
  const type = trimmedAttribute(element, "type").toLowerCase();
  return type !== "hidden";
}

/**
 * The sources axe's `label` rule accepts: implicit (wrapping) label,
 * explicit `label[for]`, `aria-label`, `aria-labelledby`, `title` and
 * `placeholder`, plus `role="presentation"` / `role="none"`.
 */
function hasFormControlLabel(scope: A11yScope, element: Element): boolean {
  const wrapping = wrappingLabel(element);
  if (wrapping && visibleText(wrapping)) {
    return true;
  }
  const explicit = explicitLabel(scope, element);
  if (explicit && visibleText(explicit)) {
    return true;
  }
  if (trimmedAttribute(element, "aria-label")) {
    return true;
  }
  if (idReferenceText(scope, element, "aria-labelledby")) {
    return true;
  }
  if (trimmedAttribute(element, "title")) {
    return true;
  }
  if (trimmedAttribute(element, "placeholder")) {
    return true;
  }
  return hasRole(element, "presentation") || hasRole(element, "none");
}

/**
 * Accessible name, following the same precedence axe uses: `aria-labelledby`,
 * `aria-label`, then the element's own naming method, then `title`.
 */
export function accessibleName(scope: A11yScope, element: Element): string {
  const referenced = idReferenceText(scope, element, "aria-labelledby");
  if (referenced) {
    return referenced;
  }
  const ariaLabel = trimmedAttribute(element, "aria-label");
  if (ariaLabel) {
    return ariaLabel;
  }

  const tag = element.tagName;
  if (tag === "BUTTON" || tag === "A" || tag === "LABEL" || tag === "SUMMARY" || tag === "LEGEND") {
    const text = visibleText(element);
    if (text) {
      return text;
    }
  }

  if (isFormControl(element)) {
    const wrapping = wrappingLabel(element);
    if (wrapping && visibleText(wrapping)) {
      return visibleText(wrapping);
    }
    const explicit = explicitLabel(scope, element);
    if (explicit && visibleText(explicit)) {
      return visibleText(explicit);
    }
    const title = trimmedAttribute(element, "title");
    if (title) {
      return title;
    }
    const placeholder = trimmedAttribute(element, "placeholder");
    if (placeholder) {
      return placeholder;
    }
    if (tag === "INPUT") {
      const type = trimmedAttribute(element, "type").toLowerCase();
      if (type === "submit" || type === "reset") {
        return trimmedAttribute(element, "value") || type;
      }
      if (type === "button") {
        return trimmedAttribute(element, "value");
      }
    }
    return "";
  }

  if (tag === "IMG") {
    return trimmedAttribute(element, "alt");
  }

  const text = visibleText(element);
  if (text) {
    return text;
  }
  return trimmedAttribute(element, "title");
}

/** Heading level for `h1`-`h6` and `role="heading"`, or `null`. */
function headingLevel(element: Element): number | null {
  const tag = element.tagName;
  if (/^H[1-6]$/.test(tag)) {
    return Number.parseInt(tag.slice(1), 10);
  }
  if (hasRole(element, "heading")) {
    const level = trimmedAttribute(element, "aria-level");
    return level ? Number.parseInt(level, 10) : 1;
  }
  return null;
}

function isLandmark(element: Element): boolean {
  return element.matches(LANDMARK_SELECTOR);
}

function hasLandmarkAncestor(element: Element, scope: A11yScope): boolean {
  let current: Element | null = element;
  while (current) {
    if (isLandmark(current)) {
      return true;
    }
    if (current === scope) {
      return false;
    }
    current = current.parentElement;
  }
  return false;
}

/** axe's `svg-img-alt` selector: only graphics roles have to name themselves. */
function needsSvgAlt(element: Element): boolean {
  if (element.tagName !== "svg") {
    return false;
  }
  return (
    hasRole(element, "img") ||
    hasRole(element, "graphics-symbol") ||
    hasRole(element, "graphics-document")
  );
}

function describeTarget(element: Element): string {
  const parts: string[] = [element.tagName.toLowerCase()];
  if (element.id) {
    parts.push(`#${element.id}`);
  }
  const className = trimmedAttribute(element, "class");
  if (className) {
    const classes = className.split(/\s+/);
    parts.push(`.${classes.slice(0, 2).join(".")}${classes.length > 2 ? "…" : ""}`);
  }
  const text = visibleText(element);
  if (text) {
    parts.push(` "${text.slice(0, 40)}${text.length > 40 ? "…" : ""}"`);
  }
  return parts.join("");
}

function isExcepted(violation: A11yViolation, exceptions: readonly A11yException[]): boolean {
  return exceptions.some(
    (exception) => exception.rule === violation.rule && violation.element.matches(exception.selector),
  );
}

/**
 * Check a rendered subtree for accessibility violations.
 *
 * Returns an empty array when the markup is clean; `expectNoA11yViolations`
 * is the usual entry point from a test.
 */
export function checkA11y(scope: A11yScope, options: A11yOptions = {}): A11yViolation[] {
  const exceptions = options.exceptions ?? [];
  const parent: ParentNode = scope;
  const elements: Element[] = [];
  if (isElement(scope)) {
    elements.push(scope);
  }
  elements.push(...Array.from(parent.querySelectorAll("*")));

  const violations: A11yViolation[] = [];
  const report = (rule: string, message: string, element: Element): void => {
    violations.push({ rule, message, target: describeTarget(element), element });
  };

  const byId = new Map<string, Element[]>();
  for (const element of elements) {
    if (!element.id) {
      continue;
    }
    const bucket = byId.get(element.id);
    if (bucket) {
      bucket.push(element);
    } else {
      byId.set(element.id, [element]);
    }
  }
  for (const [id, bucket] of byId) {
    if (bucket.length < 2) {
      continue;
    }
    for (const element of bucket) {
      report("duplicate-id", `id "${id}" is used ${bucket.length} times`, element);
    }
  }

  for (const element of elements) {
    const tag = element.tagName;
    const hidden = hiddenFromA11y(element);

    for (const attribute of Array.from(element.attributes)) {
      const name = attribute.name;
      if (!name.startsWith("aria-")) {
        continue;
      }
      if (!ARIA_ATTRIBUTES.has(name)) {
        report("aria-valid-attr", `unknown ARIA attribute ${name}`, element);
        continue;
      }
      const value = attribute.value;
      if (IDREF_ATTRIBUTES.has(name)) {
        for (const id of value.split(/\s+/).filter(Boolean)) {
          if (!findById(scope, id)) {
            report("aria-valid-attr-value", `${name} references missing id "${id}"`, element);
          }
        }
      } else if (TOKEN_VALUES[name]) {
        if (!TOKEN_VALUES[name].includes(value)) {
          report(
            "aria-valid-attr-value",
            `${name}="${value}" is not one of: ${TOKEN_VALUES[name].join(", ")}`,
            element,
          );
        }
      } else if (
        NUMERIC_ARIA_ATTRIBUTES.has(name) &&
        value.trim() !== "" &&
        !Number.isFinite(Number(value))
      ) {
        report("aria-valid-attr-value", `${name}="${value}" is not a number`, element);
      }
    }

    const role = (element.getAttribute("role") ?? "").trim().split(/\s+/)[0];
    if (role) {
      const required = REQUIRED_ARIA_BY_ROLE[role];
      for (const attribute of required ?? []) {
        if (!element.hasAttribute(attribute)) {
          report("aria-required-attr", `role="${role}" requires ${attribute}`, element);
        }
      }
    }

    if (element.getAttribute("aria-hidden") === "true" && !closestAncestor(element, (candidate) => candidate.getAttribute("aria-hidden") === "true", null)) {
      for (const descendant of descendants(element)) {
        if (isFocusable(descendant)) {
          report("aria-hidden-focus", "focusable element inside an aria-hidden subtree", descendant);
        }
      }
    }

    if (isFocusable(element)) {
      for (const descendant of descendants(element)) {
        if (isFocusable(descendant) && closestAncestor(descendant, isFocusable, element) === element) {
          report(
            "nested-interactive",
            "focusable element nested inside another focusable element",
            descendant,
          );
        }
      }
    }

    const tabindex = element.getAttribute("tabindex");
    if (tabindex !== null) {
      const parsed = Number.parseInt(tabindex, 10);
      if (!Number.isNaN(parsed) && parsed > 0) {
        report("tabindex", `tabindex="${tabindex}" moves focus out of document order`, element);
      }
    }

    if (hidden) {
      continue;
    }

    if (tag === "IMG" && !element.hasAttribute("alt")) {
      report("image-alt", "image has no alt attribute", element);
    }

    const isButton = tag === "BUTTON" || BUTTON_LIKE_ROLES.some((candidate) => hasRole(element, candidate));
    if (isButton && !accessibleName(scope, element)) {
      report("button-name", "control has no accessible name", element);
    }

    const isLink =
      (tag === "A" && element.hasAttribute("href")) ||
      LINK_LIKE_ROLES.some((candidate) => hasRole(element, candidate));
    if (isLink && !accessibleName(scope, element)) {
      report("link-name", "link has no accessible name", element);
    }

    if (tag === "A" && element.hasAttribute("href")) {
      const href = trimmedAttribute(element, "href");
      if (href === "" || href === "#") {
        report("link-href", `href="${href}" does not point anywhere`, element);
      }
    }

    if (isFormControl(element) && !hasFormControlLabel(scope, element)) {
      report("label", "form control has no associated label", element);
    }

    if (needsSvgAlt(element) && !accessibleName(scope, element)) {
      report("svg-img-alt", "graphics role has no accessible name", element);
    }

    const level = headingLevel(element);
    if (level !== null && !accessibleName(scope, element)) {
      report("empty-heading", "heading has no accessible name", element);
    }

    if (
      directText(element) &&
      !element.matches(NON_LANDMARK_EXEMPT) &&
      !hasLandmarkAncestor(element, scope)
    ) {
      report("region", "text content is not inside a landmark", element);
    }
  }

  let previousLevel: number | null = null;
  for (const element of elements) {
    const level = headingLevel(element);
    if (level === null || hiddenFromA11y(element)) {
      continue;
    }
    if (previousLevel !== null && level - previousLevel > 1) {
      report("heading-order", `heading h${level} follows h${previousLevel}`, element);
    }
    previousLevel = level;
  }

  return violations.filter((violation) => !isExcepted(violation, exceptions));
}

/** Render violations as a readable, one-per-line failure message. */
export function formatViolations(violations: readonly A11yViolation[]): string {
  if (violations.length === 0) {
    return "no accessibility violations";
  }
  return violations
    .map((violation, index) => `${index + 1}. [${violation.rule}] ${violation.target} — ${violation.message}`)
    .join("\n");
}

/** Assert that a rendered subtree has no accessibility violations. */
export function expectNoA11yViolations(scope: A11yScope, options: A11yOptions = {}): void {
  const violations = checkA11y(scope, options);
  if (violations.length > 0) {
    throw new Error(
      `Found ${violations.length} accessibility violation(s):\n${formatViolations(violations)}`,
    );
  }
}
