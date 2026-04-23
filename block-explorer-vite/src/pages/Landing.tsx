import { useState } from 'react'
import { Link } from 'react-router-dom'

const tags = [
  {
    label: 'DLT',
    desc: 'Distributed Ledger Technology — un libro de registros compartido entre multiples participantes, donde la informacion se valida de forma colectiva y no depende de una entidad central.',
    detail: 'Cerulean Ledger usa consenso BFT + DAG: cada nodo valida de forma independiente, y la red converge sin necesitar un coordinador central.',
    metric: '2,741 tests automatizados verifican la integridad de la red',
  },
  {
    label: 'Post-Cuantica',
    desc: 'Las computadoras cuanticas podran romper la criptografia actual. Cerulean Ledger usa firmas de nueva generacion (FIPS 204) que ya estan preparadas para ese escenario.',
    detail: 'Cada transaccion se firma con ML-DSA-65 (3,309 bytes) o Ed25519 (64 bytes). Ambos algoritmos coexisten en la misma red.',
    metric: 'Estandar NIST FIPS 204 — aprobado agosto 2024',
  },
  {
    label: 'Identidad Soberana',
    desc: 'Personas y organizaciones gestionan su propia identidad digital. Nadie mas la controla, nadie mas la puede revocar. Tu identidad es tuya.',
    detail: 'Formato: did:cerulean:identificador. Las credenciales (titulos, certificados) se emiten entre DIDs y se verifican en milisegundos.',
    metric: 'Verificacion criptografica en <50ms vs 3-15 dias habiles manual',
  },
  {
    label: 'Wasm + EVM',
    desc: 'Permite ejecutar logica de negocio directamente en la red, usando las mismas herramientas del ecosistema Ethereum y el rendimiento de WebAssembly.',
    detail: 'Contratos Solidity se ejecutan via revm (la misma EVM de Reth/Foundry). Chaincode Wasm corre en Wasmtime con gas metering.',
    metric: 'Compatible con MetaMask, Hardhat y todo el ecosistema Ethereum',
  },
]

const rivals = [
  {
    label: 'Fabric',
    items: [
      { feature: 'Criptografia', them: 'ECDSA', us: 'ML-DSA-65 + Ed25519' },
      { feature: 'Consenso', them: 'Raft (solo crash)', us: 'BFT + DAG (bizantino)' },
      { feature: 'Identidad (DID)', them: 'Via Indy (externo)', us: 'Nativo' },
      { feature: 'Credenciales', them: '—', us: 'Emision + verificacion' },
      { feature: 'Smart contracts', them: 'Go / Java / Node', us: 'Wasm + EVM (revm)' },
      { feature: 'Nodos', them: 'Solo consorcio', us: 'Cualquier entidad' },
    ],
  },
  {
    label: 'IOTA',
    items: [
      { feature: 'Criptografia', them: 'Ed25519', us: 'ML-DSA-65 + Ed25519' },
      { feature: 'Consenso', them: 'Tangle (probabilistico)', us: 'BFT + DAG (deterministico)' },
      { feature: 'Canales privados', them: '—', us: 'Aislamiento completo' },
      { feature: 'Credenciales', them: '—', us: 'Emision + verificacion' },
      { feature: 'Smart contracts', them: 'MoveVM', us: 'Wasm + EVM (revm)' },
      { feature: 'Nodos', them: 'IOTA Foundation', us: 'Cualquier entidad' },
    ],
  },
  {
    label: 'Hedera',
    items: [
      { feature: 'Criptografia', them: 'ECDSA', us: 'ML-DSA-65 + Ed25519' },
      { feature: 'Consenso', them: 'Hashgraph (aBFT)', us: 'BFT + DAG' },
      { feature: 'Identidad (DID)', them: '—', us: 'Nativo' },
      { feature: 'Canales privados', them: 'HashSphere (pago)', us: 'Nativo (sin costo)' },
      { feature: 'Smart contracts', them: 'EVM (~15 TPS)', us: 'Wasm + EVM (revm)' },
      { feature: 'Nodos', them: 'Solo Consejo (31 empresas)', us: 'Cualquier entidad' },
    ],
  },
]

type RightTab = 'conceptos' | 'comparativa'

