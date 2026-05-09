import { useState, useEffect, useRef } from 'react'
import { Link } from 'react-router-dom'

interface NetworkStats {
  height: number
  peers: string
  status: string
}

interface OracleFeed {
  symbol: string
  price: number
  is_stale: boolean
}

export default function Landing() {
  const [stats, setStats] = useState<NetworkStats | null>(null)
  const [oracle, setOracle] = useState<OracleFeed | null>(null)
  const [activeUse, setActiveUse] = useState(0)

  useEffect(() => { document.title = 'Cerulean Ledger' }, [])

  // Fetch live network stats + oracle feed
  useEffect(() => {
    const load = async () => {
      try {
        const res = await fetch('/api/v1/health')
        const json = await res.json()
        const d = json.data
        setStats({
          height: d.blockchain.height,
          peers: d.checks.peers === 'none' ? '0' : d.checks.peers,
          status: d.status,
        })
      } catch { /* offline */ }

      try {
        const res = await fetch('/api/v1/oracle/feeds')
        const json = await res.json()
        if (json.data?.length > 0) {
          const btc = json.data.find((f: OracleFeed) => f.symbol.includes('BTC')) || json.data[0]
          setOracle(btc)
        }
      } catch { /* no oracle */ }
    }
    load()
    const interval = setInterval(load, 10000)
    return () => clearInterval(interval)
  }, [])

  const uses = [
    {
      title: 'Credenciales',
      headline: 'Un titulo verificable en segundos, valido por decadas',
      body: 'Las instituciones emiten credenciales digitales firmadas con criptografia post-cuantica. Cualquier persona verifica su autenticidad desde el celular, sin llamar a nadie. El titulo de hoy seguira siendo verificable cuando los computadores cuanticos sean una realidad.',
      audience: 'Universidades · Colegios profesionales · RRHH',
    },
    {
      title: 'Votacion',
      headline: 'Cada voto sellado. Ningun resultado alterable.',
      body: 'La votacion queda registrada en una red distribuida donde ningun participante — ni siquiera el administrador — puede modificar el resultado. El escrutinio es publico y verificable sin exponer la identidad de los votantes.',
      audience: 'Municipios · Cooperativas · Juntas directivas',
    },
    {
      title: 'Trazabilidad',
      headline: 'La cadena de custodia que no depende de la confianza',
      body: 'Multiples organizaciones comparten un registro comun donde cada una ve solo su parte. Cada punto de control queda sellado. Ante una disputa, la evidencia es matematica, no testimonial.',
      audience: 'Mineria · Agroindustria · Farmaceutica',
    },
    {
      title: 'Finanzas',
      headline: 'Conciliacion instantanea entre instituciones',
      body: 'Un registro compartido e inmutable entre bancos o reguladores. Compatible con ISO 20022, ISO 4217 y ERC-3643 para security tokens. El auditor verifica en minutos lo que antes tomaba semanas.',
      audience: 'Banca · Fintech · Reguladores',
    },
  ]

  return (
    <div className="min-h-screen flex flex-col bg-[#fafbfc]">

      {/* ── Hero ───────────────────────────────────────────────────────── */}
      <section className="px-6 pt-20 pb-16">
        <div className="max-w-screen-xl mx-auto">
          <div className="max-w-2xl">
            <p className="text-main-500 font-semibold text-xs uppercase tracking-widest mb-4">Cerulean Ledger</p>
            <h1 className="text-4xl sm:text-5xl font-bold text-neutral-900 tracking-tight leading-[1.15]">
              La confianza deja de ser<br />una promesa
            </h1>
            <p className="text-neutral-500 text-base mt-5 leading-relaxed max-w-lg">
              Infraestructura de verificacion para instituciones que necesitan
              garantizar — no prometer — que sus registros son autenticos,
              inmutables y verificables por cualquiera.
            </p>
            <div className="flex flex-wrap gap-3 mt-8">
              <a
                href="/api/v1/health"
                target="_blank"
                rel="noreferrer"
                className="bg-main-500 text-white px-6 py-2.5 rounded-lg text-sm font-semibold
                           hover:bg-main-600 transition-colors cursor-pointer inline-block"
              >
                Ver API en vivo
              </a>
              <Link
                to="/dashboard"
                className="text-neutral-500 px-5 py-2.5 rounded-lg text-sm font-semibold
                           hover:text-neutral-700 transition-colors cursor-pointer inline-block"
              >
                Dashboard
              </Link>
            </div>
          </div>

          {/* Live network pulse */}
          {stats && (
            <div className="mt-12 flex items-center gap-6 text-xs text-neutral-400">
              <div className="flex items-center gap-1.5">
                <span className={`w-1.5 h-1.5 rounded-full ${stats.status === 'healthy' ? 'bg-green-400 animate-pulse' : 'bg-neutral-300'}`} />
                <span>Red activa</span>
              </div>
              <span>Bloque #{stats.height}</span>
              <span>Peers: {stats.peers}</span>
              {oracle && (
                <span className="flex items-center gap-1.5">
                  <span className={`w-1.5 h-1.5 rounded-full ${oracle.is_stale ? 'bg-amber-400' : 'bg-blue-400 animate-pulse'}`} />
                  {oracle.symbol}: ${(oracle.price / 100).toLocaleString()}
                </span>
              )}
            </div>
          )}
        </div>
      </section>

      {/* ── Tesis ──────────────────────────────────────────────────────── */}
      <section className="px-6 py-14">
        <div className="max-w-screen-xl mx-auto grid grid-cols-1 md:grid-cols-3 gap-8">
          {[
            {
              title: 'Integridad por diseno',
              body: 'Los registros no son protegidos por politicas ni por personas. Son protegidos por matematicas. Alterar un dato requiere romper criptografia que hoy no se puede romper — y manana tampoco.',
            },
            {
              title: 'Privacidad sin sacrificio',
              body: 'Cada organizacion opera en su propio espacio aislado. Comparte lo que necesita, oculta lo que no. La verificacion es publica; los datos son privados.',
            },
            {
              title: 'Soberania operacional',
              body: 'Corre en tu infraestructura. No dependes de una nube, un token cotizado, ni una empresa que puede desaparecer. Tu red, tus reglas, tus datos.',
            },
          ].map((t) => (
            <div key={t.title}>
              <h3 className="font-bold text-sm text-neutral-900 mb-2">{t.title}</h3>
              <p className="text-sm text-neutral-500 leading-relaxed">{t.body}</p>
            </div>
          ))}
        </div>
      </section>

      {/* ── Usos ───────────────────────────────────────────────────────── */}
      <section className="bg-white border-y border-neutral-100 px-6 py-14">
        <div className="max-w-screen-xl mx-auto">
          <div className="flex flex-wrap gap-2 mb-8">
            {uses.map((u, i) => (
              <button
                key={u.title}
                onClick={() => setActiveUse(i)}
                className={`px-4 py-2 rounded-lg text-sm font-semibold transition-all cursor-pointer ${
                  activeUse === i
                    ? 'bg-neutral-900 text-white'
                    : 'text-neutral-400 hover:text-neutral-600'
                }`}
              >
                {u.title}
              </button>
            ))}
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-10 items-start">
            <div>
              <h2 className="text-2xl font-bold text-neutral-900 leading-tight mb-4">
                {uses[activeUse].headline}
              </h2>
              <p className="text-neutral-500 text-sm leading-relaxed mb-4">
                {uses[activeUse].body}
              </p>
              <p className="text-xs text-neutral-300">{uses[activeUse].audience}</p>
            </div>

            <div className="bg-neutral-50 rounded-xl p-6 space-y-3">
              {[
                { label: 'Registrar', desc: 'La institucion emite el dato firmado digitalmente.' },
                { label: 'Sellar', desc: 'Consenso distribuido lo incorpora a un bloque inmutable.' },
                { label: 'Verificar', desc: 'Cualquier autorizado comprueba autenticidad al instante.' },
              ].map((step, i) => (
                <div key={step.label} className="flex items-start gap-3">
                  <span className="w-5 h-5 rounded-full bg-neutral-200 text-neutral-500 flex items-center justify-center text-[10px] font-bold shrink-0 mt-0.5">
                    {i + 1}
                  </span>
                  <div>
                    <p className="text-sm font-semibold text-neutral-700">{step.label}</p>
                    <p className="text-xs text-neutral-400">{step.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* ── Numeros ────────────────────────────────────────────────────── */}
      <section className="px-6 py-14">
        <div className="max-w-screen-xl mx-auto">
          <div className="grid grid-cols-2 lg:grid-cols-5 gap-6">
            {[
              { n: '18,700', label: 'TX/s motor' },
              { n: '14', label: 'ms latencia (p50)' },
              { n: '1,604', label: 'tests automatizados' },
              { n: '20', label: 'ataques adversariales' },
              { n: '193', label: 'paises ISO 3166' },
            ].map((m) => (
              <div key={m.label}>
                <p className="text-2xl font-bold text-neutral-900">{m.n}</p>
                <p className="text-[10px] text-neutral-400 uppercase tracking-wide mt-0.5">{m.label}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ── Validacion ─────────────────────────────────────────────────── */}
      <section className="border-t border-neutral-100 px-6 py-10">
        <div className="max-w-screen-xl mx-auto">
          <div className="flex flex-col sm:flex-row items-start sm:items-center gap-6">
            <div className="flex-1">
              <p className="text-sm text-neutral-700 leading-relaxed">
                <span className="font-semibold">Auditado por la Camara Blockchain de Chile.</span>{' '}
                Cuatro fases de evaluacion — motor, nivel transaccional, contratos inteligentes,
                seguridad y encriptacion. Resultado:{' '}
                <span className="text-main-600 font-semibold">"Supera las expectativas. El core es muy bueno."</span>
              </p>
            </div>
            <div className="shrink-0 flex flex-wrap gap-4 text-center">
              {[
                { label: 'Ley 21.663', sub: 'Ciberseguridad' },
                { label: 'FIPS 204', sub: 'Post-cuantica' },
                { label: 'ISO 20022', sub: 'Financiero' },
                { label: '20/20', sub: 'Pentest adversarial' },
              ].map((c) => (
                <div key={c.label} className="px-3">
                  <p className="font-bold text-xs text-neutral-600">{c.label}</p>
                  <p className="text-[9px] text-neutral-400">{c.sub}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* ── Contacto ──────────────────────────────────────────────────── */}
      <ContactSection />

      {/* ── Footer ──────────────────────────────────────────────────────── */}
      <footer className="border-t border-neutral-100 px-6 py-3">
        <div className="max-w-screen-xl mx-auto flex items-center justify-between text-[11px] text-neutral-300">
          <span>Cerulean Ledger</span>
          <span>Rust · PQC · Open source</span>
        </div>
      </footer>
    </div>
  )
}

function ContactSection() {
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [org, setOrg] = useState('')
  const [message, setMessage] = useState('')
  const [status, setStatus] = useState<'idle' | 'sending' | 'sent' | 'error'>('idle')
  const formRef = useRef<HTMLFormElement>(null)

  const canSend = name.trim() && email.trim() && message.trim() && status !== 'sending'

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!canSend) return
    setStatus('sending')
    try {
      const res = await fetch('/api/v1/contact', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Org-Id': 'landing',
          'X-Msp-Role': 'client',
        },
        body: JSON.stringify({
          name: name.trim(),
          email: email.trim(),
          organization: org.trim() || null,
          message: message.trim(),
        }),
      })
      if (res.ok) {
        setStatus('sent')
        setName('')
        setEmail('')
        setOrg('')
        setMessage('')
      } else {
        setStatus('error')
      }
    } catch {
      setStatus('error')
    }
  }

  if (status === 'sent') {
    return (
      <section className="border-t border-neutral-100 px-6 py-14">
        <div className="max-w-screen-xl mx-auto text-center">
          <p className="text-lg font-bold text-neutral-900 mb-2">Mensaje recibido</p>
          <p className="text-sm text-neutral-500">Nos pondremos en contacto pronto.</p>
          <button
            onClick={() => setStatus('idle')}
            className="mt-4 text-xs text-main-500 hover:underline cursor-pointer"
          >
            Enviar otro mensaje
          </button>
        </div>
      </section>
    )
  }

  return (
    <section className="border-t border-neutral-100 px-6 py-14">
      <div className="max-w-screen-xl mx-auto">
        <div className="max-w-lg mx-auto">
          <h2 className="text-2xl font-bold text-neutral-900 mb-2 text-center">
            Solicite una demostracion
          </h2>
          <p className="text-sm text-neutral-400 mb-6 text-center">
            Sin costo, sin compromiso. Le contactamos en menos de 24 horas.
          </p>

          <form ref={formRef} onSubmit={handleSubmit} className="space-y-3">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Nombre"
                className="w-full rounded-lg border border-neutral-200 px-3 py-2 text-sm"
              />
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="Email"
                className="w-full rounded-lg border border-neutral-200 px-3 py-2 text-sm"
              />
            </div>
            <input
              type="text"
              value={org}
              onChange={(e) => setOrg(e.target.value)}
              placeholder="Organizacion (opcional)"
              className="w-full rounded-lg border border-neutral-200 px-3 py-2 text-sm"
            />
            <textarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder="En que podemos ayudarle?"
              rows={3}
              className="w-full rounded-lg border border-neutral-200 px-3 py-2 text-sm"
            />
            <button
              type="submit"
              disabled={!canSend}
              className={`w-full py-2.5 rounded-lg text-sm font-semibold transition-colors ${
                canSend
                  ? 'bg-neutral-900 text-white hover:bg-neutral-800 cursor-pointer'
                  : 'bg-neutral-100 text-neutral-300 cursor-not-allowed'
              }`}
            >
              {status === 'sending' ? 'Enviando...' : 'Enviar mensaje'}
            </button>
            {status === 'error' && (
              <p className="text-xs text-red-500 text-center">Error al enviar. Intente nuevamente.</p>
            )}
          </form>
        </div>
      </div>
    </section>
  )
}
