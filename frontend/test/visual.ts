// Issue #168: visual regression baselines.
//
// Jest's built-in DOM serialiser is deliberately fussy — attribute order,
// whitespace-only text nodes and library-owned markup (icons, routers) all
// show up as diffs that no human wrote. `describeVisual` (see below) renders a
// component into the small, stable plain object, so a
// snapshot diff always reads as "this class changed" / "this block moved",
// which is exactly the kind of change the baselines under
// `components/__snapshots__/` are meant to catch.

export interface VisualElement {
  /** Lower-cased tag name. */
  tag: string;
  /** Attributes the UI deliberately controls (see ATTRIBUTE_WHITELIST). */
  attrs: Record<string, string>;
  /** Child elements and normalised text runs. */
  children: VisualNode[];
  /** Inline styles, present only when at least one whitelisted property is set. */
  style?: Record<string, string>;
}

export type VisualNode = string | VisualElement;

/**
 * The attributes the snapshot is allowed to observe. Anything outside this
 * list (data attributes, `aria-*` noise, library internals) is ignored on
 * purpose: it is not a visual decision and must not churn the baselines.
 */
const ATTRIBUTE_WHITELIST = ["class", "href", "title", "role", "aria-label"];

/** Inline style properties the components use to lay themselves out. */
const STYLE_WHITELIST = ["width", "height"];

const TEXT_NODE = 3;
const ELEMENT_NODE = 1;

function normaliseText(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

function describeElement(element: Element): VisualElement {
  const tag = element.tagName.toLowerCase();

  // Icons are third-party markup (path data, stroke attributes); only the
  // fact that an icon is rendered here is a decision of this codebase.
  if (tag === "svg") {
    return { tag, attrs: {}, children: [] };
  }

  const attrs: Record<string, string> = {};
  for (const name of ATTRIBUTE_WHITELIST) {
    const value = element.getAttribute(name);
    if (value !== null) {
      attrs[name] = value;
    }
  }

  const style: Record<string, string> = {};
  const styleSource = element as HTMLElement;
  for (const name of STYLE_WHITELIST) {
    const value = styleSource.style ? styleSource.style.getPropertyValue(name) : "";
    if (value) {
      style[name] = value;
    }
  }

  const children: VisualNode[] = [];
  let pendingText = "";
  for (const child of Array.from(element.childNodes)) {
    if (child.nodeType === TEXT_NODE) {
      pendingText += child.textContent ?? "";
      continue;
    }
    if (child.nodeType !== ELEMENT_NODE) {
      // Comments (React inserts them around text) carry no visual meaning.
      continue;
    }
    const run = normaliseText(pendingText);
    pendingText = "";
    if (run) {
      children.push(run);
    }
    children.push(describeElement(child as Element));
  }
  const trailingRun = normaliseText(pendingText);
  if (trailingRun) {
    children.push(trailingRun);
  }

  const node: VisualElement = { tag, attrs, children };
  if (Object.keys(style).length > 0) {
    node.style = style;
  }
  return node;
}

/**
 * Describe a rendered node as a plain object suitable for `toMatchSnapshot()`.
 *
 * Text runs are whitespace-normalised and adjacent runs are merged, so JSX
 * line breaks never leak into a baseline.
 */
export function describeVisual(node: Node | null): VisualNode {
  if (!node) {
    throw new Error("describeVisual: nothing was rendered");
  }
  if (node.nodeType === TEXT_NODE) {
    return normaliseText(node.textContent ?? "");
  }
  if (node.nodeType !== ELEMENT_NODE) {
    throw new Error(`describeVisual: expected an element, got nodeType ${node.nodeType}`);
  }
  return describeElement(node as Element);
}
