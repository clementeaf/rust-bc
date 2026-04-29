import { useState, useEffect, useCallback } from 'react'
import {
  getAccount, getMempoolStats, faucetDrip, getBlocks,
  type AccountInfo, type MempoolStats, type Block,
} from '../lib/api'

export default function Crypto() {
  const [address, setAddress] = useState('')
  const [account, setAccount] = useState<AccountInfo | null>(null)
  const [mempool, setMempool] = useState<MempoolStats | null>(null)
  const [blocks, setBlocks] = useState<Block[]>([])
  const [faucetAddr, setFaucetAddr] = useState('')
  const [faucetMsg, setFaucetMsg] = useState('')
  const [error, setError] = useState('')

  const fetchMempool = useCallback(async () => {
    try { setMempool(await getMempoolStats()) } catch { /* ignore */ }
  }, [])

  const fetchBlocks = useCallback(async () => {
    try { setBlocks(await getBlocks()) } catch { /* ignore */ }
  }, [])

  useEffect(() => {
    fetchMempool()
    fetchBlocks()
    const iv = setInterval(() => { fetchMempool(); fetchBlocks() }, 5000)
    return () => clearInterval(iv)
  }, [fetchMempool, fetchBlocks])

  const lookupAccount = async () => {
    if (!address.trim()) return
    setError('')
    setAccount(null)
    try {
      setAccount(await getAccount(address.trim()))
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Error')
    }
  }

  const requestFaucet = async () => {
    if (!faucetAddr.trim()) return
    setFaucetMsg('')
    try {
      const r = await faucetDrip(faucetAddr.trim())
      setFaucetMsg(`+${r.amount} NOTA → balance: ${r.new_balance}`)
    } catch (e) {
      setFaucetMsg(e instanceof Error ? e.message : 'Error')
    }
  }

  return (
    <div className="space-y-8 max-w-5xl mx-auto">
      <h1 className="text-3xl font-bold">Cryptocurrency Dashboard</h1>

      {/* Supply & Mempool */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <StatCard title="Mempool" value={mempool ? `${mempool.pending} txs` : '—'} />
        <StatCard title="Base Fee" value={mempool ? `${mempool.base_fee} NOTA` : '—'} />
        <StatCard title="Bloques" value={`${blocks.length}`} />
      </div>

      {/* Account Lookup */}
      <section className="bg-white dark:bg-gray-800 rounded-lg p-6 shadow">
        <h2 className="text-xl font-semibold mb-4">Consultar Cuenta</h2>
        <div className="flex gap-2">
          <input
            className="flex-1 px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600"
            placeholder="Dirección (hex 40 chars)"
            value={address}
            onChange={e => setAddress(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && lookupAccount()}
          />
          <button
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
            onClick={lookupAccount}
          >
            Buscar
          </button>
        </div>
        {error && <p className="mt-2 text-red-500">{error}</p>}
        {account && (
          <div className="mt-4 grid grid-cols-2 gap-2 text-sm">
            <div className="font-medium">Dirección</div>
            <div className="font-mono break-all">{account.address}</div>
            <div className="font-medium">Balance</div>
            <div>{account.balance.toLocaleString()} NOTA</div>
            <div className="font-medium">Nonce</div>
            <div>{account.nonce}</div>
            <div className="font-medium">Tipo</div>
            <div>{account.is_contract ? 'Contrato' : 'EOA'}</div>
          </div>
        )}
      </section>

      {/* Faucet */}
      <section className="bg-white dark:bg-gray-800 rounded-lg p-6 shadow">
        <h2 className="text-xl font-semibold mb-4">Faucet (Testnet)</h2>
        <div className="flex gap-2">
          <input
            className="flex-1 px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600"
            placeholder="Dirección para recibir tokens"
            value={faucetAddr}
            onChange={e => setFaucetAddr(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && requestFaucet()}
          />
          <button
            className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
            onClick={requestFaucet}
          >
            Solicitar Tokens
          </button>
        </div>
        {faucetMsg && <p className="mt-2 text-sm">{faucetMsg}</p>}
      </section>

      {/* Recent Blocks */}
      <section className="bg-white dark:bg-gray-800 rounded-lg p-6 shadow">
        <h2 className="text-xl font-semibold mb-4">Bloques Recientes</h2>
        {blocks.length === 0 ? (
          <p className="text-gray-500">Sin bloques</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left border-b dark:border-gray-600">
                  <th className="py-2 pr-4">#</th>
                  <th className="py-2 pr-4">Hash</th>
                  <th className="py-2 pr-4">TXs</th>
                  <th className="py-2">Timestamp</th>
                </tr>
              </thead>
              <tbody>
                {blocks.slice(0, 20).map((b) => (
                  <tr key={b.hash} className="border-b dark:border-gray-700">
                    <td className="py-2 pr-4 font-mono">{b.index}</td>
                    <td className="py-2 pr-4 font-mono text-xs truncate max-w-[200px]">
                      {b.hash}
                    </td>
                    <td className="py-2 pr-4">{b.transactions?.length ?? 0}</td>
                    <td className="py-2 text-xs text-gray-500">
                      {new Date(b.timestamp * 1000).toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  )
}

function StatCard({ title, value }: { title: string; value: string }) {
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg p-4 shadow text-center">
      <div className="text-sm text-gray-500 dark:text-gray-400">{title}</div>
      <div className="text-2xl font-bold mt-1">{value}</div>
    </div>
  )
}
