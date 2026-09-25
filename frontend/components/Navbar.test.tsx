import { render } from "@testing-library/react";
import Navbar from "./Navbar";
import { describeVisual } from "@/test/visual";

// Issue #168: the sticky header defines the chrome every page shares, so its
// link treatment and the connect button are baselined like the rest.
describe("Navbar", () => {
  it("matches the visual baseline", () => {
    const { container } = render(<Navbar />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });
});
