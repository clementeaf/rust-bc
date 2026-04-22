import { useState } from 'react'
import { Link } from 'react-router-dom'
import PageIntro from '../components/PageIntro'
import { createWallet, mineBlock } from '../lib/api'

interface MineResult {
  message?: string;
  block_index?: number;
  block_hash?: string;
  [key: string]: unknown;
}

export default function Mining() {
  const [minerAddress, setMinerAddress] = useState('')
  const [mining, setMining] = useState(false)
  const [result, setResult] = useState<MineResult | null>(null)
  const [error, setError] = useState('')

  const handleCreateAndMine = async () => {
    setError('')
    setMining(true)
    setResult(null)
    try {
      const wallet = await createWallet()
      setMinerAddress(wallet.address)
      const res = await mineBlock(wallet.address)
      setResult(res)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Mining failed')
    } finally {
      setMining(false)
    }
  }

  const handleMine = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!minerAddress.trim()) return
    setError('')
    setMining(true)
    setResult(null)
    try {
      const res = await mineBlock(minerAddress)
      setResult(res)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Mining failed')
    } finally {
      setMining(false)
    }
  }

  return (
    <>
      <PageIntro title="Mining">
        Minar un nuevo bloque. Necesitas una wallet registrada como dirección del minero.
      </PageIntro>

      <div className="bg-white border border-neutral-200 rounded-2xl p-5 mb-8">
        <form onSubmit={handleMine} className="flex flex-col sm:flex-row gap-4">
          <input
            type="text"
            placeholder="Direccion del minero"
            value={minerAddress}
            onChange={(e) => setMinerAddress(e.target.value)}
            className="flex-1 border border-neutral-200 rounded-xl px-3 py-2 text-sm font-mono
                       focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <button
            type="submit"
            disabled={mining || !minerAddress.trim()}
            className="bg-main-500 text-white px-4 py-2 rounded-xl text-sm font-medium
                       hover:bg-main-600 disabled:opacity-50 transition-colors"
          >
            {mining ? 'Mining...' : 'Mine Block'}
          </button>
          <button
            type="button"
            onClick={handleCreateAndMine}
            disabled={mining}
            className="bg-neutral-800 text-white px-4 py-2 rounded-xl text-sm font-medium
                       hover:bg-neutral-700 disabled:opacity-50 transition-colors"
          >
            New Wallet + Mine
          </button>
        </form>
        {error && <p className="text-red-500 text-sm mt-3">{error}</p>}
      </div>

      {result && (
        <div className="bg-green-50 border border-green-200 rounded-2xl p-5">
          <h2 className="text-lg font-semibold text-green-800 mb-2">Block Mined</h2>
          <dl className="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-2 text-sm">
            {result.block_index != null && (
              <>
                <dt className="text-neutral-500">Block Index</dt>
                <dd className="text-neutral-900 font-medium">{result.block_index}</dd>
              </>
            )}
            {result.block_hash && (
              <>
                <dt className="text-neutral-500">Block Hash</dt>
                <dd>
                  <Link
                    to={`/block/${result.block_hash}`}
                    className="text-main-500 hover:text-main-600 font-mono text-xs"
                  >
                    {result.block_hash}
                  </Link>
                </dd>
              </>
            )}
            {result.message && (
              <>
                <dt className="text-neutral-500">Message</dt>
                <dd className="text-neutral-900">{result.message}</dd>
              </>
            )}
          </dl>
        </div>
      )}
    </>
  )
}
