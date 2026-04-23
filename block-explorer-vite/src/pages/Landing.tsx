import { Link } from 'react-router-dom'

const pillars = [
  {
    title: 'Criptografia post-cuantica',
    desc: 'Firmas ML-DSA-65 (FIPS 204) resistentes a computacion cuantica. Seguridad a largo plazo sin migraciones futuras.',
    color: 'bg-emerald-500',
  },
  {
    title: 'Identidad soberana',
    desc: 'Identidades descentralizadas (DID) y credenciales verificables. Sin intermediarios, sin dependencia externa.',
    color: 'bg-violet-500',
  },
  {
    title: 'Canales privados',
    desc: 'Redes aisladas con su propio ledger y world state. Cada organizacion controla sus datos.',
    color: 'bg-blue-500',
  },
  {
    title: 'Smart contracts',
    desc: 'Contratos en WebAssembly y compatibilidad EVM. Ejecucion paralela con deteccion de conflictos.',
    color: 'bg-amber-500',
  },
]

const specs = [
  { label: 'Lenguaje', value: 'Rust' },
  { label: 'Consenso', value: 'BFT + DAG' },
  { label: 'Firmas', value: 'ML-DSA-65 + Ed25519' },
  { label: 'Licencia', value: 'Open source' },
]

export default function Landing() {
  return (
    <div className="min-h-screen flex flex-col">
      {/* Header */}
      <header className="sticky top-0 z-50 bg-white/80 backdrop-blur-xl border-b border-neutral-200">
        <div className="max-w-5xl mx-auto px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-xl bg-main-500 flex items-center justify-center">
              <span className="text-white font-bold text-sm">CL</span>
            </div>
            <span className="text-lg font-bold text-neutral-900 tracking-tight">Cerulean Ledger</span>
          </div>
          <Link
            to="/dashboard"
            className="text-sm text-main-500 hover:text-main-600 font-medium transition-colors"
          >
            Explorar la red
          </Link>
        </div>
      </header>

      {/* Hero */}
      <section className="flex-1 flex flex-col items-center justify-center px-6 py-20 sm:py-28">
        <div className="max-w-2xl text-center">
          <div className="inline-flex items-center gap-2 text-xs text-neutral-400 mb-6">
            <span className="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
            PQC-ready · ML-DSA-65 + Ed25519
          </div>
          <h1 className="text-4xl sm:text-5xl font-bold text-neutral-900 tracking-tight leading-tight">
            Infraestructura DLT con soberania tecnologica
          </h1>
          <p className="text-neutral-500 text-lg mt-6 leading-relaxed max-w-xl mx-auto">
            Plataforma de registro distribuido con criptografia post-cuantica, identidad
            descentralizada y canales privados. Codigo abierto, escrito en Rust.
          </p>
          <div className="flex flex-col sm:flex-row items-center justify-center gap-3 mt-10">
            <Link
              to="/demo"
              className="bg-main-500 text-white px-6 py-3 rounded-xl text-sm font-semibold
                         hover:bg-main-600 transition-colors shadow-sm hover:shadow-md w-full sm:w-auto text-center"
            >
              Ver demo en vivo
            </Link>
            <Link
              to="/dashboard"
              className="bg-white text-neutral-700 border border-neutral-200 px-6 py-3 rounded-xl text-sm font-semibold
                         hover:bg-neutral-50 hover:border-neutral-300 transition-colors w-full sm:w-auto text-center"
            >
              Explorar la red
            </Link>
          </div>
        </div>
      </section>

      {/* Pillars */}
      <section className="border-t border-neutral-200 bg-surface-alt">
        <div className="max-w-5xl mx-auto px-6 py-16">
          <h2 className="text-xs font-bold text-neutral-400 uppercase tracking-widest mb-8 text-center">
            Capacidades
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
            {pillars.map((p) => (
              <div
                key={p.title}
                className="bg-white border border-neutral-200 rounded-2xl p-5"
              >
                <div className="flex items-start gap-3">
                  <div className={`w-2 h-2 rounded-full mt-2 ${p.color} shrink-0`} />
                  <div>
                    <h3 className="text-neutral-900 font-semibold text-sm">{p.title}</h3>
                    <p className="text-neutral-500 text-xs mt-1 leading-relaxed">{p.desc}</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Tech specs */}
      <section className="border-t border-neutral-200">
        <div className="max-w-5xl mx-auto px-6 py-12">
          <div className="flex flex-wrap items-center justify-center gap-8">
            {specs.map((s) => (
              <div key={s.label} className="text-center">
                <p className="text-neutral-400 text-[10px] uppercase tracking-widest">{s.label}</p>
                <p className="text-neutral-900 font-semibold text-sm mt-1">{s.value}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-neutral-200 py-4">
        <div className="max-w-5xl mx-auto px-6 flex items-center justify-between text-xs text-neutral-400">
          <span>Cerulean Ledger</span>
          <span>DLT post-cuantica · Soberania digital</span>
        </div>
      </footer>
    </div>
  )
}
