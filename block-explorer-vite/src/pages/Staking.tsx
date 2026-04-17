import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import PageIntro from '../components/PageIntro'
import { getValidators, stakeTokens, requestUnstake, type Validator } from '../lib/api'

function shortAddr(a: string) {
  return a.length > 16 ? a.slice(0, 8) + '...' + a.slice(-8) : a
}

export default function Staking() {
  const [validators, setValidators] = useState<Validator[]>([])
  const [address, setAddress] = useState('')
  const [amount, setAmount] = useState('')
  const [staking, setStaking] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')

  const load = () => getValidators().then(setValidators).catch(() => {})

  useEffect(() => {
    load()
    const id = setInterval(load, 15000)
    return () => clearInterval(id)
  }, [])

  const handleStake = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    setSuccess('')
    setStaking(true)
    try {
      await stakeTokens(address, Number(amount))
      setSuccess(`Staked ${amount} from ${shortAddr(address)}`)
      setAddress('')
      setAmount('')
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Staking failed')
    } finally {
      setStaking(false)
    }
  }

  const handleUnstake = async (addr: string) => {
    try {
      await requestUnstake(addr)
      await load()
    } catch {
      // ignore
    }
  }

  const totalStaked = validators.reduce((s, v) => s + v.staked_amount, 0)
  const activeCount = validators.filter((v) => v.is_active).length

  return (
    <>
      <PageIntro title="Staking">
        Bloquea tokens para participar como validador. Mínimo 1,000 coins para ser elegible.
      </PageIntro>

      <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-8">
        {[
          { label: 'Total Staked', value: totalStaked },
          { label: 'Validators', value: validators.length },
          { label: 'Active', value: activeCount },
        ].map((s) => (
          <div key={s.label} className="bg-white border border-neutral-200 rounded-2xl p-4">
            <p className="text-neutral-500 text-xs uppercase tracking-wide">{s.label}</p>
            <p className="text-2xl font-bold text-neutral-900 mt-1">{s.value}</p>
          </div>
        ))}
      </div>

      <div className="bg-white border border-neutral-200 rounded-2xl p-5 mb-8">
        <h2 className="text-lg font-semibold text-neutral-900 mb-4">Stake Tokens</h2>
        <form onSubmit={handleStake} className="flex flex-col sm:flex-row gap-4">
          <input
            type="text"
            placeholder="Wallet address"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            required
            className="flex-1 border border-neutral-200 rounded-xl px-3 py-2 text-sm font-mono
                       focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <input
            type="number"
            placeholder="Amount"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            required
            min="1"
            className="w-32 border border-neutral-200 rounded-xl px-3 py-2 text-sm
                       focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <button
            type="submit"
            disabled={staking}
            className="bg-main-500 text-white px-4 py-2 rounded-xl text-sm font-medium
                       hover:bg-main-600 disabled:opacity-50 transition-colors"
          >
            {staking ? 'Staking...' : 'Stake'}
          </button>
        </form>
        {error && <p className="text-red-500 text-sm mt-3">{error}</p>}
        {success && <p className="text-green-600 text-sm mt-3">{success}</p>}
      </div>

      <h2 className="text-lg font-semibold text-neutral-900 mb-1">Validators</h2>
      <p className="text-xs text-neutral-400 mb-4">Nodos que han bloqueado tokens para validar bloques.</p>

      {validators.length === 0 ? (
        <div className="bg-white border border-neutral-200 rounded-2xl p-8 text-center">
          <p className="text-neutral-500">No validators yet. Stake at least 1,000 coins.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
                <th className="text-left py-3 px-2">Address</th>
                <th className="text-right py-3 px-2">Staked</th>
                <th className="text-right py-3 px-2">Rewards</th>
                <th className="text-center py-3 px-2">Status</th>
                <th className="text-center py-3 px-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              {validators.map((v) => (
                <tr key={v.address} className="border-b border-neutral-100 hover:bg-white">
                  <td className="py-3 px-2">
                    <Link
                      to={`/wallet/${v.address}`}
                      className="text-main-500 hover:text-main-600 font-mono text-xs"
                    >
                      {shortAddr(v.address)}
                    </Link>
                  </td>
                  <td className="py-3 px-2 text-right text-neutral-900 font-medium">
                    {v.staked_amount}
                  </td>
                  <td className="py-3 px-2 text-right text-green-600">{v.total_rewards}</td>
                  <td className="py-3 px-2 text-center">
                    {v.unstaking_requested ? (
                      <span className="text-amber-600 bg-amber-50 px-2 py-0.5 rounded text-xs">Unstaking</span>
                    ) : v.is_active ? (
                      <span className="text-green-600 bg-green-50 px-2 py-0.5 rounded text-xs">Active</span>
                    ) : (
                      <span className="text-neutral-500 bg-neutral-100 px-2 py-0.5 rounded text-xs">Inactive</span>
                    )}
                  </td>
                  <td className="py-3 px-2 text-center">
                    {v.is_active && !v.unstaking_requested && (
                      <button
                        onClick={() => handleUnstake(v.address)}
                        className="text-red-500 hover:text-red-600 text-xs font-medium"
                      >
                        Unstake
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  )
}
