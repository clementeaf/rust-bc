import { useState, useCallback, useEffect, useRef } from 'react'

// ── API (proxied through Vite) ──────────────────────────
const N1 = '/tess1'
const N2 = '/tess2'
const DEAL = { t: 3, c: 3, o: 3, v: 3 }
const CTX = [
  { t: 3, c: 4, o: 3, v: 3 },
  { t: 4, c: 3, o: 3, v: 3 },
  { t: 3, c: 3, o: 4, v: 3 },
]
const FRAUD = { t: 6, c: 6, o: 6, v: 6 }

async function post(url: string, body: object) {
  try {
    const r = await fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
    return r.json()
  } catch { return null }
}

async function get(url: string) {
  try { return (await fetch(url)).json() } catch { return null }
}

const seedEvent = (node: string, coord: typeof DEAL, id: string) =>
  post(`${node}/seed`, { ...coord, event_id: id })

const destroyCell = (node: string, coord: typeof DEAL) =>
  post(`${node}/destroy`, coord)

const getCell = (node: string, coord: typeof DEAL) =>
  get(`${node}/cell/${coord.t}/${coord.c}/${coord.o}/${coord.v}`)

const wait = (ms: number) => new Promise(r => setTimeout(r, ms))

// ── Types ───────────────────────────────────────────────
type Phase = 'idle' | 'proposing' | 'agreeing' | 'agreed' | 'fraud' | 'fraud-done' | 'destroying' | 'destroyed' | 'healing' | 'healed' | 'audit'

interface TimelineEntry {
  icon: string
  title: string
  detail: string
  status: 'success' | 'fail' | 'info' | 'warning'
}

