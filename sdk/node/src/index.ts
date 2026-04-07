/**
 * @rust-bc/sdk — Node.js client for rust-bc blockchain network.
 *
 * Usage:
 *   const { Gateway } = require('@rust-bc/sdk');
 *   const gw = new Gateway({ url: 'https://localhost:8080' });
 *   const result = await gw.submitTransaction('mycc', 'mychannel', { ... });
 */

import https from "https";
import WebSocket from "ws";

// ── Types ────────────────────────────────────────────────────────────────────

export interface GatewayOptions {
  /** Base URL of the rust-bc node API (e.g. "https://localhost:8080"). */
  url: string;
  /** Organization ID sent via X-Org-Id header. */
  orgId?: string;
  /** MSP role sent via X-Msp-Role header. */
  mspRole?: string;
  /** Channel ID sent via X-Channel-Id header. */
  channelId?: string;
  /** Skip TLS certificate verification (default: true for self-signed certs). */
  insecure?: boolean;
}

export interface TransactionInput {
  id: string;
  input_did: string;
  output_recipient: string;
  amount: number;
}

export interface TxResult {
  tx_id: string;
  block_height: number;
  valid: boolean;
}

export interface ApiResponse<T> {
  status: string;
  status_code: number;
  message: string;
  data?: T;
  error?: string;
  timestamp: string;
  trace_id: string;
}

export interface Organization {
  org_id: string;
  msp_id: string;
  admin_dids: string[];
  member_dids: string[];
  root_public_keys: number[][];
}

export interface EndorsementPolicy {
  [key: string]: unknown;
}

export interface BlockEvent {
  channel_id: string;
  height: number;
  tx_count?: number;
  tx_id?: string;
  block_height?: number;
  valid?: boolean;
}

export interface PrivateDataResult {
  collection: string;
  key: string;
  hash?: string;
  value?: string;
}

export interface SimulateResult {
  result: string;
  rwset: { reads: unknown[]; writes: unknown[] };
}

// ── HTTP Client ──────────────────────────────────────────────────────────────

class HttpClient {
  private baseUrl: string;
  private headers: Record<string, string>;
  private agent: https.Agent;

  constructor(opts: GatewayOptions) {
    this.baseUrl = `${opts.url}/api/v1`;
    this.headers = { "Content-Type": "application/json" };
    if (opts.orgId) this.headers["X-Org-Id"] = opts.orgId;
    if (opts.mspRole) this.headers["X-Msp-Role"] = opts.mspRole;
    if (opts.channelId) this.headers["X-Channel-Id"] = opts.channelId;
    this.agent = new https.Agent({ rejectUnauthorized: !(opts.insecure ?? true) });
  }

  async get<T>(path: string): Promise<ApiResponse<T>> {
    return this.request<T>("GET", path);
  }

