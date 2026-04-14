import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getWallet, getWalletTransactions, type Wallet, type Transaction } from '../lib/api'

function shortAddr(a: string) {
  return a.length > 16 ? a.slice(0, 8) + '...' + a.slice(-8) : a
}

export default function WalletDetail() {
  const { address } = useParams<{ address: string }>()
  const [wallet, setWallet] = useState<Wallet | null>(null)
  const [txs, setTxs] = useState<Transaction[]>([])
  const [error, setError] = useState('')

  useEffect(() => {
    if (!address) return
    getWallet(address).then(setWallet).catch(() => setError('Wallet not found'))
    getWalletTransactions(address).then(setTxs).catch(() => {})
  }, [address])

  if (error) return <p className="text-red-400">{error}</p>
  if (!wallet) return <p className="text-gray-400">Loading...</p>

  return (
    <>
      <div className="flex flex-col gap-2 mb-6">
        <div className="flex items-center gap-3">
          <Link to="/" className="text-gray-400 hover:text-white text-sm">&larr; Inicio</Link>
          <h1 className="text-xl font-bold text-white">Cartera</h1>
        </div>
        <p className="text-sm text-gray-400 max-w-3xl">
          Saldo y movimientos de una dirección en la cadena. El “balance” es el calculado por el nodo
          a partir de los bloques.
        </p>
      </div>

      <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 mb-6 text-left">
        <div className="grid gap-4 text-sm">
          <div>
            <span className="text-gray-400">Address</span>
            <p className="text-white font-mono text-xs break-all mt-1">{wallet.address}</p>
          </div>
          <div>
            <span className="text-gray-400">Balance</span>
            <p className="text-3xl font-bold text-white mt-1">{wallet.balance} <span className="text-lg text-gray-400">coins</span></p>
          </div>
        </div>
      </div>

      <h2 className="text-lg font-semibold text-white mb-4">Transactions</h2>
      {txs.length === 0 ? (
        <p className="text-gray-500 text-center py-8">No transactions found.</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-gray-400 text-xs uppercase border-b border-gray-800">
                <th className="text-left py-3 px-2">ID</th>
                <th className="text-left py-3 px-2">From</th>
                <th className="text-left py-3 px-2">To</th>
                <th className="text-right py-3 px-2">Amount</th>
                <th className="text-right py-3 px-2">Fee</th>
              </tr>
            </thead>
            <tbody>
              {txs.map((tx) => (
                <tr key={tx.id} className="border-b border-gray-800/50">
                  <td className="py-3 px-2 font-mono text-xs text-gray-300">{shortAddr(tx.id)}</td>
                  <td className="py-3 px-2">
                    <Link to={`/wallet/${tx.from}`} className="text-cyan-400 hover:text-cyan-300 font-mono text-xs">
                      {tx.from === address ? 'You' : shortAddr(tx.from)}
                    </Link>
                  </td>
                  <td className="py-3 px-2">
                    <Link to={`/wallet/${tx.to}`} className="text-cyan-400 hover:text-cyan-300 font-mono text-xs">
                      {tx.to === address ? 'You' : shortAddr(tx.to)}
                    </Link>
                  </td>
                  <td className="py-3 px-2 text-right text-white">{tx.amount}</td>
                  <td className="py-3 px-2 text-right text-gray-400">{tx.fee}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  )
}
