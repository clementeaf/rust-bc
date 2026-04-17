import { useEffect, useState } from 'react'
import { getStats } from '../lib/api'

export default function ServerStatus() {
  const [online, setOnline] = useState<boolean | null>(null)

  useEffect(() => {
    const check = () => {
      getStats()
        .then(() => setOnline(true))
        .catch(() => setOnline(false))
    }
    check()
    const id = setInterval(check, 5000)
    return () => clearInterval(id)
  }, [])

  if (online === true) return null
  if (online === null)
    return (
      <div className="bg-amber-50 border border-amber-200 text-amber-700 px-4 py-2.5 rounded-2xl text-sm mb-6">
        Checking connection to blockchain node...
      </div>
    )
  return (
    <div className="bg-red-50 border border-red-200 text-red-600 px-4 py-2.5 rounded-2xl text-sm mb-6">
      Cannot reach the API (is the node up on port 8080?)
    </div>
  )
}
