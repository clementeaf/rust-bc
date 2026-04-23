import axios from 'axios';

const API_URL = '/api/v1';

const client = axios.create({ baseURL: API_URL, timeout: 10000 });

function unwrap<T>(body: unknown): T {
  const r = body as Record<string, unknown>;
  if (r.status === 'Success' && r.data != null) return r.data as T;
  if (r.success === true && r.data != null) return r.data as T;
  const msg =
    typeof r.message === 'string' ? r.message : 'Request failed';
  throw new Error(msg);
}

// ── Types ────────────────────────────────────────────────────────────────────

export interface Block {
  index: number;
  timestamp: number;
  transactions: Transaction[];
  previous_hash: string;
  hash: string;
  nonce: number;
  difficulty: number;
  merkle_root: string;
}

export interface Transaction {
  id: string;
  from: string;
  to: string;
  amount: number;
  fee: number;
  data?: string | null;
  timestamp: number;
  signature: string;
}

export interface Wallet {
  address: string;
  balance: number;
  public_key?: string;
}

export interface Stats {
  blockchain: {
    block_count: number;
    total_transactions: number;
    difficulty: number;
    latest_block_hash: string;
    latest_block_index: number;
    total_coinbase: number;
    unique_addresses: number;
  };
  mempool: { pending_transactions: number; total_fees_pending: number };
  network: { connected_peers: number };
}

export interface Validator {
  address: string;
  staked_amount: number;
  is_active: boolean;
  total_rewards: number;
  created_at: number;
  last_validated_block: number;
  validation_count: number;
  slash_count: number;
  unstaking_requested: boolean;
  unstake_start_time: number | null;
}

export interface SmartContract {
  address: string;
  code: string;
  state: Record<string, unknown>;
  created_at: number;
  updated_at: number;
  update_sequence: number;
}

export interface AirdropStatistics {
  total_nodes: number;
  eligible_nodes: number;
  claimed_nodes: number;
  pending_claims: number;
  total_distributed: number;
  airdrop_amount_per_node: number;
  max_eligible_nodes: number;
  tiers_count: number;
}

export interface AirdropTier {
  tier_id: number;
  name: string;
  min_block_index: number;
  max_block_index: number;
  base_amount: number;
  bonus_per_block: number;
  bonus_per_uptime_day: number;
}

export interface NodeTracking {
  node_address: string;
  blocks_validated: number;
  is_eligible: boolean;
  airdrop_claimed: boolean;
  eligibility_tier: number;
  uptime_seconds: number;
}

export interface IdentityRecord {
  did: string;
  created_at: number;
  updated_at: number;
  status: string;
}

export interface Credential {
  id: string;
  issuer_did: string;
  subject_did: string;
  cred_type: string;
  issued_at: number;
  expires_at: number;
  revoked_at: number | null;
}

// ── API calls ───────────────────────────────────────────────────────────────

export const getBlocks = () =>
  client.get('/blocks').then((r) => unwrap<Block[]>(r.data));

export const getBlockByHash = (hash: string) =>
  client.get(`/blocks/${hash}`).then((r) => unwrap<Block>(r.data));

export const getBlockByIndex = (idx: number) =>
  client.get(`/blocks/index/${idx}`).then((r) => unwrap<Block>(r.data));

export const getWallet = (addr: string) =>
  client.get(`/wallets/${addr}`).then((r) => unwrap<Wallet>(r.data));

export const getWalletTransactions = (addr: string) =>
  client.get(`/wallets/${addr}/transactions`).then((r) => {
    try {
      return unwrap<Transaction[]>(r.data);
    } catch {
      return [];
    }
  });

export const getStats = () =>
  client.get('/stats').then((r) => unwrap<Stats>(r.data));

export const getMempool = () =>
  client.get('/mempool').then((r) => {
    const d = unwrap<{ count: number; transactions: Transaction[] } | Transaction[]>(r.data);
    return Array.isArray(d) ? d : d.transactions;
  });

export const getValidators = () =>
  client.get('/staking/validators').then((r) => {
    try {
      return unwrap<Validator[]>(r.data);
    } catch {
      return [];
    }
  });

export const getAllContracts = () =>
  client.get('/contracts').then((r) => {
    try {
      return unwrap<SmartContract[]>(r.data);
    } catch {
      return [];
    }
  });

export const getContract = (addr: string) =>
  client.get(`/contracts/${addr}`).then((r) => unwrap<SmartContract>(r.data));

export const getAirdropStatistics = () =>
  client.get('/airdrop/statistics').then((r) => unwrap<AirdropStatistics>(r.data));

export const getAirdropTiers = () =>
  client.get('/airdrop/tiers').then((r) => {
    try {
      return unwrap<AirdropTier[]>(r.data);
    } catch {
      return [];
    }
  });

export const getEligibleNodes = () =>
  client.get('/airdrop/eligible').then((r) => {
    try {
      return unwrap<NodeTracking[]>(r.data);
    } catch {
      return [];
    }
  });