export default function Landing() {
  const [selected, setSelected] = useState(0)
  const [selectedRival, setSelectedRival] = useState(0)
  const [rightTab, setRightTab] = useState<RightTab>('conceptos')

  return (
    <div className="h-screen flex flex-col overflow-hidden">
      {/* Hero */}
      <section className="flex-1 flex items-center px-6 max-w-screen-xl mx-auto w-full">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 lg:gap-24 w-full">
          {/* Left — value proposition */}
          <div className="text-left flex flex-col justify-center">
            <p className="text-3xl sm:text-4xl font-bold text-main-500 tracking-tight mb-2">Cerulean Ledger</p>
            <p className="text-sm text-neutral-500 mb-4">
              Seguridad resistente a computacion cuantica
            </p>
            <h1 className="text-xl sm:text-2xl font-bold text-neutral-900 tracking-tight leading-tight">
              Infraestructura DLT con soberania tecnologica
            </h1>
            <p className="text-neutral-500 text-base mt-4 leading-relaxed">
              Registro distribuido con criptografia post-cuantica, identidad descentralizada
              y canales privados. Codigo abierto, escrito en Rust.
            </p>
            <div className="flex gap-3 mt-8">
              <Link
                to="/services"
                className="bg-main-500 text-white px-6 py-2.5 rounded-xl text-sm font-semibold
                           hover:bg-main-600 transition-colors shadow-sm hover:shadow-md cursor-pointer inline-block"
              >
                Ver servicios
              </Link>
              <Link
                to="/tesseract"
                className="bg-neutral-100 text-neutral-600 px-5 py-2.5 rounded-xl text-sm font-semibold
                           hover:bg-neutral-200 transition-colors cursor-pointer inline-block"
              >
                Tesseract
              </Link>
            </div>
          </div>

          {/* Right — switchable module */}
          <div className="hidden lg:flex flex-col justify-center min-h-[340px]">
            {/* Module tabs */}
            <div className="flex gap-1 mb-4 relative z-20">
              {(['conceptos', 'comparativa'] as const).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setRightTab(tab)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                    rightTab === tab
                      ? 'bg-main-500 text-white'
                      : 'text-neutral-400 hover:text-neutral-600'
                  }`}
                >
                  {tab === 'conceptos' ? 'Conceptos' : 'Comparativa'}
                </button>
              ))}
            </div>

            {/* Content area — relative container for the drawer overlay */}
            <div className="flex-1 flex flex-col relative">

            {/* Conceptos module */}
            {rightTab === 'conceptos' && (
              <div className="flex-1 flex flex-col">
                <div className="flex flex-wrap gap-2">
                  {tags.map((t, i) => (
                    <button
                      key={t.label}
                      onClick={() => setSelected(i)}
                      className={`px-4 py-2 rounded-xl text-sm font-semibold transition-all duration-150 cursor-pointer ${
                        selected === i
                          ? 'bg-main-500 text-white'
                          : 'bg-neutral-100 text-neutral-600 hover:bg-neutral-200'
                      }`}
                    >
                      {t.label}
                    </button>
                  ))}
                </div>
                <div className="mt-4 bg-white border border-neutral-200 rounded-2xl px-5 py-4 flex-1 flex flex-col justify-between">
                  <div>
                    <p className="text-neutral-700 text-sm leading-relaxed">{tags[selected].desc}</p>
                    <p className="text-neutral-500 text-xs leading-relaxed mt-3">{tags[selected].detail}</p>
                  </div>
                  <div className="mt-4 pt-3 border-t border-neutral-100">
                    <p className="text-main-600 text-xs font-semibold">{tags[selected].metric}</p>
                  </div>
                </div>
              </div>
            )}

            {/* Comparativa module */}
            {rightTab === 'comparativa' && (
              <div className="flex-1 flex flex-col">
                <div className="flex flex-wrap gap-2">
                  {rivals.map((r, i) => (
                    <button
                      key={r.label}
                      onClick={() => setSelectedRival(i)}
                      className={`px-4 py-2 rounded-xl text-sm font-semibold transition-all duration-150 cursor-pointer ${
                        selectedRival === i
                          ? 'bg-main-500 text-white'
                          : 'bg-neutral-100 text-neutral-600 hover:bg-neutral-200'
                      }`}
                    >
                      vs {r.label}
                    </button>
                  ))}
                </div>
                <div className="mt-4 bg-white border border-neutral-200 rounded-2xl px-5 py-4 flex-1">
                  <div className="space-y-2.5">
                    {rivals[selectedRival].items.map((item) => (
                      <div key={item.feature} className="flex items-start gap-3">
                        <p className="text-neutral-700 text-xs font-medium w-28 shrink-0 pt-0.5">{item.feature}</p>
                        <div className="flex-1 flex gap-3">
                          <p className="text-main-600 text-xs font-semibold flex-1">{item.us}</p>
                          <p className="text-neutral-600 text-xs flex-1">{item.them}</p>
                        </div>
                      </div>
                    ))}
                  </div>
                  <div className="mt-3 pt-2 border-t border-neutral-100 flex justify-between text-[10px] text-neutral-300 uppercase tracking-wider">
                    <span></span>
                    <span className="flex gap-6">
                      <span className="text-main-500">Cerulean</span>
                      <span>{rivals[selectedRival].label}</span>
                    </span>
                  </div>
                </div>
              </div>
            )}

            </div>{/* close content area relative */}
          </div>
        </div>
      </section>

      {/* Pillars */}
      <section className="border-t border-neutral-200 bg-surface-alt px-6 py-6">
        <div className="max-w-4xl mx-auto grid grid-cols-2 sm:grid-cols-4 gap-5">
          {[
            { title: 'Criptografia post-cuantica', desc: 'ML-DSA-65 (FIPS 204) + Ed25519.', color: 'bg-emerald-500' },
            { title: 'Identidad soberana', desc: 'DIDs + credenciales verificables.', color: 'bg-violet-500' },
            { title: 'Canales privados', desc: 'Aislamiento por organizacion.', color: 'bg-blue-500' },
            { title: 'Smart contracts', desc: 'Wasm + EVM con ejecucion paralela.', color: 'bg-amber-500' },
          ].map((p) => (
            <div key={p.title} className="flex items-start gap-2.5">
              <div className={`w-2 h-2 rounded-full mt-1.5 ${p.color} shrink-0`} />
              <div>
                <p className="text-neutral-900 font-semibold text-sm">{p.title}</p>
                <p className="text-neutral-500 text-xs leading-relaxed mt-1">{p.desc}</p>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-neutral-200 px-6 py-3">
        <div className="max-w-4xl mx-auto flex items-center justify-between text-[11px] text-neutral-400">
          <span>Cerulean Ledger</span>
          <div className="flex items-center gap-4">
            <span>Rust</span>
            <span>BFT + DAG</span>
            <span>Open source</span>
          </div>
        </div>
      </footer>
    </div>
  )
}
