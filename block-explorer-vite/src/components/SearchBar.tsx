import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { searchByHash } from '../lib/api'

export default function SearchBar() {
  const [query, setQuery] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const navigate = useNavigate()

  const search = async () => {
    const q = query.trim()
    if (!q) return
    setError('')
    setLoading(true)
    try {
      const result = await searchByHash(q)
      const paths = { block: '/block/', contract: '/contract/', wallet: '/wallet/' }
      navigate(paths[result.type] + result.id)
    } catch {
      setError('Not found. Try a block hash, wallet address, or contract address.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="mb-8">
      <label htmlFor="chain-search" className="block text-sm font-medium text-gray-300 mb-2">
        Buscar en la cadena
      </label>
      <p className="text-xs text-gray-500 mb-3">
        Acepta un hash de bloque, una dirección de cartera o la dirección de un contrato; te lleva a la ficha correspondiente.
      </p>
      <div className="flex gap-2">
        <input
          id="chain-search"
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && search()}
          placeholder="Hash de bloque, wallet o contrato…"
          className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-4 py-2.5 text-sm
                     placeholder-gray-500 focus:outline-none focus:border-cyan-500 transition-colors"
        />
        <button
          onClick={search}
          disabled={loading}
          className="bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 text-white px-5 py-2.5
                     rounded-lg text-sm font-medium transition-colors"
        >
          {loading ? '...' : 'Search'}
        </button>
      </div>
      {error && <p className="text-red-400 text-sm mt-2">{error}</p>}
    </div>
  )
}
