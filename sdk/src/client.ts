import { SorobanRpc, Contract, TransactionBuilder, BASE_FEE, nativeToScVal, scValToNative, Address, Account, xdr } from '@stellar/stellar-sdk';
import type { Proposal, GovernanceConfig, QuorumClientConfig, VoteSupport } from './types';

/**
 * Source account used for read-only simulation.
 *
 * Simulating a contract call still requires a source account to build the
 * transaction, but a simulation is never signed or submitted, so the all-zero
 * ed25519 account works and means reads need no funded account.
 */
const READ_ONLY_SOURCE = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';

export class QuorumClient {
  private server: SorobanRpc.Server;
  private governance: Contract;
  private config: QuorumClientConfig;

  constructor(config: QuorumClientConfig) {
    this.config = config;
    this.server = new SorobanRpc.Server(config.rpcUrl);
    this.governance = new Contract(config.governanceContractId);
  }

  // ─── Read ────────────────────────────────────────────────────────────────

  async getProposal(id: bigint): Promise<Proposal | null> {
    // TODO: implement via simulateTransaction → governance.get_proposal(id)
    throw new Error('Not implemented');
  }

  async getProposalCount(): Promise<bigint> {
    // TODO: implement via simulateTransaction → governance.get_proposal_count()
    throw new Error('Not implemented');
  }

  async getConfig(): Promise<GovernanceConfig> {
    // TODO: implement via simulateTransaction → governance.get_config()
    throw new Error('Not implemented');
  }

  async hasVoted(proposalId: bigint, voter: string): Promise<boolean> {
    // has_voted and get_vote read the same storage entry, so one round trip
    // answers both questions.
    return (await this.getVote(proposalId, voter)) !== null;
  }

  /**
   * The choice a voter recorded on a proposal, or `null` if they have not
   * voted.
   *
   * @param proposalId Proposal to look up.
   * @param voter Stellar address of the voter.
   * @returns `0` Against, `1` For, `2` Abstain, or `null`.
   */
  async getVote(proposalId: bigint, voter: string): Promise<VoteSupport | null> {
    const support = await this.simulate<number | null | undefined>(
      'get_vote',
      nativeToScVal(proposalId, { type: 'u64' }),
      new Address(voter).toScVal(),
    );
    // The contract returns Option<u32>; None decodes to null/undefined.
    return support === null || support === undefined ? null : (support as VoteSupport);
  }

  async getProposalsByStatus(status: Proposal['status']): Promise<Proposal[]> {
    const all = await this.getAllProposals();
    return all.filter(p => p.status === status);
  }

  async getAllProposals(): Promise<Proposal[]> {
    const count = await this.getProposalCount();
    const proposals = await Promise.all(
      Array.from({ length: Number(count) }, (_, i) => this.getProposal(BigInt(i + 1)))
    );
    return proposals.filter(Boolean) as Proposal[];
  }

  // ─── Internals ───────────────────────────────────────────────────────────

  /**
   * Calls a read-only contract method through `simulateTransaction` and decodes
   * the return value.
   *
   * Nothing is signed or submitted, so this costs no fee and needs no funded
   * account.
   */
  private async simulate<T>(method: string, ...args: xdr.ScVal[]): Promise<T> {
    const source = new Account(READ_ONLY_SOURCE, '0');
    const tx = new TransactionBuilder(source, {
      fee: BASE_FEE,
      networkPassphrase: this.config.networkPassphrase,
    })
      .addOperation(this.governance.call(method, ...args))
      .setTimeout(30)
      .build();

    const simulation = await this.server.simulateTransaction(tx);

    if (SorobanRpc.Api.isSimulationError(simulation)) {
      throw new Error(`Simulation of ${method} failed: ${simulation.error}`);
    }
    if (!simulation.result) {
      throw new Error(`Simulation of ${method} returned no result`);
    }

    return scValToNative(simulation.result.retval) as T;
  }

  // ─── Transaction Builders ────────────────────────────────────────────────
  // These return unsigned XDR strings — the caller signs with Freighter and submits.

  async buildCreateProposal(proposer: string, title: string, description: string): Promise<string> {
    // TODO: build transaction → governance.create_proposal(proposer, title, description)
    throw new Error('Not implemented');
  }

  async buildVote(voter: string, proposalId: bigint, support: VoteSupport): Promise<string> {
    // TODO: build transaction → governance.vote(voter, proposalId, support)
    throw new Error('Not implemented');
  }

  async buildFinalize(proposalId: bigint): Promise<string> {
    // TODO: build transaction → governance.finalize(proposalId)
    throw new Error('Not implemented');
  }

  async buildExecute(proposalId: bigint): Promise<string> {
    // TODO: build transaction → governance.execute(proposalId)
    throw new Error('Not implemented');
  }
}

export const TESTNET: Partial<QuorumClientConfig> = {
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
};

export const MAINNET: Partial<QuorumClientConfig> = {
  rpcUrl: 'https://soroban-rpc.stellar.org',
  networkPassphrase: 'Public Global Stellar Network ; September 2015',
};
