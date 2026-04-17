import { useState } from 'react'
import PageIntro from '../components/PageIntro'

export default function Governance() {
  const [info] = useState<string | null>(null)

  return (
    <>
      <PageIntro title="Governance">
        Gobernanza on-chain: propuestas de cambio de parámetros del protocolo, votación ponderada
        por stake, y ejecución tras timelock. El módulo de governance opera internamente; esta
        página muestra el estado cuando se exponga vía API.
      </PageIntro>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
        {[
          {
            title: 'Proposals',
            desc: 'Submit parameter change proposals (e.g. block size, fee rate, quorum threshold).',
          },
          {
            title: 'Voting',
            desc: 'Stake-weighted voting: Yes / No / Abstain. Quorum and threshold checks enforced.',
          },
          {
            title: 'Execution',
            desc: 'Passed proposals enter a timelock period before automatic execution.',
          },
        ].map((card) => (
          <div key={card.title} className="bg-white border border-neutral-200 rounded-2xl p-5">
            <h3 className="text-neutral-900 font-semibold mb-1">{card.title}</h3>
            <p className="text-neutral-500 text-sm">{card.desc}</p>
          </div>
        ))}
      </div>

      <div className="bg-amber-50 border border-amber-200 rounded-2xl p-5 text-center">
        <p className="text-amber-700 text-sm font-medium mb-1">Governance API coming soon</p>
        <p className="text-amber-600 text-xs">
          The governance module (ProposalStore, VoteStore, ParamRegistry) is fully implemented
          internally. HTTP endpoints for proposal submission and voting will be exposed in a future
          release.
        </p>
        {info && <p className="text-amber-600 text-xs mt-2">{info}</p>}
      </div>
    </>
  )
}
