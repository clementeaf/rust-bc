'use client';

import { useEffect, useState } from 'react';
import { getStats } from '@/lib/api';

/**
 * Componente que verifica el estado del servidor backend
 */
export default function ServerStatus() {
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [checking, setChecking] = useState(true);

  useEffect(() => {
    checkServerStatus();
    const interval = setInterval(checkServerStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  async function checkServerStatus() {
    try {
      setChecking(true);
      await getStats();
      setIsOnline(true);
    } catch (err) {
      setIsOnline(false);
    } finally {
      setChecking(false);
    }
  }

  if (isOnline === null || checking) {
    return (
      <div className="bg-yellow-50 border border-yellow-200 text-yellow-800 px-4 py-3 rounded mb-6">
        <p className="font-medium">⏳ Checking server connection...</p>
      </div>
    );
  }

  if (!isOnline) {
    return (
      <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded mb-6">
        <p className="font-medium">❌ Server Connection Error</p>
        <p className="text-sm mt-2">
          The backend server is not running. Please start it with:
        </p>
        <code className="block mt-2 p-2 bg-red-100 rounded text-sm">
          cargo run
        </code>
        <p className="text-sm mt-2">
          Or check the <code className="bg-red-100 px-1 rounded">README_START.md</code> file for instructions.
        </p>
      </div>
    );
  }

  return null;
}

