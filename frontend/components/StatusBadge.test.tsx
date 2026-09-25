import { render } from "@testing-library/react";
import StatusBadge from "./StatusBadge";
import { describeVisual } from "@/test/visual";

// Issue #168: the badge carries the status colours of the whole UI. Every
// status is snapshotted so a colour, border or label change surfaces as a
// reviewable baseline diff instead of slipping through review.
describe("StatusBadge", () => {
  it("matches the visual baseline when active", () => {
    const { container } = render(<StatusBadge status="active" />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline when passed", () => {
    const { container } = render(<StatusBadge status="passed" />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline when failed", () => {
    const { container } = render(<StatusBadge status="failed" />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline when pending", () => {
    const { container } = render(<StatusBadge status="pending" />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline when executed", () => {
    const { container } = render(<StatusBadge status="executed" />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline when cancelled", () => {
    const { container } = render(<StatusBadge status="cancelled" />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });
});
