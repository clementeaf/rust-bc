import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { getBlocks, getStats, type Block, type Stats } from '../lib/api'
import { timeAgo, shortHash } from '../lib/format'
import SearchBar from '../components/SearchBar'
import ServerStatus from '../components/ServerStatus'

interface HubCard {
  to: string
  title: string
  desc: string
  color: string
  tag?: string
}

const hubSections: { title: string; cards: HubCard[] }[] = [
  {
    title: 'Demos interactivos',
    cards: [
      {
        to: '/demo',
        title: 'Verificacion de credenciales para RRHH',
        desc: 'Flujo completo en 5 pasos: una institucion emite un titulo y una empresa lo verifica en segundos.',
        color: 'bg-main-500',
        tag: 'Nuevo',
      },
    ],
  },
  {
    title: 'Operaciones de la red',
    cards: [
      {
        to: '/mining',
        title: 'Mineria',
        desc: 'Crea bloques nuevos y acumula recompensas en tu wallet.',
        color: 'bg-amber-500',
      },
      {
        to: '/wallets',
        title: 'Wallets',
        desc: 'Crea cuentas, consulta balances y revisa transacciones.',
        color: 'bg-emerald-500',
      },
      {
        to: '/transactions',
        title: 'Transacciones',
        desc: 'Envia tokens entre wallets y monitorea el mempool en tiempo real.',
        color: 'bg-blue-500',
      },
    ],
  },
  {
    title: 'Identidad y credenciales',
    cards: [
      {
        to: '/identity',
        title: 'Identidades (DID)',
        desc: 'Registra identidades descentralizadas para personas, empresas o instituciones.',
        color: 'bg-violet-500',
      },
      {
        to: '/credentials',
        title: 'Credenciales verificables',
        desc: 'Emite y verifica titulos, certificaciones y constancias con firma criptografica.',
        color: 'bg-purple-500',
      },
    ],
  },
  {
    title: 'Infraestructura avanzada',
    cards: [
      {
        to: '/staking',
        title: 'Staking',
        desc: 'Bloquea tokens para participar como validador de la red.',
        color: 'bg-teal-500',
      },
      {
        to: '/channels',
        title: 'Canales privados',
        desc: 'Redes aisladas donde cada grupo tiene su propio ledger (estilo Fabric).',
        color: 'bg-indigo-500',
      },
      {
        to: '/contracts',
        title: 'Smart Contracts',
        desc: 'Contratos inteligentes ejecutados en WebAssembly con gas metering.',
        color: 'bg-rose-500',
      },
      {
        to: '/airdrop',
        title: 'Airdrop',
        desc: 'Distribucion de recompensas a nodos que participan activamente.',
        color: 'bg-cyan-500',
      },
    ],
  },
]

export default function Home() {
  const [blocks, setBlocks] = useState<Block[]>([])
  const [stats, setStats] = useState<Stats | null>(null)

  useEffect(() => {
    const load = () => {
      getBlocks().then((b) => setBlocks(b.slice(-6).reverse())).catch(() => {})
      getStats().then(setStats).catch(() => {})
    }
    load()
    const id = setInterval(load, 10000)
    return () => clearInterval(id)
  }, [])

  return (
    <>
      <ServerStatus />

      <div className="mb-8">
        <h1 className="text-3xl font-bold text-neutral-900 tracking-tight">Cerulean Ledger</h1>
        <p className="text-neutral-500 text-sm mt-2 max-w-2xl">
          Plataforma DLT con criptografia post-cuantica, identidad descentralizada, credenciales
          verificables, canales privados y smart contracts. Soberania tecnologica completa.
        </p>
      </div>

      <SearchBar />

      {/* Stats */}
      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-10">
          {[
            { label: 'Bloques', value: stats.blockchain.block_count },
            { label: 'Transacciones', value: stats.blockchain.total_transactions },
            { label: 'Pendientes', value: stats.mempool.pending_transactions },
            { label: 'Peers', value: stats.network.connected_peers },
          ].map((s) => (
            <div key={s.label} className="bg-white border border-neutral-200 rounded-2xl p-4">
              <p className="text-neutral-400 text-xs font-semibold uppercase tracking-wider">{s.label}</p>
              <p className="text-2xl font-bold text-neutral-900 mt-1">{s.value}</p>
            </div>
          ))}
        </div>
      )}

      {/* Hub cards */}
      {hubSections.map((section) => (
        <div key={section.title} className="mb-8">
          <h2 className="text-xs font-bold text-neutral-400 uppercase tracking-widest mb-3">
            {section.title}
          </h2>
          <div className={`grid gap-4 ${
            section.cards.length === 1
              ? 'grid-cols-1'
              : section.cards.length === 2
                ? 'grid-cols-1 md:grid-cols-2'
                : 'grid-cols-1 md:grid-cols-2 lg:grid-cols-3'
          }`}>
            {section.cards.map((card) => (
              <Link
                key={card.to}
                to={card.to}
                className="group bg-white border border-neutral-200 rounded-2xl p-5
                           hover:shadow-md hover:border-neutral-300 transition-all duration-200"
              >
                <div className="flex items-start gap-3">
                  <div className={`w-2 h-2 rounded-full mt-2 ${card.color} shrink-0`} />
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <h3 className="text-neutral-900 font-semibold text-sm group-hover:text-main-600 transition-colors">
                        {card.title}
                      </h3>
                      {card.tag && (
                        <span className="bg-main-100 text-main-600 text-[10px] font-bold px-1.5 py-0.5 rounded-full">
                          {card.tag}
                        </span>
                      )}
                    </div>
                    <p className="text-neutral-500 text-xs mt-1 leading-relaxed">{card.desc}</p>
                  </div>
                </div>
              </Link>
            ))}
          </div>
        </div>
      ))}

      {/* Latest blocks */}
      <div className="bg-white border border-neutral-200 rounded-2xl shadow-sm overflow-hidden">
        <div className="px-5 py-4 border-b border-neutral-100">
          <h2 className="text-lg font-semibold text-neutral-900">Ultimos bloques</h2>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-400 text-xs uppercase border-b border-neutral-100 bg-surface-alt">
                <th className="text-left py-3 px-5 font-semibold">#</th>
                <th className="text-left py-3 px-5 font-semibold">Hash</th>
                <th className="text-right py-3 px-5 font-semibold">Txns</th>
                <th className="text-right py-3 px-5 font-semibold">Tiempo</th>
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
                      className="text-main-500 hover:text-main-600 font-mono text-xs hover:underline transition-colors"
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
              No hay bloques aun. Mina uno desde la seccion Mineria.
            </p>
          )}
        </div>
      </div>
    </>
  )
}
