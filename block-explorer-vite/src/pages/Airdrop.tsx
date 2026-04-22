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
import { shortHash } from '../lib/format'

function formatUptime(seconds: number) {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  if (days > 0) return `${days}d ${hours}h`
  const mins = Math.floor((seconds % 3600) / 60)
  return `${hours}h ${mins}m`
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
        Estadisticas del reparto de recompensas a nodos: cuantos hay, cuantos pueden cobrar, tramos
        y nodos elegibles segun reglas del nodo.
      </PageIntro>

      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          {[
            { label: 'Nodos totales', value: stats.total_nodes },
            { label: 'Elegibles', value: stats.eligible_nodes },
            { label: 'Reclamados', value: stats.claimed_nodes },
            { label: 'Distribuidos', value: stats.total_distributed },
          ].map((s) => (
            <div key={s.label} className="bg-white border border-neutral-200 rounded-2xl p-4">
              <p className="text-neutral-500 text-xs uppercase tracking-wide">{s.label}</p>
              <p className="text-2xl font-bold text-neutral-900 mt-1">{s.value}</p>
            </div>
          ))}
        </div>
      )}

      {tiers.length > 0 && (
        <>
          <h2 className="text-lg font-semibold text-neutral-900 mb-1">Tramos</h2>
          <p className="text-xs text-neutral-400 mb-4">Reglas de cantidad segun altura de bloque y tiempo activo.</p>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
            {tiers.map((t) => (
              <div key={t.tier_id} className="bg-white border border-neutral-200 rounded-2xl p-4 text-left">
                <p className="text-main-500 font-semibold">{t.name}</p>
                <p className="text-sm text-neutral-500 mt-1">Base: {t.base_amount} tokens</p>
                <p className="text-xs text-neutral-400 mt-1">
                  +{t.bonus_per_block}/bloque, +{t.bonus_per_uptime_day}/dia activo
                </p>
                <p className="text-xs text-neutral-400">
                  Bloques {t.min_block_index}–{t.max_block_index}
                </p>
              </div>
            ))}
          </div>
        </>
      )}

      <h2 className="text-lg font-semibold text-neutral-900 mb-1">Nodos elegibles</h2>
      <p className="text-xs text-neutral-400 mb-4">Direcciones P2P que cumplen condiciones para el reparto.</p>
      {nodes.length === 0 ? (
        <div className="bg-white border border-neutral-200 rounded-2xl p-8 text-center">
          <p className="text-neutral-500">Sin nodos elegibles aun. Mina bloques para calificar.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
                <th className="text-left py-3 px-2">Nodo</th>
                <th className="text-right py-3 px-2">Bloques</th>
                <th className="text-right py-3 px-2">Tiempo activo</th>
                <th className="text-right py-3 px-2">Tramo</th>
                <th className="text-center py-3 px-2">Reclamado</th>
              </tr>
            </thead>
            <tbody>
              {nodes.map((n) => (
                <tr key={n.node_address} className="border-b border-neutral-100">
                  <td className="py-3 px-2 font-mono text-xs text-neutral-600">{shortHash(n.node_address)}</td>
                  <td className="py-3 px-2 text-right">{n.blocks_validated}</td>
                  <td className="py-3 px-2 text-right text-neutral-500">{formatUptime(n.uptime_seconds)}</td>
                  <td className="py-3 px-2 text-right">{n.eligibility_tier}</td>
                  <td className="py-3 px-2 text-center">
                    {n.airdrop_claimed ? (
                      <span className="text-green-600 text-xs">Si</span>
                    ) : (
                      <span className="text-neutral-400 text-xs">No</span>
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
