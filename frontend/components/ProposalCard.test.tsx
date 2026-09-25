import { render } from "@testing-library/react";
import ProposalCard from "./ProposalCard";
import type { Proposal } from "@/lib/types";
import { describeVisual } from "@/test/visual";

// Issue #168: the card is the list page's whole visual language — status chip,
// category chip, quorum bar and the For/Against summary. The clock is pinned
// so the relative deadline text is part of the baseline rather than a flake.
const NOW = Date.parse("2026-06-01T00:00:00.000Z");

function proposal(overrides: Partial<Proposal>): Proposal {
  return {
    id: "QIP-001",
    title: "Increase Protocol Fee to 0.05%",
    description: "Raises the base protocol fee from 0.03% to 0.05%.",
    proposer: "GABC...3XZK",
    status: "active",
    startTime: "2026-05-25T00:00:00Z",
    endTime: "2026-06-06T00:00:00Z",
    forVotes: 600000,
    againstVotes: 300000,
    abstainVotes: 100000,
    quorumRequired: 500000,
    category: "Financial",
    actions: [],
    votes: [],
    ...overrides,
  };
}

describe("ProposalCard", () => {
  beforeEach(() => {
    jest.spyOn(Date, "now").mockReturnValue(NOW);
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("matches the visual baseline for an active proposal with votes", () => {
    const { container } = render(<ProposalCard proposal={proposal({})} />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline for a pending proposal without votes", () => {
    const pending = proposal({
      id: "QIP-007",
      title: "Reduce Quorum Threshold to 4%",
      proposer: "GCOMM...7RST",
      status: "pending",
      endTime: "2026-06-11T00:00:00Z",
      forVotes: 0,
      againstVotes: 0,
      abstainVotes: 0,
      category: "Governance",
    });
    const { container } = render(<ProposalCard proposal={pending} />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });

  it("matches the visual baseline for a proposal that has ended", () => {
    const passed = proposal({
      id: "QIP-002",
      title: "Add USDC as Accepted Collateral",
      status: "passed",
      endTime: "2026-05-29T00:00:00Z",
    });
    const { container } = render(<ProposalCard proposal={passed} />);
    expect(describeVisual(container.firstChild)).toMatchSnapshot();
  });
});
