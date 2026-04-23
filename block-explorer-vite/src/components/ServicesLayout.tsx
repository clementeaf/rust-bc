import { NavLink, Outlet, Link, useLocation } from 'react-router-dom'

const serviceNav = [
  { to: '/services/demo', label: 'Verificacion RRHH' },
  { to: '/services/identity', label: 'Identidad DID' },
  { to: '/services/credentials', label: 'Credenciales' },
  { to: '/services/governance', label: 'Gobernanza' },
  { to: '/services/dashboard', label: 'Dashboard' },
  { to: '/services/wallets', label: 'Wallets' },
  { to: '/services/transactions', label: 'Transacciones' },
  { to: '/services/staking', label: 'Staking' },
  { to: '/services/channels', label: 'Canales' },
  { to: '/services/mining', label: 'Mineria' },
  { to: '/services/contracts', label: 'Contratos' },
]

export default function ServicesLayout() {
  const location = useLocation()
  const current = serviceNav.find((s) => location.pathname.startsWith(s.to))

  return (
    <div className="min-h-screen flex flex-col">
      {/* Header */}
      <header className="sticky top-0 z-50 bg-white/80 backdrop-blur-xl border-b border-neutral-200">
        <div className="max-w-screen-2xl mx-auto px-4 sm:px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Link to="/" className="flex items-center gap-2">
              <div className="w-8 h-8 rounded-xl bg-main-500 flex items-center justify-center">
                <span className="text-white font-bold text-sm">CL</span>
              </div>
              <span className="text-lg font-bold text-neutral-900 tracking-tight">Cerulean Ledger</span>
            </Link>
            <Link
              to="/services"
              className="text-xs text-neutral-400 border-l border-neutral-200 pl-3 hover:text-main-500 transition-colors cursor-pointer"
            >
              Servicios
            </Link>
            {current && (
              <span className="text-xs text-neutral-400">
                / {current.label}
              </span>
            )}
          </div>
          <div className="hidden sm:flex items-center gap-2 text-xs text-neutral-400">
            <span className="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
            PQC-ready · ML-DSA-65 + Ed25519
          </div>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* Compact sidebar — fixed height, never scrolls with content */}
        <aside className="hidden lg:block w-52 border-r border-neutral-200 bg-white py-4 px-2 overflow-y-auto shrink-0 h-[calc(100vh-57px)] sticky top-[57px]">
          <Link
            to="/services"
            className="flex items-center gap-2 px-3 py-2 mb-3 text-xs font-semibold text-main-600 hover:bg-main-50 rounded-lg cursor-pointer transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
            </svg>
            Todos los servicios
          </Link>
          <div className="space-y-0.5">
            {serviceNav.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  `block px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
                    isActive
                      ? 'bg-main-500 text-white'
                      : 'text-neutral-600 hover:bg-neutral-100'
                  }`
                }
              >
                {item.label}
              </NavLink>
            ))}
          </div>
        </aside>

        {/* Content */}
        <main className="flex-1 min-w-0 px-4 sm:px-6 lg:px-8 py-8 overflow-y-auto h-[calc(100vh-57px)]">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
