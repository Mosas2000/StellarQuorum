import { PROPOSALS, getProposalById } from "./proposals";

// Issue #177: the frontend previously had no test script at all, so
// `npm test -- --passWithNoTests` at the root passed vacuously. This is a
// real regression test for getProposalById's lookup behavior.
describe("getProposalById", () => {
  it("returns the matching proposal for a known id", () => {
    const first = PROPOSALS[0];
    expect(getProposalById(first.id)).toEqual(first);
  });

  it("returns undefined for an unknown id", () => {
    expect(getProposalById("QIP-does-not-exist")).toBeUndefined();
  });

  it("is case-sensitive", () => {
    const first = PROPOSALS[0];
    expect(getProposalById(first.id.toLowerCase())).toBeUndefined();
  });
});
