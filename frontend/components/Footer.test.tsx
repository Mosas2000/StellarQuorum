import { render } from "@testing-library/react";
import Footer from "./Footer";
import { describeVisual } from "@/test/visual";

// Issue #168: the footer's separator runs and muted colour are a deliberate
// visual decision.
describe("Footer", () => {
  it("matches the visual baseline", () => {
    const { container } = render(<Footer />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });
});
