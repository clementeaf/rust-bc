import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import PageIntro from '../components/PageIntro'
import { createWallet, getWallets, type Wallet } from '../lib/api'

function shortAddr(a: string) {
  return a.length > 16 ? a.slice(0, 8) + '...' + a.slice(-8) : a
}

export default function Wallets() {
  const [wallets, setWallets] = useState<Wallet[]>([])
  const [creating, setCreating] = useState(false)

  const load = () => getWallets().then(setWallets).catch(() => {})

  useEffect(() => {
    load()
    const id = setInterval(load, 15000)
    return () => clearInterval(id)
  }, [])

  const handleCreate = async () => {
    setCreating(true)
    try {
      await createWallet()
      await load()
    } catch {
      // ignore
    } finally {
      setCreating(false)
    }
  }

  return (
    <>
      <PageIntro title="Wallets">
        Cuentas registradas en la red. Puedes crear una nueva wallet o ver el detalle de cada una.
      </PageIntro>

      <div className="mb-6">
        <button
          onClick={handleCreate}
          disabled={creating}
          className="bg-main-500 text-white px-4 py-2 rounded-xl text-sm font-medium
                     hover:bg-main-600 disabled:opacity-50 transition-colors"
        >
          {creating ? 'Creating...' : 'Create Wallet'}
        </button>
      </div>

      {wallets.length === 0 ? (
        <div className="bg-white border border-neutral-200 rounded-2xl p-8 text-center">
          <p className="text-neutral-500 mb-2">No wallets yet.</p>
          <p className="text-neutral-400 text-sm">Create one to get started.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
                <th className="text-left py-3 px-2">Address</th>
                <th className="text-right py-3 px-2">Balance</th>
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
                      {shortAddr(w.address)}
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
      )}
    </>
  )
}
