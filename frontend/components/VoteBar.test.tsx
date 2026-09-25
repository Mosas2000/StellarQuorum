import { render } from "@testing-library/react";
import VoteBar from "./VoteBar";
import { describeVisual } from "@/test/visual";

// Issue #168: the bar proportions are the UI's headline number. These four
// states cover the empty track, a three-way split with its legend, the compact
// variant used on proposal cards, and a single-sided result.
describe("VoteBar", () => {
  it("matches the visual baseline with no votes cast", () => {
    const { container } = render(<VoteBar forVotes={0} againstVotes={0} abstainVotes={0} />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline for a three-way split", () => {
    const { container } = render(
      <VoteBar forVotes={600000} againstVotes={300000} abstainVotes={100000} />,
    );
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline for a three-way split in compact mode", () => {
    const { container } = render(
      <VoteBar forVotes={600000} againstVotes={300000} abstainVotes={100000} compact />,
    );
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline when only one side has votes", () => {
    const { container } = render(
      <VoteBar forVotes={800000} againstVotes={0} abstainVotes={0} />,
    );
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });
});
