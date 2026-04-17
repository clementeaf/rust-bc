import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getBlockByHash, type Block } from '../lib/api'

function shortAddr(a: string) {
  return a.length > 16 ? a.slice(0, 8) + '...' + a.slice(-8) : a
}

export default function BlockDetail() {
  const { hash } = useParams<{ hash: string }>()
  const [block, setBlock] = useState<Block | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!hash) return
    getBlockByHash(hash).then(setBlock).catch(() => setError('Block not found'))
  }, [hash])

  if (error) return <p className="text-red-500">{error}</p>
  if (!block) return <p className="text-neutral-500">Loading...</p>

  return (
    <>
      <div className="flex flex-col gap-2 mb-6">
        <div className="flex items-center gap-3">
          <Link to="/" className="text-neutral-500 hover:text-neutral-900 text-sm">&larr; Inicio</Link>
          <h1 className="text-xl font-bold text-neutral-900">Bloque #{block.index}</h1>
        </div>
        <p className="text-sm text-neutral-500 max-w-3xl">
          Cabecera y prueba de trabajo de un bloque ya incluido en la cadena. Las transacciones listadas
          abajo forman parte del merkle root.
        </p>
      </div>

      <div className="bg-white border border-neutral-200 rounded-2xl p-6 mb-6 text-left">
        <div className="grid gap-4 text-sm">
          {[
            ['Hash', block.hash],
            ['Previous Hash', block.previous_hash],
            ['Merkle Root', block.merkle_root],
            ['Nonce', block.nonce],
            ['Difficulty', block.difficulty],
            ['Timestamp', new Date(block.timestamp * 1000).toLocaleString()],
            ['Transactions', block.transactions.length],
          ].map(([label, value]) => (
            <div key={String(label)} className="flex flex-col sm:flex-row sm:gap-4">
              <span className="text-neutral-500 sm:w-36 shrink-0">{String(label)}</span>
              <span className="text-neutral-900 font-mono text-xs break-all">{String(value)}</span>
            </div>
          ))}
        </div>
      </div>

      {block.index > 0 && (
        <Link
          to={`/block/${block.previous_hash}`}
          className="text-main-500 hover:text-main-600 text-sm mb-6 inline-block"
        >
          &larr; Previous Block
        </Link>
      )}

      <h2 className="text-lg font-semibold text-neutral-900 mb-4 mt-4">Transactions</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
              <th className="text-left py-3 px-2">ID</th>
              <th className="text-left py-3 px-2">From</th>
              <th className="text-left py-3 px-2">To</th>
              <th className="text-right py-3 px-2">Amount</th>
              <th className="text-right py-3 px-2">Fee</th>
            </tr>
          </thead>
          <tbody>
            {block.transactions.map((tx) => (
              <tr key={tx.id} className="border-b border-neutral-100">
                <td className="py-3 px-2 font-mono text-xs text-neutral-600">{shortAddr(tx.id)}</td>
                <td className="py-3 px-2">
                  {tx.from === '0' ? (
                    <span className="text-amber-600 text-xs">Coinbase</span>
                  ) : (
                    <Link to={`/wallet/${tx.from}`} className="text-main-500 hover:text-main-600 font-mono text-xs">
                      {shortAddr(tx.from)}
                    </Link>
                  )}
                </td>
                <td className="py-3 px-2">
                  <Link to={`/wallet/${tx.to}`} className="text-main-500 hover:text-main-600 font-mono text-xs">
                    {shortAddr(tx.to)}
                  </Link>
                </td>
                <td className="py-3 px-2 text-right text-neutral-900">{tx.amount}</td>
                <td className="py-3 px-2 text-right text-neutral-500">{tx.fee}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  )
}
