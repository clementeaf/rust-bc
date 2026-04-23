import { useState } from 'react'
import { Link } from 'react-router-dom'

const tags = [
  {
    label: 'DLT',
    desc: 'Distributed Ledger Technology — un libro de registros compartido entre multiples participantes, donde la informacion se valida de forma colectiva y no depende de una entidad central.',
  },
  {
    label: 'Post-Cuantica',
    desc: 'Las computadoras cuanticas podran romper la criptografia actual. Cerulean Ledger usa firmas de nueva generacion (FIPS 204) que ya estan preparadas para ese escenario.',
  },
  {
    label: 'Identidad Soberana',
    desc: 'Personas y organizaciones gestionan su propia identidad digital. Nadie mas la controla, nadie mas la puede revocar. Tu identidad es tuya.',
  },
  {
    label: 'Wasm + EVM',
    desc: 'Permite ejecutar logica de negocio directamente en la red, usando las mismas herramientas del ecosistema Ethereum y el rendimiento de WebAssembly.',
  },
]

const pillars = [
  {
    title: 'Criptografia post-cuantica',
    desc: 'Seguridad resistente a computacion cuantica (ML-DSA-65, FIPS 204).',
    color: 'bg-emerald-500',
  },
  {
    title: 'Identidad soberana',
    desc: 'Identidades descentralizadas y credenciales verificables.',
    color: 'bg-violet-500',
  },
  {
    title: 'Canales privados',
    desc: 'Redes aisladas donde cada organizacion controla sus datos.',
    color: 'bg-blue-500',
  },
  {
    title: 'Smart contracts',
    desc: 'Contratos WebAssembly + EVM con ejecucion paralela.',
    color: 'bg-amber-500',
  },
]

export default function Landing() {
  const [selected, setSelected] = useState(0)

  return (
    <div className="h-screen flex flex-col overflow-hidden">
      {/* Hero */}
      <section className="flex-1 flex items-center px-6 max-w-screen-xl mx-auto w-full">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 lg:gap-48 w-full">
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
            <div className="flex flex-row items-center gap-3 mt-8">
              <Link
                to="/demo"
                className="bg-main-500 text-white px-6 py-2.5 rounded-xl text-sm font-semibold
                           hover:bg-main-600 transition-colors shadow-sm hover:shadow-md text-center"
              >
                Ver demo en vivo
              </Link>
              <Link
                to="/dashboard"
                className="bg-white text-neutral-700 border border-neutral-200 px-6 py-2.5 rounded-xl text-sm font-semibold
                           hover:bg-neutral-50 hover:border-neutral-300 transition-colors text-center"
              >
                Explorar la red
              </Link>
            </div>
          </div>

          {/* Right — tags row + description */}
          <div className="hidden lg:flex flex-col justify-center">
            <div className="flex flex-wrap gap-2">
              {tags.map((t, i) => (
                <button
                  key={t.label}
                  onClick={() => setSelected(i)}
                  className={`px-4 py-2 rounded-xl text-sm font-semibold transition-all duration-150 ${
                    selected === i
                      ? 'bg-main-500 text-white'
                      : 'bg-neutral-100 text-neutral-600 hover:bg-neutral-200'
                  }`}
                >
                  {t.label}
                </button>
              ))}
            </div>
            <div className="mt-4 bg-white border border-neutral-200 rounded-2xl px-5 py-4">
              <p className="text-neutral-700 text-sm leading-relaxed">{tags[selected].desc}</p>
            </div>
          </div>
        </div>
      </section>

      {/* Pillars */}
      <section className="border-t border-neutral-200 bg-surface-alt px-6 py-6">
        <div className="max-w-4xl mx-auto grid grid-cols-2 sm:grid-cols-4 gap-5">
          {pillars.map((p) => (
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