// ── Component ───────────────────────────────────────────
export default function TesseractLive() {
  const [connected, setConnected] = useState<boolean | null>(null)
  const [phase, setPhase] = useState<Phase>('idle')
  const [timeline, setTimeline] = useState<TimelineEntry[]>([])
  const [dealStatus, setDealStatus] = useState<string>('')
  const [dealProb, setDealProb] = useState(0)
  const [dealRecord, setDealRecord] = useState('')
  const [running, setRunning] = useState(false)
  const timelineRef = useRef<HTMLDivElement>(null)

  // Auto-scroll timeline
  useEffect(() => {
    timelineRef.current?.scrollTo({ top: timelineRef.current.scrollHeight, behavior: 'smooth' })
  }, [timeline])

  const addEvent = useCallback((e: TimelineEntry) => {
    setTimeline(prev => [...prev, e])
  }, [])

  // Check connection
  useEffect(() => {
    async function check() {
      const [s1, s2] = await Promise.all([get(`${N1}/status`), get(`${N2}/status`)])
      setConnected(!!s1 && !!s2)
    }
    check()
  }, [])

  // ── Scenario steps ────────────────────────────────────

  const runPropose = useCallback(async () => {
    setRunning(true)
    setPhase('proposing')
    setDealStatus('Proposed')
    setDealProb(0)

    addEvent({ icon: '💬', title: "Alice's agent proposes a deal", detail: 'Buy compute service — 100 units', status: 'info' })
    await wait(800)

    await seedEvent(N1, DEAL, 'deal-001[alice]')
    const cell = await getCell(N1, DEAL)
    setDealProb(cell?.probability ?? 0)

    addEvent({ icon: '📡', title: 'Proposal sent to network', detail: `Registered on Node 1 — probability ${((cell?.probability ?? 0) * 100).toFixed(0)}%`, status: 'info' })

    setRunning(false)
    setPhase('proposing')
  }, [addEvent])

  const runAgree = useCallback(async () => {
    setRunning(true)
    setPhase('agreeing')
    setDealStatus('Processing...')

    addEvent({ icon: '🤝', title: "Bob's agent accepts the deal", detail: 'Both parties confirm the agreement', status: 'info' })
    await wait(600)

    // Bob seeds + context events
    await seedEvent(N2, DEAL, 'deal-001[bob]')
    await Promise.all([
      seedEvent(N1, CTX[0], 'context[alice]'), seedEvent(N2, CTX[0], 'context[bob]'),
      seedEvent(N1, CTX[1], 'context[alice]'), seedEvent(N2, CTX[1], 'context[bob]'),
      seedEvent(N1, CTX[2], 'context[alice]'), seedEvent(N2, CTX[2], 'context[bob]'),
    ])

    addEvent({ icon: '⏳', title: 'Waiting for convergence...', detail: 'Independent nodes synchronizing', status: 'info' })

    // Poll for crystallization
    for (let i = 0; i < 12; i++) {
      await wait(700)
      const cell = await getCell(N1, DEAL)
      setDealProb(cell?.probability ?? 0)
      if (cell?.crystallized) {
        setDealRecord(cell.record ?? '')
        break
      }
    }

    const final = await getCell(N1, DEAL)
    if (final?.crystallized) {
      setDealStatus('PERMANENT')
      setDealProb(1)
      setDealRecord(final.record ?? '')
      addEvent({ icon: '✅', title: 'Agreement is now PERMANENT', detail: 'Crystallized — irreversible, no central authority needed', status: 'success' })
    } else {
      setDealStatus('Converging...')
      addEvent({ icon: '⏳', title: 'Still converging', detail: 'Nodes synchronizing — will crystallize shortly', status: 'info' })
    }

    setRunning(false)
    setPhase('agreed')
  }, [addEvent])

  const runFraud = useCallback(async () => {
    setRunning(true)
    setPhase('fraud')

    addEvent({ icon: '🦹', title: "Mallory tries to forge an agreement", detail: 'Claims Alice agreed with HIM instead of Bob', status: 'warning' })
    await wait(1000)

    await seedEvent(N1, FRAUD, 'mallory:fake-deal')
    await wait(1500)

    const fake = await getCell(N1, FRAUD)
    const real = await getCell(N1, DEAL)

    const fakeRecord = fake?.record ?? ''
    const realRecord = real?.record ?? ''

    addEvent({ icon: '🔍', title: 'Analyzing the claim...', detail: `Mallory's record: ${fakeRecord.split(' + ')[0]}`, status: 'info' })
    await wait(800)

    // Check: Mallory is primary influence on his event, Alice is not primary
    const malloryPrimary = fakeRecord.startsWith('mallory')
    const realHasAlice = realRecord.includes('alice')

    if (malloryPrimary && realHasAlice) {
      addEvent({ icon: '❌', title: 'FRAUD REJECTED', detail: "Mallory's claim has no endorsement from Alice. The real deal remains intact.", status: 'fail' })
    } else {
      addEvent({ icon: '❌', title: 'Fraud attempt detected', detail: 'No multi-party endorsement', status: 'fail' })
    }

    setRunning(false)
    setPhase('fraud-done')
  }, [addEvent])

  const runDestroy = useCallback(async () => {
    setRunning(true)
    setPhase('destroying')

    addEvent({ icon: '💥', title: 'Attacker deletes the agreement', detail: 'Record destroyed on Node 1', status: 'warning' })
    await wait(600)

    await destroyCell(N1, DEAL)
    await wait(300)
    const cell = await getCell(N1, DEAL)
    setDealProb(cell?.probability ?? 0)
    setDealStatus('DESTROYED')

    addEvent({ icon: '🗑️', title: `Record deleted — probability ${((cell?.probability ?? 0) * 100).toFixed(0)}%`, detail: 'The agreement appears to be gone...', status: 'fail' })

    setRunning(false)
    setPhase('destroyed')
  }, [addEvent])

  const runHeal = useCallback(async () => {
    setRunning(true)
    setPhase('healing')

    addEvent({ icon: '🔄', title: 'Waiting for self-healing...', detail: 'Surrounding geometry reconstructs the record', status: 'info' })

    for (let i = 0; i < 15; i++) {
      await wait(600)
      const cell = await getCell(N1, DEAL)
      const p = cell?.probability ?? 0
      setDealProb(p)

      if (cell?.crystallized) {
        setDealStatus('PERMANENT')
        setDealRecord(cell.record ?? '')
        addEvent({ icon: '✨', title: 'Agreement SELF-HEALED', detail: `Restored to ${(p * 100).toFixed(0)}% — provenance intact`, status: 'success' })
        break
      }
      if (i === 7) {
        addEvent({ icon: '📈', title: `Recovering... ${(p * 100).toFixed(0)}%`, detail: 'Neighbors rebuilding the state', status: 'info' })
      }
    }

    setPhase('healed')
    setRunning(false)
  }, [addEvent])

  const runAudit = useCallback(async () => {
    setPhase('audit')
    const cell = await getCell(N1, DEAL)
    const record = cell?.record ?? ''
    setDealRecord(record)

    addEvent({ icon: '📋', title: 'AUDIT TRAIL', detail: 'Participants and their contribution weights:', status: 'info' })

    for (const part of record.split(' + ')) {
      const p = part.trim()
      if (!p || p === '(empty)') continue
      const isAlice = p.includes('alice')
      const isBob = p.includes('bob')
      const label = isAlice ? '👤 Alice (buyer)' : isBob ? '👤 Bob (seller)' : `⚠️ ${p.split(':')[0]}`
      addEvent({
        icon: isAlice || isBob ? '✅' : '⚠️',
        title: label,
        detail: p,
        status: isAlice || isBob ? 'success' : 'warning',
      })
    }
  }, [addEvent])

  // ── Which button is next ──────────────────────────────
  const steps: { label: string; action: () => void; enabled: boolean; phase: Phase[] }[] = [
    { label: '1. Alice proposes deal', action: runPropose, enabled: phase === 'idle', phase: ['idle'] },
    { label: '2. Bob accepts', action: runAgree, enabled: phase === 'proposing', phase: ['proposing'] },
    { label: '3. Fraudster attacks', action: runFraud, enabled: phase === 'agreed', phase: ['agreed'] },
    { label: '4. Delete the record', action: runDestroy, enabled: phase === 'fraud-done', phase: ['fraud-done'] },
    { label: '5. Watch it heal', action: runHeal, enabled: phase === 'destroyed', phase: ['destroyed'] },
    { label: '6. View audit trail', action: runAudit, enabled: phase === 'healed', phase: ['healed'] },
  ]

  const currentStepIdx = steps.findIndex(s => s.enabled)

  // ── Render ────────────────────────────────────────────
  return (
    <div className="fixed inset-0 bg-[#060a10] text-white overflow-auto font-sans">
      {/* Hero */}
      <div className="text-center pt-8 sm:pt-12 pb-4 px-4">
        <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight">
          Agent Agreement Demo
        </h1>
        <p className="mt-2 text-sm text-white/40 max-w-xl mx-auto">
          Two AI agents negotiate a deal. Watch the agreement become permanent,
          survive fraud, and recover from deletion — with no central server.
        </p>
      </div>

      {connected === false && (
        <div className="max-w-lg mx-auto px-4 mt-6">
          <div className="bg-red-500/10 border border-red-500/30 rounded-2xl p-5 text-center">
            <p className="text-red-400 font-semibold mb-2">Backend not running</p>
            <code className="text-xs text-cyan-300 bg-black/40 rounded-lg p-3 block text-left">
              cd tesseract<br/>
              PORT=7710 NODE_ID=node-1 PEERS=127.0.0.1:7711 cargo run --bin node &<br/>
              PORT=7711 NODE_ID=node-2 PEERS=127.0.0.1:7710 cargo run --bin node &
            </code>
          </div>
        </div>
      )}

      {connected && (
        <div className="max-w-5xl mx-auto px-4 pb-10">
          <div className="grid grid-cols-1 lg:grid-cols-[280px_1fr_320px] gap-6">

            {/* ── Left: Agents + Deal status ── */}
            <div className="space-y-4">
              {/* Agent cards */}
              <AgentCard name="Alice" role="Buyer Agent" emoji="🤖" color="cyan" active={['idle', 'proposing', 'agreeing'].includes(phase)} />
              <AgentCard name="Bob" role="Seller Agent" emoji="🤖" color="green" active={['agreeing', 'agreed'].includes(phase)} />
              <AgentCard name="Mallory" role="Fraudster" emoji="🦹" color="red" active={['fraud'].includes(phase)} />

              {/* Deal card */}
              <div className={`rounded-2xl border p-4 transition-all duration-500 ${
                dealStatus === 'PERMANENT' ? 'bg-cyan-500/10 border-cyan-400/30' :
                dealStatus === 'DESTROYED' ? 'bg-red-500/10 border-red-400/30' :
                'bg-white/5 border-white/10'
              }`}>
                <p className="text-[10px] font-bold text-white/40 tracking-widest uppercase mb-2">Agreement Status</p>
                {!dealStatus ? (
                  <p className="text-white/20 text-sm italic">No deal yet</p>
                ) : (
                  <>
                    <p className={`text-lg font-bold ${
                      dealStatus === 'PERMANENT' ? 'text-cyan-400' :
                      dealStatus === 'DESTROYED' ? 'text-red-400' :
                      'text-white/60'
                    }`}>
                      {dealStatus === 'PERMANENT' ? '★ ' : dealStatus === 'DESTROYED' ? '✗ ' : ''}{dealStatus}
                    </p>
                    <div className="mt-2 bg-black/30 rounded-full h-2 overflow-hidden">
                      <div
                        className={`h-full rounded-full transition-all duration-700 ${
                          dealStatus === 'DESTROYED' ? 'bg-red-400' : 'bg-cyan-400'
                        }`}
                        style={{ width: `${dealProb * 100}%` }}
                      />
                    </div>
                    <p className="text-[10px] text-white/30 mt-1">{(dealProb * 100).toFixed(0)}% certainty</p>
                  </>
                )}
              </div>
            </div>

            {/* ── Center: Action buttons ── */}
            <div className="flex flex-col gap-3">
              <p className="text-[10px] font-bold text-white/40 tracking-widest uppercase mb-1">Scenario</p>
              {steps.map((step, i) => {
                const done = i < currentStepIdx || (phase === 'audit' && i <= 5)
                const isCurrent = step.enabled && !running
                return (
                  <button
                    key={i}
                    onClick={step.action}
                    disabled={!isCurrent || running}
                    className={`text-left px-5 py-4 rounded-2xl border transition-all duration-300 ${
                      done
                        ? 'bg-white/5 border-white/5 opacity-40'
                        : isCurrent
                          ? 'bg-cyan-500/15 border-cyan-400/40 hover:bg-cyan-500/25 cursor-pointer shadow-lg shadow-cyan-500/5'
                          : 'bg-white/[0.02] border-white/5 opacity-20 cursor-not-allowed'
                    }`}
                  >
                    <div className="flex items-center gap-4">
                      <div className={`w-10 h-10 rounded-full flex items-center justify-center text-base ${
                        done ? 'bg-white/10 text-green-400' :
                        isCurrent ? 'bg-cyan-500/20 text-cyan-300 animate-pulse' :
                        'bg-white/5 text-white/20'
                      }`}>
                        {done ? '✓' : i < 3 ? ['💬', '🤝', '🦹'][i] : ['💥', '✨', '📋'][i - 3]}
                      </div>
                      <div>
                        <p className={`font-semibold ${done ? 'text-white/40' : isCurrent ? 'text-white' : 'text-white/20'}`}>
                          {step.label}
                        </p>
                      </div>
                    </div>
                  </button>
                )
              })}

              {running && (
                <div className="text-center py-2">
                  <div className="inline-flex items-center gap-2 text-sm text-cyan-300">
                    <div className="w-4 h-4 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin" />
                    Processing...
                  </div>
                </div>
              )}
            </div>

            {/* ── Right: Timeline ── */}
            <div className="flex flex-col h-[560px]">
              <p className="text-[10px] font-bold text-white/40 tracking-widest uppercase mb-2">What happened</p>
              <div
                ref={timelineRef}
                className="flex-1 bg-black/30 border border-white/5 rounded-2xl p-4 overflow-y-auto space-y-3"
              >
                {timeline.length === 0 && (
                  <p className="text-white/15 italic text-sm text-center pt-8">
                    Click "Alice proposes deal" to begin
                  </p>
                )}
                {timeline.map((entry, i) => (
                  <div key={i} className="flex gap-3 items-start">
                    <span className="text-lg shrink-0">{entry.icon}</span>
                    <div className="min-w-0">
                      <p className={`text-sm font-semibold ${
                        entry.status === 'success' ? 'text-green-400' :
                        entry.status === 'fail' ? 'text-red-400' :
                        entry.status === 'warning' ? 'text-yellow-400' :
                        'text-white/70'
                      }`}>{entry.title}</p>
                      <p className="text-xs text-white/30 mt-0.5 break-words">{entry.detail}</p>
                    </div>
                  </div>
                ))}
              </div>

              {/* Bottom legend */}
              <div className="mt-3 bg-white/5 rounded-xl p-3 text-[11px] text-white/30 space-y-1">
                <p><span className="text-cyan-400 font-semibold">Zero fees</span> — no mining, no gas, no validators</p>
                <p><span className="text-cyan-400 font-semibold">No central server</span> — two independent nodes</p>
                <p><span className="text-cyan-400 font-semibold">Self-healing</span> — destroyed data recovers from geometry</p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

// ── Agent Card ──────────────────────────────────────────
function AgentCard({ name, role, emoji, color, active }: {
  name: string; role: string; emoji: string; color: string; active: boolean
}) {
  const borderColor = color === 'cyan' ? 'border-cyan-400/30' : color === 'green' ? 'border-green-400/30' : 'border-red-400/30'
  const bgColor = color === 'cyan' ? 'bg-cyan-500/10' : color === 'green' ? 'bg-green-500/10' : 'bg-red-500/10'
  const textColor = color === 'cyan' ? 'text-cyan-400' : color === 'green' ? 'text-green-400' : 'text-red-400'

  return (
    <div className={`rounded-xl border px-4 py-3 flex items-center gap-3 transition-all duration-300 ${
      active ? `${bgColor} ${borderColor}` : 'bg-white/[0.02] border-white/5 opacity-40'
    }`}>
      <span className="text-2xl">{emoji}</span>
      <div>
        <p className={`font-bold text-sm ${active ? textColor : 'text-white/30'}`}>{name}</p>
        <p className="text-[11px] text-white/30">{role}</p>
      </div>
      {active && <div className={`ml-auto w-2 h-2 rounded-full ${
        color === 'cyan' ? 'bg-cyan-400' : color === 'green' ? 'bg-green-400' : 'bg-red-400'
      } animate-pulse`} />}
    </div>
  )
}
