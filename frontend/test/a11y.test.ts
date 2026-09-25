import { checkA11y, expectNoA11yViolations, formatViolations } from "./a11y";

// Issue #169: the checker itself has to be trustworthy — these fixtures prove
// that each rule fires on the markup it is meant to catch, that clean markup
// passes, and that a documented exception actually suppresses a finding.

function fixture(html: string): Element {
  const root = document.createElement("div");
  root.innerHTML = html;
  return root;
}

function rulesOf(root: Element, options?: Parameters<typeof checkA11y>[1]): string[] {
  return [...new Set(checkA11y(root, options).map((violation) => violation.rule))].sort();
}

describe("checkA11y", () => {
  it("accepts markup that follows every rule", () => {
    const root = fixture(`
      <main>
        <h1>Quorum</h1>
        <p>Create and vote on Stellar proposals.</p>
        <a href="/proposals">Proposals</a>
        <button type="button">Connect Wallet</button>
        <label for="title">Title</label>
        <input id="title" type="text" />
        <svg aria-hidden="true" focusable="false"></svg>
        <img src="/badge.svg" alt="Stellar badge" />
      </main>
    `);
    expect(checkA11y(root)).toEqual([]);
    expect(() => expectNoA11yViolations(root)).not.toThrow();
  });

  it("flags every rule the suite enforces", () => {
    const root = fixture(`
      <h1>Page title</h1>
      <h3>Skipped a level</h3>
      <h2></h2>
      <div id="dup">one</div>
      <span id="dup">two</span>
      <a href="#">nowhere</a>
      <a href="/ok"></a>
      <button></button>
      <img src="x.png" />
      <input type="text" />
      <div tabindex="3">jump the queue</div>
      <div aria-owns="missing-id">orphan</div>
      <span aria-bogus="1">typo</span>
      <div aria-hidden="true"><a href="/secret">secret</a></div>
      <a href="/outer"><button>nested</button></a>
      <div role="checkbox">toggle</div>
      <svg role="img"></svg>
      <div>not in a landmark</div>
    `);

    expect(rulesOf(root)).toEqual([
      "aria-hidden-focus",
      "aria-required-attr",
      "aria-valid-attr",
      "aria-valid-attr-value",
      "button-name",
      "duplicate-id",
      "empty-heading",
      "heading-order",
      "image-alt",
      "label",
      "link-href",
      "link-name",
      "nested-interactive",
      "region",
      "svg-img-alt",
      "tabindex",
    ]);
  });

  it("does not report hidden subtrees for rules about visible content", () => {
    const root = fixture(`
      <div aria-hidden="true">
        <img src="x.png" />
        <a href="/secret">secret</a>
        <h3>hidden heading</h3>
      </div>
    `);
    expect(rulesOf(root)).toEqual(["aria-hidden-focus"]);
  });

  it("keeps heading order comparisons between consecutive headings", () => {
    expect(rulesOf(fixture("<main><h1>One</h1><h2>Two</h2></main>"))).toEqual([]);
    expect(rulesOf(fixture("<main><h1>One</h1><h3>Three</h3></main>"))).toEqual(["heading-order"]);
    expect(rulesOf(fixture("<main><h3>Start</h3></main>"))).toEqual([]);
  });

  it("throws a readable error when violations are asserted away", () => {
    const root = fixture('<img src="x.png" />');
    expect(() => expectNoA11yViolations(root)).toThrow(/Found 1 accessibility violation/);
    expect(formatViolations(checkA11y(root))).toContain("[image-alt]");
  });

  it("suppresses a violation covered by a matching exception", () => {
    const root = fixture("<div>orphan</div>");
    expect(rulesOf(root)).toEqual(["region"]);

    const suppressed = checkA11y(root, {
      exceptions: [
        { rule: "region", selector: "*", reason: "component rendered in isolation" },
      ],
    });
    expect(suppressed).toEqual([]);
  });

  it("keeps reporting when the exception does not match", () => {
    const root = fixture("<div>orphan</div>");
    expect(rulesOf(root, { exceptions: [{ rule: "image-alt", selector: "*", reason: "unrelated" }] })).toEqual([
      "region",
    ]);
    expect(
      rulesOf(root, { exceptions: [{ rule: "region", selector: "img", reason: "wrong element" }] }),
    ).toEqual(["region"]);
  });
});
