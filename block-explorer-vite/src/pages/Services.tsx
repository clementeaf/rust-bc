import { Link } from 'react-router-dom'

const icons = {
  credential: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <rect x="3" y="4" width="14" height="12" rx="2" /><path d="M7 8h6M7 11h4" />
    </svg>
  ),
  identity: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <circle cx="10" cy="7" r="3" /><path d="M4 17c0-3.3 2.7-6 6-6s6 2.7 6 6" />
    </svg>
  ),
  badge: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <path d="M10 2l2.3 4.7 5.2.8-3.8 3.7.9 5.2L10 14l-4.6 2.4.9-5.2L2.5 7.5l5.2-.8z" />
    </svg>
  ),
  governance: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <path d="M10 2l7 4v2H3V6l7-4zM5 9v6M10 9v6M15 9v6M3 16h14" />
    </svg>
  ),
  dashboard: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <rect x="2" y="2" width="7" height="7" rx="1" /><rect x="11" y="2" width="7" height="4" rx="1" /><rect x="2" y="11" width="7" height="4" rx="1" /><rect x="11" y="8" width="7" height="7" rx="1" />
    </svg>
  ),
  wallet: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <rect x="2" y="5" width="16" height="11" rx="2" /><path d="M2 8h16" /><circle cx="14" cy="12" r="1" />
    </svg>
  ),
  staking: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <path d="M10 2v16M6 6l4-4 4 4M6 10h8M6 14h8" />
    </svg>
  ),
  lock: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <rect x="4" y="9" width="12" height="8" rx="2" /><path d="M7 9V6a3 3 0 016 0v3" />
    </svg>
  ),
  cube: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <path d="M10 1.5L18 6v8l-8 4.5L2 14V6l8-4.5z" /><path d="M10 10v8.5M10 10l8-4.5M10 10L2 5.5" />
    </svg>
  ),
  code: (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth={1.5} className="w-5 h-5">
      <path d="M7 5L3 10l4 5M13 5l4 5-4 5M11 3l-2 14" />
    </svg>
  ),
}

interface ServiceCard {
  title: string
  desc: string
  to: string
  icon: React.ReactNode
  iconBg: string
  color: string
  badge?: string
}

const services: ServiceCard[] = [
  {
    title: 'Verificacion RRHH',
    desc: 'Demo guiado: emitir y verificar credenciales laborales en 5 pasos.',
    to: '/services/demo',
    icon: icons.credential,
    iconBg: 'bg-main-500 text-white',
    color: 'border-main-200 hover:border-main-400',
    badge: 'Demo interactivo',
  },
  {
    title: 'Identidad Digital (DID)',
    desc: 'Crear y consultar identidades descentralizadas did:cerulean.',
    to: '/services/identity',
    icon: icons.identity,
    iconBg: 'bg-violet-500 text-white',
    color: 'border-violet-200 hover:border-violet-400',
  },
  {
    title: 'Credenciales Verificables',
    desc: 'Emitir, buscar y verificar titulos y certificados digitales.',
    to: '/services/credentials',
    icon: icons.badge,
    iconBg: 'bg-emerald-500 text-white',
    color: 'border-emerald-200 hover:border-emerald-400',
  },
  {
    title: 'Gobernanza On-Chain',
    desc: 'Propuestas, votacion ponderada por stake y parametros del protocolo.',
    to: '/services/governance',
    icon: icons.governance,
    iconBg: 'bg-blue-500 text-white',
    color: 'border-blue-200 hover:border-blue-400',
    badge: 'Nuevo',
  },
  {
    title: 'Dashboard de Red',
    desc: 'Estado de la cadena, bloques recientes, peers conectados.',
    to: '/services/dashboard',
    icon: icons.dashboard,
    iconBg: 'bg-neutral-700 text-white',
    color: 'border-neutral-200 hover:border-neutral-400',
  },
  {
    title: 'Wallets y Transacciones',
    desc: 'Crear cuentas, enviar tokens, consultar el mempool en vivo.',
    to: '/services/wallets',
    icon: icons.wallet,
    iconBg: 'bg-amber-500 text-white',
    color: 'border-amber-200 hover:border-amber-400',
  },
  {
    title: 'Staking y Validadores',
    desc: 'Bloquear tokens, ver validadores activos y recompensas.',
    to: '/services/staking',
    icon: icons.staking,
    iconBg: 'bg-orange-500 text-white',
    color: 'border-orange-200 hover:border-orange-400',
  },
  {
    title: 'Canales Privados',
    desc: 'Redes aisladas entre organizaciones (estilo Fabric).',
    to: '/services/channels',
    icon: icons.lock,
    iconBg: 'bg-sky-500 text-white',
    color: 'border-sky-200 hover:border-sky-400',
  },
  {
    title: 'Mineria',
    desc: 'Crear nuevos bloques y ver resultados inmediatos.',
    to: '/services/mining',
    icon: icons.cube,
    iconBg: 'bg-stone-500 text-white',
    color: 'border-stone-200 hover:border-stone-400',
  },
  {
    title: 'Smart Contracts',
    desc: 'Contratos Wasm y Solidity desplegados en la red.',
    to: '/services/contracts',
    icon: icons.code,
    iconBg: 'bg-pink-500 text-white',
    color: 'border-pink-200 hover:border-pink-400',
  },
]

export default function Services() {
  return (
    <div>
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-2.5">
        {services.map((s) => (
          <Link
            key={s.to}
            to={s.to}
            className={`relative bg-white border rounded-xl p-3 transition-all hover:shadow-md hover:-translate-y-0.5 cursor-pointer ${s.color}`}
          >
            {s.badge && (
              <span className="absolute top-2 right-2 text-[8px] font-bold uppercase tracking-wider bg-main-500 text-white px-1.5 py-0.5 rounded-full">
                {s.badge}
              </span>
            )}
            <div className={`w-7 h-7 rounded-lg flex items-center justify-center ${s.iconBg}`}>
              {s.icon}
            </div>
            <h3 className="text-xs font-bold text-neutral-900 mt-2 leading-tight">{s.title}</h3>
            <p className="text-[10px] text-neutral-500 mt-0.5 leading-snug line-clamp-2">{s.desc}</p>
          </Link>
        ))}
      </div>
    </div>
  )
}
