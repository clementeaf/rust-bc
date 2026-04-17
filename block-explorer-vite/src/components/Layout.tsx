import type { ReactElement } from 'react'
import { NavLink, Outlet } from 'react-router-dom'

const links: { to: string; label: string; hint: string }[] = [
  { to: '/', label: 'Dashboard', hint: 'Chain overview, latest blocks, and search' },
  { to: '/validators', label: 'Validators', hint: 'Staking validators and consensus status' },
  { to: '/contracts', label: 'Contracts', hint: 'Deployed smart contracts' },
  { to: '/airdrop', label: 'Airdrop', hint: 'Reward distribution and eligible nodes' },
  { to: '/identity', label: 'Identity', hint: 'DID management and lookup' },
  { to: '/credentials', label: 'Credentials', hint: 'Verifiable credentials lifecycle' },
]

export default function Layout(): ReactElement {
  return (
    <div className="min-h-screen flex flex-col">
      {/* Sticky nav */}
      <nav className="sticky top-0 z-50 bg-white/80 backdrop-blur-xl border-b border-neutral-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-3 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
          <div className="flex items-center gap-3">
            {/* Logo */}
            <NavLink to="/" className="flex items-center gap-2">
              <div className="w-8 h-8 rounded-xl bg-main-500 flex items-center justify-center">
                <span className="text-white font-bold text-sm">rc</span>
              </div>
              <span className="text-lg font-bold text-neutral-900 tracking-tight">
                rust-bc
              </span>
            </NavLink>
            <span className="hidden sm:inline text-xs text-neutral-400 border-l border-neutral-200 pl-3">
              Block Explorer
            </span>
          </div>

          {/* Nav links */}
          <div className="flex flex-wrap gap-x-1 gap-y-1">
            {links.map((l) => (
              <NavLink
                key={l.to}
                to={l.to}
                title={l.hint}
                end={l.to === '/'}
                className={({ isActive }) =>
                  `text-sm font-medium px-3 py-1.5 rounded-full transition-all duration-200 ${
                    isActive
                      ? 'bg-main-500 text-white shadow-sm'
                      : 'text-neutral-500 hover:text-neutral-900 hover:bg-neutral-100'
                  }`
                }
              >
                {l.label}
              </NavLink>
            ))}
          </div>
        </div>
      </nav>

      {/* Main content */}
      <main className="flex-1 max-w-7xl mx-auto w-full px-4 sm:px-6 lg:px-8 py-8">
        <Outlet />
      </main>

      {/* Footer */}
      <footer className="border-t border-neutral-200 py-6">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 flex items-center justify-between text-xs text-neutral-400">
          <span>rust-bc Explorer</span>
          <span>PQC-ready blockchain · ML-DSA-65 + Ed25519</span>
        </div>
      </footer>
    </div>
  )
}
