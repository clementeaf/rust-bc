import { useState } from 'react'
import { Link } from 'react-router-dom'
import PageIntro from '../components/PageIntro'
import { createWallet, getWallet, type Wallet } from '../lib/api'
import { shortHash } from '../lib/format'

export default function Wallets() {
  const [wallets, setWallets] = useState<Wallet[]>([])
  const [creating, setCreating] = useState(false)
  const [lookupAddr, setLookupAddr] = useState('')
  const [lookupError, setLookupError] = useState('')

  const handleCreate = async () => {
    setCreating(true)
    try {
      const w = await createWallet()
      setWallets((prev) => [w, ...prev])
    } catch {
      // ignore
    } finally {
      setCreating(false)
    }
  }

  const handleLookup = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!lookupAddr.trim()) return
    setLookupError('')
    try {
      const w = await getWallet(lookupAddr.trim())
      setWallets((prev) => {
        if (prev.some((x) => x.address === w.address)) return prev
        return [w, ...prev]
      })
      setLookupAddr('')
    } catch {
      setLookupError('Wallet no encontrada')
    }
  }

  const refreshBalances = async () => {
    const updated = await Promise.all(
      wallets.map((w) => getWallet(w.address).catch(() => w))
    )
    setWallets(updated)
  }

  return (
    <>
      <PageIntro title="Wallets">
        Crea wallets nuevas o busca por direccion. Las wallets creadas en esta sesion aparecen abajo.
      </PageIntro>

      <div className="flex flex-col sm:flex-row gap-4 mb-6">
        <button
          onClick={handleCreate}
          disabled={creating}
          className="bg-main-500 text-white px-4 py-2 rounded-xl text-sm font-medium
                     hover:bg-main-600 disabled:opacity-50 transition-colors"
        >
          {creating ? 'Creando...' : 'Crear wallet'}
        </button>

        <form onSubmit={handleLookup} className="flex gap-2 flex-1">
          <input
            type="text"
            placeholder="Buscar por direccion"
            value={lookupAddr}
            onChange={(e) => setLookupAddr(e.target.value)}
            className="flex-1 border border-neutral-200 rounded-xl px-3 py-2 text-sm font-mono
                       focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <button
            type="submit"
            className="bg-neutral-800 text-white px-4 py-2 rounded-xl text-sm font-medium
                       hover:bg-neutral-700 transition-colors"
          >
            Buscar
          </button>
        </form>
      </div>
      {lookupError && <p className="text-red-500 text-sm mb-4">{lookupError}</p>}

      {wallets.length === 0 ? (
        <div className="bg-white border border-neutral-200 rounded-2xl p-8 text-center">
          <p className="text-neutral-500 mb-2">No hay wallets aun.</p>
          <p className="text-neutral-400 text-sm">Crea una o busca una direccion existente.</p>
        </div>
      ) : (
        <>
          <div className="flex items-center justify-between mb-2">
            <h2 className="text-lg font-semibold text-neutral-900">
              Wallets ({wallets.length})
            </h2>
            <button
              onClick={refreshBalances}
              className="text-main-500 hover:text-main-600 text-xs font-medium"
            >
              Actualizar saldos
            </button>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
                  <th className="text-left py-3 px-2">Direccion</th>
                  <th className="text-right py-3 px-2">Saldo</th>
                </tr>
              </thead>
              <tbody>
                {wallets.map((w) => (
                  <tr key={w.address} className="border-b border-neutral-100 hover:bg-white">
                    <td className="py-3 px-2">
                      <Link
                        to={`/wallet/${w.address}`}
                        className="text-main-500 hover:text-main-600 font-mono text-xs"
                      >
                        {shortHash(w.address)}
                      </Link>
                    </td>
                    <td className="py-3 px-2 text-right text-neutral-900 font-medium">
                      {w.balance}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </>
  )
}
