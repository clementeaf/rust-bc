import type { ReactElement } from 'react'
import { NavLink, Outlet } from 'react-router-dom'

const links: { to: string; label: string; hint: string }[] = [
  { to: '/', label: 'Inicio', hint: 'Resumen de la cadena, últimos bloques y búsqueda' },
  { to: '/validators', label: 'Validadores', hint: 'Staking: quién participa en consenso (PoS)' },
  { to: '/contracts', label: 'Contratos', hint: 'Smart contracts desplegados en el nodo' },
  { to: '/airdrop', label: 'Airdrop', hint: 'Reparto de recompensas y nodos elegibles' },
  { to: '/identity', label: 'Personas', hint: 'Códigos únicos para cada persona u organización en el nodo' },
  {
    to: '/credentials',
    label: 'Certificados',
    hint: 'Opcional: reconocimiento entre dos fichas (quién declara sobre quién)',
  },
]

export default function Layout(): ReactElement {
  return (
    <div className="min-h-screen flex flex-col">
      <nav className="bg-gray-900 border-b border-gray-800">
        <div className="max-w-6xl mx-auto px-4 py-3 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
          <div>
            <NavLink to="/" className="text-lg font-bold text-cyan-400 tracking-tight block">
              rust-bc
            </NavLink>
            <p className="text-xs text-gray-500 mt-0.5 hidden sm:block">
              Explorador de la API del nodo (bloques, cuentas, contratos, identidad)
            </p>
          </div>
          <div className="flex flex-wrap gap-x-5 gap-y-2">
            {links.map((l) => (
              <NavLink
                key={l.to}
                to={l.to}
                title={l.hint}
                end={l.to === '/'}
                className={({ isActive }) =>
                  `text-sm font-medium transition-colors border-b-2 pb-0.5 -mb-px ${
                    isActive
                      ? 'text-cyan-400 border-cyan-400'
                      : 'text-gray-400 hover:text-gray-200 border-transparent'
                  }`
                }
              >
                {l.label}
              </NavLink>
            ))}
          </div>
        </div>
      </nav>
      <main className="flex-1 max-w-6xl mx-auto w-full px-4 py-8">
        <Outlet />
      </main>
    </div>
  )
}
