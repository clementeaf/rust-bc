import { useEffect, useState } from 'react'
import PageIntro from '../components/PageIntro'
import { getMempool, sendTransaction, type Transaction } from '../lib/api'

function shortAddr(a: string) {
  return a.length > 16 ? a.slice(0, 8) + '...' + a.slice(-8) : a
}

function timeAgo(ts: number) {
  const diff = Math.floor(Date.now() / 1000 - ts)
  if (diff < 60) return `${diff}s ago`
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
  return new Date(ts * 1000).toLocaleDateString()
}

export default function Transactions() {
  const [txs, setTxs] = useState<Transaction[]>([])
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [amount, setAmount] = useState('')
  const [fee, setFee] = useState('1')
  const [sending, setSending] = useState(false)
  const [error, setError] = useState('')

  const load = () => getMempool().then(setTxs).catch(() => {})

  useEffect(() => {
    load()
    const id = setInterval(load, 5000)
    return () => clearInterval(id)
  }, [])

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    setSending(true)
    try {
      await sendTransaction(from, to, Number(amount), Number(fee))
      setFrom('')
      setTo('')
      setAmount('')
      setFee('1')
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send')
    } finally {
      setSending(false)
    }
  }

  return (
    <>
      <PageIntro title="Transactions">
        Enviar transacciones y ver el mempool de transacciones pendientes.
      </PageIntro>

      <div className="bg-white border border-neutral-200 rounded-2xl p-5 mb-8">
        <h2 className="text-lg font-semibold text-neutral-900 mb-4">Send Transaction</h2>
        <form onSubmit={handleSend} className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <input
            type="text"
            placeholder="From address"
            value={from}
            onChange={(e) => setFrom(e.target.value)}
            required
            className="border border-neutral-200 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <input
            type="text"
            placeholder="To address"
            value={to}
            onChange={(e) => setTo(e.target.value)}
            required
            className="border border-neutral-200 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <input
            type="number"
            placeholder="Amount"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            required
            min="0"
            step="any"
            className="border border-neutral-200 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <input
            type="number"
            placeholder="Fee"
            value={fee}
            onChange={(e) => setFee(e.target.value)}
            required
            min="0"
            step="any"
            className="border border-neutral-200 rounded-xl px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <div className="md:col-span-2 flex items-center gap-4">
            <button
              type="submit"
              disabled={sending}
              className="bg-main-500 text-white px-4 py-2 rounded-xl text-sm font-medium
                         hover:bg-main-600 disabled:opacity-50 transition-colors"
            >
              {sending ? 'Sending...' : 'Send'}
            </button>
            {error && <p className="text-red-500 text-sm">{error}</p>}
          </div>
        </form>
      </div>

      <h2 className="text-lg font-semibold text-neutral-900 mb-1">Mempool</h2>
      <p className="text-xs text-neutral-400 mb-4">Transacciones pendientes esperando ser incluidas en un bloque.</p>

      {txs.length === 0 ? (
        <div className="bg-white border border-neutral-200 rounded-2xl p-8 text-center">
          <p className="text-neutral-500">No pending transactions.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
                <th className="text-left py-3 px-2">ID</th>
                <th className="text-left py-3 px-2">From</th>
                <th className="text-left py-3 px-2">To</th>
                <th className="text-right py-3 px-2">Amount</th>
                <th className="text-right py-3 px-2">Fee</th>
                <th className="text-right py-3 px-2">Time</th>
              </tr>
            </thead>
            <tbody>
              {txs.map((tx) => (
                <tr key={tx.id} className="border-b border-neutral-100">
                  <td className="py-3 px-2 font-mono text-xs text-neutral-600">{shortAddr(tx.id)}</td>
                  <td className="py-3 px-2 font-mono text-xs text-neutral-600">{shortAddr(tx.from)}</td>
                  <td className="py-3 px-2 font-mono text-xs text-neutral-600">{shortAddr(tx.to)}</td>
                  <td className="py-3 px-2 text-right text-neutral-900 font-medium">{tx.amount}</td>
                  <td className="py-3 px-2 text-right text-neutral-500">{tx.fee}</td>
                  <td className="py-3 px-2 text-right text-neutral-400">{timeAgo(tx.timestamp)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  )
}