  async post<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>("POST", path, body);
  }

  async put<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>("PUT", path, body);
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}/${path}`;
    const resp = await fetch(url, {
      method,
      headers: this.headers,
      body: body ? JSON.stringify(body) : undefined,
      // @ts-expect-error — Node 18+ fetch supports agent via dispatcher
      dispatcher: this.agent,
    });
    return (await resp.json()) as ApiResponse<T>;
  }
}

// ── Gateway ──────────────────────────────────────────────────────────────────

export class Gateway {
  private http: HttpClient;
  private opts: GatewayOptions;

  constructor(opts: GatewayOptions) {
    this.opts = opts;
    this.http = new HttpClient(opts);
  }

  // ── Transactions ───────────────────────────────────────────────────────

  /** Submit a transaction through the full endorse -> order -> commit pipeline. */
  async submitTransaction(
    chaincodeId: string,
    tx: TransactionInput,
    channelId?: string
  ): Promise<TxResult> {
    const resp = await this.http.post<TxResult>("gateway/submit", {
      chaincode_id: chaincodeId,
      channel_id: channelId ?? "",
      transaction: tx,
    });
    if (!resp.data) throw new Error(resp.error ?? resp.message);
    return resp.data;
  }

  /** Evaluate a transaction (read-only, no commit). */
  async evaluate(
    chaincodeId: string,
    version: string,
    func: string
  ): Promise<SimulateResult> {
    const resp = await this.http.post<SimulateResult>(
      `chaincode/${chaincodeId}/simulate?version=${version}`,
      { function: func }
    );
    if (!resp.data) throw new Error(resp.error ?? resp.message);
    return resp.data;
  }

  // ── Organizations ──────────────────────────────────────────────────────

  /** Register an organization. */
  async registerOrg(org: Organization): Promise<Organization> {
    const resp = await this.http.post<Organization>("store/organizations", org);
    if (!resp.data) throw new Error(resp.error ?? resp.message);
    return resp.data;
  }

  /** List all organizations. */
  async listOrgs(): Promise<Organization[]> {
    const resp = await this.http.get<Organization[]>("store/organizations");
    return resp.data ?? [];
  }

  // ── Policies ───────────────────────────────────────────────────────────

  /** Set an endorsement policy. */
  async setPolicy(resourceId: string, policy: EndorsementPolicy): Promise<void> {
    const resp = await this.http.post("store/policies", {
      resource_id: resourceId,
      policy,
    });
    if (resp.status_code !== 200) throw new Error(resp.error ?? resp.message);
  }

  // ── Channels ───────────────────────────────────────────────────────────

  /** Create a new channel. */
  async createChannel(channelId: string): Promise<void> {
    const resp = await this.http.post("channels", { channel_id: channelId });
    if (resp.status_code !== 200 && resp.status_code !== 201) {
      throw new Error(resp.error ?? resp.message);
    }
  }

  // ── Private Data ───────────────────────────────────────────────────────

  /** Store private data in a collection. */
  async putPrivateData(
    collection: string,
    key: string,
    value: string
  ): Promise<PrivateDataResult> {
    const resp = await this.http.put<PrivateDataResult>(
      `private-data/${collection}/${key}`,
      { value }
    );
    if (!resp.data) throw new Error(resp.error ?? resp.message);
    return resp.data;
  }

  /** Retrieve private data from a collection. */
  async getPrivateData(
    collection: string,
    key: string
  ): Promise<PrivateDataResult> {
    const resp = await this.http.get<PrivateDataResult>(
      `private-data/${collection}/${key}`
    );
    if (!resp.data) throw new Error(resp.error ?? resp.message);
    return resp.data;
  }

  // ── Events (WebSocket) ─────────────────────────────────────────────────

  /** Subscribe to block events via WebSocket. Returns an unsubscribe function. */
  subscribeBlocks(
    onEvent: (event: BlockEvent) => void,
    onError?: (err: Error) => void
  ): () => void {
    const wsUrl = this.opts.url.replace(/^https/, "wss").replace(/^http/, "ws");
    const ws = new WebSocket(`${wsUrl}/api/v1/events/blocks`, {
      rejectUnauthorized: !(this.opts.insecure ?? true),
      headers: {
        ...(this.opts.orgId ? { "X-Org-Id": this.opts.orgId } : {}),
        ...(this.opts.channelId ? { "X-Channel-Id": this.opts.channelId } : {}),
      },
    });

    ws.on("message", (data: WebSocket.Data) => {
      try {
        const event = JSON.parse(data.toString()) as BlockEvent;
        onEvent(event);
      } catch (e) {
        onError?.(e instanceof Error ? e : new Error(String(e)));
      }
    });

    ws.on("error", (err: Error) => onError?.(err));

    return () => ws.close();
  }

  // ── Block Polling ──────────────────────────────────────────────────────

  /** Poll for blocks from a given height (REST long-polling). */
  async pollBlocks(fromHeight: number): Promise<unknown[]> {
    const resp = await this.http.get<unknown[]>(
      `events/blocks?from_height=${fromHeight}`
    );
    return resp.data ?? [];
  }

  // ── Health ─────────────────────────────────────────────────────────────

  /** Check node health. */
  async health(): Promise<{ status: string }> {
    const resp = await this.http.get<{ status: string }>("health");
    return resp.data ?? { status: "unknown" };
  }
}

export default Gateway;
