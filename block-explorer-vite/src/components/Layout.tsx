import { type ReactElement } from 'react'
import { Outlet } from 'react-router-dom'

export default function Layout(): ReactElement {
  return (
    <div className="min-h-screen flex flex-col">
      {/* Main content */}
      <main className="flex-1 min-w-0 max-w-screen-2xl mx-auto w-full px-4 sm:px-6 lg:px-8 py-8">
        <Outlet />
      </main>

      {/* Footer */}
      <footer className="border-t border-neutral-200 py-4">
        <div className="max-w-screen-2xl mx-auto px-4 sm:px-6 flex items-center justify-between text-xs text-neutral-400">
          <span>Cerulean Ledger Explorer</span>
          <span>Blockchain PQC-ready</span>
        </div>
      </footer>
    </div>
  )
}
