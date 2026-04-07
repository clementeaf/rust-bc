import React, { useEffect, useState, useCallback } from "react";
import {
  fetchHealth,
  fetchStats,
  fetchBlocks,
  fetchBlock,
  fetchOrgs,
  fetchBlockTransactions,
  subscribeBlocks,
  type Block,
  type Transaction,
  type Organization,
  type NodeHealth,
} from "./api";

// ── Helpers ──────────────────────────────────────────────────────────────────

function truncHash(hash: string | number[], len = 12): string {
  const s = Array.isArray(hash) ? hash.map((b) => b.toString(16).padStart(2, "0")).join("") : String(hash);
  return s.length > len ? `${s.slice(0, len)}...` : s;
}

function timeAgo(ts: number): string {
  if (!ts) return "-";
  const diff = Math.floor(Date.now() / 1000 - ts);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

// ── Components ───────────────────────────────────────────────────────────────

function StatusBar({ health, blockCount, peers }: {
  health: NodeHealth | null;
  blockCount: number;
  peers: number;
}) {
  const statusColor = health?.status === "healthy" ? "#4caf50" : "#f44336";
  return (
    <div className="status-bar">
      <span className="status-dot" style={{ backgroundColor: statusColor }} />
      <span>{health?.status ?? "connecting..."}</span>
      <span className="sep">|</span>
      <span>Blocks: <b>{blockCount}</b></span>
      <span className="sep">|</span>
      <span>Peers: <b>{peers}</b></span>
      <span className="sep">|</span>
      <span>Network: {health?.network_id ?? "-"}</span>
    </div>
  );
}

function BlockList({ blocks, onSelect }: {
  blocks: Block[];
  onSelect: (b: Block) => void;
}) {
  return (
    <div className="panel">
      <h2>Blocks</h2>
      <table>
        <thead>
          <tr>
            <th>Height</th>
            <th>Proposer</th>
            <th>TXs</th>
            <th>Time</th>
            <th>Hash</th>
          </tr>
        </thead>
        <tbody>
          {blocks.map((b) => (
            <tr key={b.height} onClick={() => onSelect(b)} className="clickable">
              <td>#{b.height}</td>
              <td>{b.proposer}</td>
              <td>{b.transactions.length}</td>
              <td>{timeAgo(b.timestamp)}</td>
              <td className="mono">{truncHash(b.merkle_root)}</td>
            </tr>
          ))}
          {blocks.length === 0 && (
            <tr><td colSpan={5} className="empty">No blocks yet</td></tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

function BlockDetail({ block, txs }: {
  block: Block;
  txs: Transaction[];
}) {
  return (
    <div className="panel">
      <h2>Block #{block.height}</h2>
      <dl>
        <dt>Proposer</dt><dd>{block.proposer}</dd>
        <dt>Timestamp</dt><dd>{new Date(block.timestamp * 1000).toISOString()}</dd>
        <dt>Parent Hash</dt><dd className="mono">{truncHash(block.parent_hash, 24)}</dd>
        <dt>Merkle Root</dt><dd className="mono">{truncHash(block.merkle_root, 24)}</dd>
        <dt>Transactions</dt><dd>{block.transactions.length}</dd>
      </dl>
      {txs.length > 0 && (
        <table>
          <thead>
            <tr><th>TX ID</th><th>From</th><th>To</th><th>Amount</th><th>State</th></tr>
          </thead>
          <tbody>
            {txs.map((tx) => (
              <tr key={tx.id}>
                <td className="mono">{truncHash(tx.id, 16)}</td>
                <td>{tx.input_did}</td>
                <td>{tx.output_recipient}</td>
                <td>{tx.amount}</td>
                <td>{tx.state}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function OrgList({ orgs }: { orgs: Organization[] }) {
  return (
    <div className="panel">
      <h2>Organizations ({orgs.length})</h2>
      <table>
        <thead>
          <tr><th>Org ID</th><th>MSP ID</th><th>Admins</th><th>Members</th></tr>
        </thead>
        <tbody>
          {orgs.map((org) => (
            <tr key={org.org_id}>
              <td>{org.org_id}</td>
              <td>{org.msp_id}</td>
              <td>{org.admin_dids.length}</td>
              <td>{org.member_dids.length}</td>
            </tr>
          ))}
          {orgs.length === 0 && (
            <tr><td colSpan={4} className="empty">No organizations</td></tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

// ── Tabs ─────────────────────────────────────────────────────────────────────

type Tab = "blocks" | "orgs";

// ── App ──────────────────────────────────────────────────────────────────────

export default function App() {
  const [health, setHealth] = useState<NodeHealth | null>(null);
  const [blockCount, setBlockCount] = useState(0);
  const [peers, setPeers] = useState(0);
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [orgs, setOrgs] = useState<Organization[]>([]);
  const [selectedBlock, setSelectedBlock] = useState<Block | null>(null);
  const [blockTxs, setBlockTxs] = useState<Transaction[]>([]);
  const [tab, setTab] = useState<Tab>("blocks");

  // Fetch initial data
  const refresh = useCallback(async () => {
    try {
      const [h, stats, blks, orgList] = await Promise.all([
        fetchHealth(),
        fetchStats().catch(() => ({ blockchain: { block_count: 0, latest_block_hash: "" }, network: { connected_peers: 0 } })),
        fetchBlocks(0, 50).catch(() => ({ items: [], total: 0, offset: 0, limit: 50 })),
        fetchOrgs().catch(() => []),
      ]);
      setHealth(h);
      setBlockCount(stats.blockchain.block_count);
      setPeers(stats.network.connected_peers);
      setBlocks(blks.items ?? []);
      setOrgs(orgList);
    } catch {
      // Node might be down
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  }, [refresh]);

  // WebSocket for real-time block updates
  useEffect(() => {
    const unsub = subscribeBlocks(() => refresh());
    return unsub;
  }, [refresh]);

  // Load block detail
  const selectBlock = async (b: Block) => {
    setSelectedBlock(b);
    try {
      const txs = await fetchBlockTransactions(b.height);
      setBlockTxs(txs);
    } catch {
      setBlockTxs([]);
    }
  };

  return (
    <div className="app">
      <header>
        <h1>rust-bc Explorer</h1>
        <StatusBar health={health} blockCount={blockCount} peers={peers} />
      </header>

      <nav>
        <button className={tab === "blocks" ? "active" : ""} onClick={() => { setTab("blocks"); setSelectedBlock(null); }}>
          Blocks
        </button>
        <button className={tab === "orgs" ? "active" : ""} onClick={() => setTab("orgs")}>
          Organizations
        </button>
      </nav>

      <main>
        {tab === "blocks" && !selectedBlock && (
          <BlockList blocks={blocks} onSelect={selectBlock} />
        )}
        {tab === "blocks" && selectedBlock && (
          <>
            <button className="back" onClick={() => setSelectedBlock(null)}>Back to blocks</button>
            <BlockDetail block={selectedBlock} txs={blockTxs} />
          </>
        )}
        {tab === "orgs" && <OrgList orgs={orgs} />}
      </main>
    </div>
  );
}
