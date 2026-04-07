/** API client for rust-bc explorer. */

const BASE = "/api/v1";

async function get<T>(path: string): Promise<T> {
  const resp = await fetch(`${BASE}/${path}`);
  const json = await resp.json();
  return json.data ?? json;
}

// ── Types ────────────────────────────────────────────────────────────────────

export interface Block {
  height: number;
  timestamp: number;
  parent_hash: string;
  merkle_root: string;
  transactions: string[];
  proposer: string;
}

export interface Transaction {
  id: string;
  block_height: number;
  timestamp: number;
  input_did: string;
  output_recipient: string;
  amount: number;
  state: string;
}

export interface Organization {
  org_id: string;
  msp_id: string;
  admin_dids: string[];
  member_dids: string[];
}

export interface NodeHealth {
  status: string;
  node_address?: string;
  network_id?: string;
}

export interface ChainInfo {
  block_count: number;
  latest_block_hash: string;
  connected_peers: number;
}

export interface PaginatedBlocks {
  items: Block[];
  total: number;
  offset: number;
  limit: number;
}

// ── API calls ────────────────────────────────────────────────────────────────

export async function fetchHealth(): Promise<NodeHealth> {
  return get<NodeHealth>("health");
}

export async function fetchStats(): Promise<{ blockchain: ChainInfo; network: { connected_peers: number } }> {
  return get("stats");
}

export async function fetchBlocks(offset = 0, limit = 20): Promise<PaginatedBlocks> {
  return get<PaginatedBlocks>(`store/blocks?offset=${offset}&limit=${limit}`);
}

export async function fetchBlock(height: number): Promise<Block> {
  return get<Block>(`store/blocks/${height}`);
}

export async function fetchTransaction(txId: string): Promise<Transaction> {
  return get<Transaction>(`store/transactions/${txId}`);
}

export async function fetchOrgs(): Promise<Organization[]> {
  return get<Organization[]>("store/organizations");
}

export async function fetchBlockTransactions(height: number): Promise<Transaction[]> {
  return get<Transaction[]>(`store/blocks/${height}/transactions`);
}

// ── WebSocket ────────────────────────────────────────────────────────────────

export function subscribeBlocks(onBlock: (event: unknown) => void): () => void {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const ws = new WebSocket(`${protocol}//${window.location.host}/api/v1/events/blocks`);

  ws.onmessage = (evt) => {
    try {
      onBlock(JSON.parse(evt.data));
    } catch {
      // ignore parse errors
    }
  };

  return () => ws.close();
}