export const searchByHash = async (
  hash: string,
): Promise<{ type: 'block' | 'contract' | 'wallet'; id: string }> => {
  try {
    await getBlockByHash(hash);
    return { type: 'block', id: hash };
  } catch {
    try {
      await getContract(hash);
      return { type: 'contract', id: hash };
    } catch {
      await getWallet(hash);
      return { type: 'wallet', id: hash };
    }
  }
};

// Wallet creation & mining (for demo)
export const createWallet = () =>
  client.post('/wallets/create', {}).then((r) => unwrap<Wallet>(r.data));

export const mineBlock = (minerAddress: string) =>
  client.post('/mine', { miner_address: minerAddress }).then((r) => r.data);

export const sendTransaction = (from: string, to: string, amount: number, fee: number) =>
  client.post('/transactions', { from, to, amount, fee }).then((r) => r.data);

// ── Identity & Credentials ──────────────────────────────────────────────────

export const createIdentity = (did: string, status: string) => {
  const now = Math.floor(Date.now() / 1000);
  return client
    .post('/store/identities', {
      did,
      created_at: now,
      updated_at: now,
      status,
    })
    .then((r) => unwrap<IdentityRecord>(r.data));
};

export const getIdentity = (did: string) =>
  client.get(`/store/identities/${encodeURIComponent(did)}`).then((r) => unwrap<IdentityRecord>(r.data));

export const createCredential = (
  id: string,
  issuer_did: string,
  subject_did: string,
  cred_type: string,
  issued_at: number,
  expires_at: number,
) =>
  client
    .post('/store/credentials', {
      id,
      issuer_did,
      subject_did,
      cred_type,
      issued_at,
      expires_at,
      revoked_at: null,
    })
    .then((r) => unwrap<Credential>(r.data));

export const getCredential = (credId: string) =>
  client.get(`/store/credentials/${encodeURIComponent(credId)}`).then((r) => unwrap<Credential>(r.data));

export const getCredentialsBySubject = (subjectDid: string) =>
  client
    .get(`/store/credentials/by-subject/${encodeURIComponent(subjectDid)}`)
    .then((r) => unwrap<Credential[]>(r.data));

// ── Staking ──────────────────────────────────────────────────────────────────

export const stakeTokens = (address: string, amount: number) =>
  client.post('/staking/stake', { address, amount }).then((r) => r.data);

export const requestUnstake = (address: string) =>
  client.post('/staking/unstake', { address }).then((r) => r.data);

// ── Channels ─────────────────────────────────────────────────────────────────

export interface Channel {
  channel_id: string;
}

export interface ChannelConfig {
  channel_id: string;
  member_orgs: string[];
  anchor_peers: string[];
  max_block_size: number;
  batch_timeout_ms: number;
}

export const listChannels = () =>
  client.get('/channels/list').then((r) => {
    try {
      return unwrap<Channel[]>(r.data);
    } catch {
      return [];
    }
  });

export const createChannel = (channelId: string) =>
  client.post('/channels/create', { channel_id: channelId }).then((r) => r.data);

export const getChannelConfig = (channelId: string) =>
  client
    .get(`/channels/${encodeURIComponent(channelId)}/config`)
    .then((r) => unwrap<ChannelConfig>(r.data));

// ── Governance ──────────────────────────────────────────────────────────────

export interface Proposal {
  id: number;
  proposer: string;
  action: any;
  status: 'Voting' | 'Passed' | 'Rejected' | 'Executed' | 'Cancelled' | 'Expired';
  deposit: number;
  submitted_at: number;
  voting_ends_at: number;
  timelock_ends_at: number | null;
  finalized_at: number | null;
  description: string;
}

export interface Vote {
  voter: string;
  proposal_id: number;
  option: 'Yes' | 'No' | 'Abstain';
  power: number;
  voted_at: number;
}

export interface TallyResult {
  proposal_id: number;
  yes_power: number;
  no_power: number;
  abstain_power: number;
  total_voted_power: number;
  total_staked_power: number;
  quorum_reached: boolean;
  passed: boolean;
}

export interface ProtocolParam {
  key: string;
  value: string;
  raw: any;
}

export const getGovernanceParams = () =>
  client.get('/governance/params').then((r) => unwrap<ProtocolParam[]>(r.data));

export const getProposals = (status?: string) =>
  client.get(`/governance/proposals${status ? `?status=${status}` : ''}`).then((r) => unwrap<Proposal[]>(r.data));

export const getProposal = (id: number) =>
  client.get(`/governance/proposals/${id}`).then((r) => unwrap<Proposal>(r.data));

export const submitProposal = (data: { proposer: string; description: string; deposit: number; action: any }) =>
  client.post('/governance/proposals', data).then((r) => unwrap<Proposal>(r.data));

export const castVote = (proposalId: number, data: { voter: string; option: string; power: number }) =>
  client.post(`/governance/proposals/${proposalId}/vote`, data).then((r) => unwrap<Vote[]>(r.data));

export const getVotes = (proposalId: number) =>
  client.get(`/governance/proposals/${proposalId}/votes`).then((r) => unwrap<Vote[]>(r.data));

export const tallyVotes = (proposalId: number) =>
  client.get(`/governance/proposals/${proposalId}/tally`).then((r) => unwrap<TallyResult>(r.data));
