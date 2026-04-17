import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import PageIntro from '../components/PageIntro'
import { getValidators, type Validator } from '../lib/api'

function shortAddr(a: string) {
  return a.length > 16 ? a.slice(0, 8) + '...' + a.slice(-8) : a
}

export default function Validators() {
  const [validators, setValidators] = useState<Validator[]>([])

  useEffect(() => {
    const load = () => getValidators().then(setValidators).catch(() => {})
    load()
    const id = setInterval(load, 30000)
    return () => clearInterval(id)
  }, [])

  return (
    <>
      <PageIntro title="Validadores (staking)">
        Cuentas que han bloqueado moneda para participar en la prueba de participación (PoS). Aquí ves
        stake, recompensas y si el validador sigue activo.
      </PageIntro>

      {validators.length === 0 ? (
        <div className="bg-white border border-neutral-200 rounded-2xl p-8 text-center">
          <p className="text-neutral-500 mb-2">No validators yet.</p>
          <p className="text-neutral-400 text-sm">Stake at least 1,000 coins to become a validator.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
                <th className="text-left py-3 px-2">Address</th>
                <th className="text-right py-3 px-2">Staked</th>
                <th className="text-right py-3 px-2">Rewards</th>
                <th className="text-right py-3 px-2">Validations</th>
                <th className="text-center py-3 px-2">Status</th>
              </tr>
            </thead>
            <tbody>
              {validators.map((v) => (
                <tr key={v.address} className="border-b border-neutral-100 hover:bg-white">
                  <td className="py-3 px-2">
                    <Link to={`/wallet/${v.address}`} className="text-main-500 hover:text-main-600 font-mono text-xs">
                      {shortAddr(v.address)}
                    </Link>
                  </td>
                  <td className="py-3 px-2 text-right text-neutral-900 font-medium">{v.staked_amount}</td>
                  <td className="py-3 px-2 text-right text-green-600">{v.total_rewards}</td>
                  <td className="py-3 px-2 text-right">{v.validation_count}</td>
                  <td className="py-3 px-2 text-center">
                    {v.unstaking_requested ? (
                      <span className="text-amber-600 bg-amber-50 px-2 py-0.5 rounded text-xs">Unstaking</span>
                    ) : v.is_active ? (
                      <span className="text-green-600 bg-green-50 px-2 py-0.5 rounded text-xs">Active</span>
                    ) : (
                      <span className="text-neutral-500 bg-neutral-100 px-2 py-0.5 rounded text-xs">Inactive</span>
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
