import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getWallet, getWalletTransactions, type Wallet, type Transaction } from '../lib/api'
import { shortHash } from '../lib/format'

export default function WalletDetail() {
  const { address } = useParams<{ address: string }>()
  const [wallet, setWallet] = useState<Wallet | null>(null)
  const [txs, setTxs] = useState<Transaction[]>([])
  const [error, setError] = useState('')

  useEffect(() => {
    if (!address) return
    getWallet(address).then(setWallet).catch(() => setError('Wallet no encontrada'))
    getWalletTransactions(address).then(setTxs).catch(() => {})
  }, [address])

  if (error) return <p className="text-red-500">{error}</p>
  if (!wallet) return <p className="text-neutral-500">Cargando...</p>

  return (
    <>
      <div className="flex flex-col gap-2 mb-6">
        <div className="flex items-center gap-3">
          <Link to="/" className="text-neutral-500 hover:text-neutral-900 text-sm">&larr; Inicio</Link>
          <h1 className="text-xl font-bold text-neutral-900">Cartera</h1>
        </div>
        <p className="text-sm text-neutral-500 max-w-3xl">
          Saldo y movimientos de una direccion en la cadena. El "balance" es el calculado por el nodo
          a partir de los bloques.
        </p>
      </div>

      <div className="bg-white border border-neutral-200 rounded-2xl p-6 mb-6 text-left">
        <div className="grid gap-4 text-sm">
          <div>
            <span className="text-neutral-500">Direccion</span>
            <p className="text-neutral-900 font-mono text-xs break-all mt-1">{wallet.address}</p>
          </div>
          <div>
            <span className="text-neutral-500">Saldo</span>
            <p className="text-3xl font-bold text-neutral-900 mt-1">{wallet.balance} <span className="text-lg text-neutral-500">tokens</span></p>
          </div>
        </div>
      </div>

      <h2 className="text-lg font-semibold text-neutral-900 mb-4">Transacciones</h2>
      {txs.length === 0 ? (
        <p className="text-neutral-400 text-center py-8">Sin transacciones encontradas.</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
                <th className="text-left py-3 px-2">ID</th>
                <th className="text-left py-3 px-2">De</th>
                <th className="text-left py-3 px-2">Para</th>
                <th className="text-right py-3 px-2">Cantidad</th>
                <th className="text-right py-3 px-2">Comision</th>
              </tr>
            </thead>
            <tbody>
              {txs.map((tx) => (
                <tr key={tx.id} className="border-b border-neutral-100">
                  <td className="py-3 px-2 font-mono text-xs text-neutral-600">{shortHash(tx.id)}</td>
                  <td className="py-3 px-2">
                    <Link to={`/wallet/${tx.from}`} className="text-main-500 hover:text-main-600 font-mono text-xs">
                      {tx.from === address ? 'Tu' : shortHash(tx.from)}
                    </Link>
                  </td>
                  <td className="py-3 px-2">
                    <Link to={`/wallet/${tx.to}`} className="text-main-500 hover:text-main-600 font-mono text-xs">
                      {tx.to === address ? 'Tu' : shortHash(tx.to)}
                    </Link>
                  </td>
                  <td className="py-3 px-2 text-right text-neutral-900">{tx.amount}</td>
                  <td className="py-3 px-2 text-right text-neutral-500">{tx.fee}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  )
}
