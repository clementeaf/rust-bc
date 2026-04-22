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
      setError('No encontrado. Intenta con un hash de bloque, direccion de wallet o contrato.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="mb-8">
      <div className="flex gap-2">
        <input
          id="chain-search"
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && search()}
          placeholder="Buscar por hash de bloque, wallet o contrato..."
          className="flex-1 bg-white border border-neutral-200 rounded-full px-5 py-2.5 text-sm
                     text-neutral-900 placeholder-neutral-400
                     focus:outline-none focus:ring-2 focus:ring-main-500/20 focus:border-main-500
                     transition-all duration-200 shadow-sm"
        />
        <button
          onClick={search}
          disabled={loading}
          className="bg-main-500 hover:bg-main-600 disabled:opacity-50 text-white px-6 py-2.5
                     rounded-full text-sm font-semibold transition-all duration-200 shadow-sm
                     hover:shadow-md"
        >
          {loading ? '...' : 'Buscar'}
        </button>
      </div>
      {error && <p className="text-red-500 text-sm mt-2 pl-5">{error}</p>}
    </div>
  )
}
