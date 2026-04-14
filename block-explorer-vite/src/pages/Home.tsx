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
      <PageIntro title="Inicio">
        Vista general del nodo al que apunta el proxy: altura de cadena, transacciones en mempool y
        conexiones P2P. Abajo, los últimos bloques confirmados. Usa la búsqueda para ir directo a un
        bloque, cartera o contrato.
      </PageIntro>
      <SearchBar />

      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          {[
            {
              label: 'Bloques',
              hint: 'Cantidad de bloques en la cadena local del nodo',
              value: stats.blockchain.block_count,
            },
            {
              label: 'Tx en cadena',
              hint: 'Transacciones incluidas en bloques ya minados',
              value: stats.blockchain.total_transactions,
            },
            {
              label: 'Pendientes',
              hint: 'Transacciones en el mempool (aún no incluidas en un bloque)',
              value: stats.mempool.pending_transactions,
            },
            {
              label: 'Peers',
              hint: 'Otros nodos conectados por P2P',
              value: stats.network.connected_peers,
            },
          ].map((s) => (
            <div
              key={s.label}
              title={s.hint}
              className="bg-gray-900 border border-gray-800 rounded-xl p-4 cursor-help"
            >
              <p className="text-gray-400 text-xs uppercase tracking-wide">{s.label}</p>
              <p className="text-2xl font-bold text-white mt-1">{s.value}</p>
            </div>
          ))}
        </div>
      )}

      <h2 className="text-lg font-semibold text-white mb-1">Últimos bloques</h2>
      <p className="text-xs text-gray-500 mb-4">Haz clic en el hash para ver el detalle y las transacciones.</p>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-gray-400 text-xs uppercase border-b border-gray-800">
              <th className="text-left py-3 px-2">#</th>
              <th className="text-left py-3 px-2">Hash</th>
              <th className="text-right py-3 px-2">Txns</th>
              <th className="text-right py-3 px-2">Time</th>
            </tr>
          </thead>
          <tbody>
            {blocks.map((b) => (
              <tr key={b.hash} className="border-b border-gray-800/50 hover:bg-gray-900/50">
                <td className="py-3 px-2 text-white font-medium">{b.index}</td>
                <td className="py-3 px-2">
                  <Link to={`/block/${b.hash}`} className="text-cyan-400 hover:text-cyan-300 font-mono text-xs">
                    {shortHash(b.hash)}
                  </Link>
                </td>
                <td className="py-3 px-2 text-right">{b.transactions.length}</td>
                <td className="py-3 px-2 text-right text-gray-400">{timeAgo(b.timestamp)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {blocks.length === 0 && (
          <p className="text-gray-500 text-center py-8">Aún no hay bloques. Minar uno con la API para poblar la cadena.</p>
        )}
      </div>
    </>
  )
}
