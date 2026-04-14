import { useEffect, useState } from 'react'
import PageIntro from '../components/PageIntro'
import {
  getAirdropStatistics,
  getAirdropTiers,
  getEligibleNodes,
  type AirdropStatistics,
  type AirdropTier,
  type NodeTracking,
} from '../lib/api'

function formatUptime(seconds: number) {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  if (days > 0) return `${days}d ${hours}h`
  const mins = Math.floor((seconds % 3600) / 60)
  return `${hours}h ${mins}m`
}

function shortAddr(a: string) {
  return a.length > 16 ? a.slice(0, 8) + '...' + a.slice(-8) : a
}

export default function Airdrop() {
  const [stats, setStats] = useState<AirdropStatistics | null>(null)
  const [tiers, setTiers] = useState<AirdropTier[]>([])
  const [nodes, setNodes] = useState<NodeTracking[]>([])

  useEffect(() => {
    getAirdropStatistics().then(setStats).catch(() => {})
    getAirdropTiers().then(setTiers).catch(() => {})
    getEligibleNodes().then(setNodes).catch(() => {})
  }, [])

  return (
    <>
      <PageIntro title="Airdrop">
        Estadísticas del reparto de recompensas a nodos: cuántos hay, cuántos pueden cobrar, tramos
        (tiers) y nodos elegibles según reglas del nodo.
      </PageIntro>

      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          {[
            { label: 'Total Nodes', value: stats.total_nodes },
            { label: 'Eligible', value: stats.eligible_nodes },
            { label: 'Claimed', value: stats.claimed_nodes },
            { label: 'Distributed', value: stats.total_distributed },
          ].map((s) => (
            <div key={s.label} className="bg-gray-900 border border-gray-800 rounded-xl p-4">
              <p className="text-gray-400 text-xs uppercase tracking-wide">{s.label}</p>
              <p className="text-2xl font-bold text-white mt-1">{s.value}</p>
            </div>
          ))}
        </div>
      )}

      {tiers.length > 0 && (
        <>
          <h2 className="text-lg font-semibold text-white mb-1">Tramos (tiers)</h2>
          <p className="text-xs text-gray-500 mb-4">Reglas de cantidad según altura de bloque y tiempo activo.</p>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
            {tiers.map((t) => (
              <div key={t.tier_id} className="bg-gray-900 border border-gray-800 rounded-xl p-4 text-left">
                <p className="text-cyan-400 font-semibold">{t.name}</p>
                <p className="text-sm text-gray-400 mt-1">Base: {t.base_amount} coins</p>
                <p className="text-xs text-gray-500 mt-1">
                  +{t.bonus_per_block}/block, +{t.bonus_per_uptime_day}/day uptime
                </p>
                <p className="text-xs text-gray-500">
                  Blocks {t.min_block_index}–{t.max_block_index}
                </p>
              </div>
            ))}
          </div>
        </>
      )}

      <h2 className="text-lg font-semibold text-white mb-1">Nodos elegibles</h2>
      <p className="text-xs text-gray-500 mb-4">Direcciones P2P que cumplen condiciones para el reparto.</p>
      {nodes.length === 0 ? (
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-8 text-center">
          <p className="text-gray-400">No eligible nodes yet. Mine blocks to qualify.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-gray-400 text-xs uppercase border-b border-gray-800">
                <th className="text-left py-3 px-2">Node</th>
                <th className="text-right py-3 px-2">Blocks</th>
                <th className="text-right py-3 px-2">Uptime</th>
                <th className="text-right py-3 px-2">Tier</th>
                <th className="text-center py-3 px-2">Claimed</th>
              </tr>
            </thead>
            <tbody>
              {nodes.map((n) => (
                <tr key={n.node_address} className="border-b border-gray-800/50">
                  <td className="py-3 px-2 font-mono text-xs text-gray-300">{shortAddr(n.node_address)}</td>
                  <td className="py-3 px-2 text-right">{n.blocks_validated}</td>
                  <td className="py-3 px-2 text-right text-gray-400">{formatUptime(n.uptime_seconds)}</td>
                  <td className="py-3 px-2 text-right">{n.eligibility_tier}</td>
                  <td className="py-3 px-2 text-center">
                    {n.airdrop_claimed ? (
                      <span className="text-green-400 text-xs">Yes</span>
                    ) : (
                      <span className="text-gray-500 text-xs">No</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  )
}
