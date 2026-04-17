import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { getBlocks, getStats, type Block, type Stats } from '../lib/api'
import PageIntro from '../components/PageIntro'
import SearchBar from '../components/SearchBar'
import ServerStatus from '../components/ServerStatus'

function shortHash(h: string) {
  return h.length > 16 ? h.slice(0, 8) + '...' + h.slice(-8) : h
}

function timeAgo(ts: number) {
  const diff = Math.floor(Date.now() / 1000 - ts)
  if (diff < 60) return `${diff}s ago`
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
  return new Date(ts * 1000).toLocaleDateString()
}

export default function Home() {
  const [blocks, setBlocks] = useState<Block[]>([])
  const [stats, setStats] = useState<Stats | null>(null)

  useEffect(() => {
    const load = () => {
      getBlocks().then((b) => setBlocks(b.slice(-10).reverse())).catch(() => {})
      getStats().then(setStats).catch(() => {})
    }
    load()
    const id = setInterval(load, 10000)
    return () => clearInterval(id)
  }, [])

  return (
    <>
      <ServerStatus />
      <PageIntro title="Dashboard">
        Chain overview — block height, mempool, peer connections.
        Search by hash to jump directly to a block, wallet, or contract.
      </PageIntro>
      <SearchBar />

      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-10">
          {[
            { label: 'Blocks', value: stats.blockchain.block_count, icon: '⬡' },
            { label: 'Transactions', value: stats.blockchain.total_transactions, icon: '⇄' },
            { label: 'Pending', value: stats.mempool.pending_transactions, icon: '◷' },
            { label: 'Peers', value: stats.network.connected_peers, icon: '◉' },
          ].map((s) => (
            <div
              key={s.label}
              className="bg-white border border-neutral-200 rounded-2xl p-5
                         shadow-sm hover:shadow-md transition-all duration-200"
            >
              <div className="flex items-center justify-between">
                <p className="text-neutral-400 text-xs font-semibold uppercase tracking-wider">
                  {s.label}
                </p>
                <span className="text-neutral-300 text-lg">{s.icon}</span>
              </div>
              <p className="text-3xl font-bold text-neutral-900 mt-2">{s.value}</p>
            </div>
          ))}
        </div>
      )}

      <div className="bg-white border border-neutral-200 rounded-2xl shadow-sm overflow-hidden">
        <div className="px-5 py-4 border-b border-neutral-100">
          <h2 className="text-lg font-semibold text-neutral-900">Latest Blocks</h2>
          <p className="text-xs text-neutral-400 mt-0.5">Click on a hash to view details and transactions.</p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-400 text-xs uppercase border-b border-neutral-100 bg-surface-alt">
                <th className="text-left py-3 px-5 font-semibold">#</th>
                <th className="text-left py-3 px-5 font-semibold">Hash</th>
                <th className="text-right py-3 px-5 font-semibold">Txns</th>
                <th className="text-right py-3 px-5 font-semibold">Time</th>
              </tr>
            </thead>
            <tbody>
              {blocks.map((b) => (
                <tr
                  key={b.hash}
                  className="border-b border-neutral-50 hover:bg-main-50/50 transition-colors duration-150"
                >
                  <td className="py-3.5 px-5 text-neutral-900 font-semibold">{b.index}</td>
                  <td className="py-3.5 px-5">
                    <Link
                      to={`/block/${b.hash}`}
                      className="text-main-500 hover:text-main-600 font-mono text-xs
                                 hover:underline transition-colors"
                    >
                      {shortHash(b.hash)}
                    </Link>
                  </td>
                  <td className="py-3.5 px-5 text-right text-neutral-600">{b.transactions.length}</td>
                  <td className="py-3.5 px-5 text-right text-neutral-400">{timeAgo(b.timestamp)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {blocks.length === 0 && (
            <p className="text-neutral-400 text-center py-12">
              No blocks yet. Mine one via the API to populate the chain.
            </p>
          )}
        </div>
      </div>
    </>
  )
}
