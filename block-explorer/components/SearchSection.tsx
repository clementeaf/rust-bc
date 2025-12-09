'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { searchByHash } from '@/lib/api';

/**
 * Componente de búsqueda para bloques, transacciones, wallets y contratos
 */
export default function SearchSection() {
  const [searchTerm, setSearchTerm] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const router = useRouter();

  async function handleSearch() {
    if (!searchTerm.trim()) {
      setError('Please enter a search term');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const result = await searchByHash(searchTerm.trim());
      
      if (result.type === 'block') {
        router.push(`/block/${searchTerm.trim()}`);
      } else if (result.type === 'contract') {
        router.push(`/contract/${searchTerm.trim()}`);
      } else if (result.type === 'wallet') {
        router.push(`/wallet/${searchTerm.trim()}`);
      } else {
        setError('Unknown result type');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Not found');
    } finally {
      setLoading(false);
    }
  }

  function handleKeyPress(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') {
      handleSearch();
    }
  }

  return (
    <div className="mt-8 bg-white rounded-lg shadow p-6">
      <h2 className="text-xl font-semibold text-gray-900 mb-4">Search</h2>
      <div className="flex gap-4">
        <input
          type="text"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          onKeyPress={handleKeyPress}
          placeholder="Search by block hash, transaction ID, wallet address, or contract address..."
          className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          disabled={loading}
        />
        <button
          onClick={handleSearch}
          disabled={loading}
          className="px-6 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {loading ? 'Searching...' : 'Search'}
        </button>
      </div>
      {error && (
        <div className="mt-4 text-red-600 text-sm">{error}</div>
      )}
    </div>
  );
}

