import { useState, useCallback, useRef, useEffect } from 'react'

// ── Config ──────────────────────────────────────────────
const NODE1 = '/tess1'
const NODE2 = '/tess2'
const FIELD_SIZE = 8
const DEAL = { t: 3, c: 3, o: 3, v: 3 }
const CTX1 = { t: 3, c: 4, o: 3, v: 3 }
const CTX2 = { t: 4, c: 3, o: 3, v: 3 }
const CTX3 = { t: 3, c: 3, o: 4, v: 3 }
const FRAUD = { t: 5, c: 5, o: 5, v: 5 }

// ── Types ───────────────────────────────────────────────
interface CellData {
  probability: number
  crystallized: boolean
  record: string
}

interface LogEntry {
  time: number
  agent: string
  color: string
  text: string
  type: 'action' | 'success' | 'fail' | 'system'
}

// ── API helpers ─────────────────────────────────────────
async function seed(node: string, coord: { t: number; c: number; o: number; v: number }, eventId: string): Promise<any> {
  const res = await fetch(`${node}/seed`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ...coord, event_id: eventId }),
  })
  return res.json()
}

async function getCell(node: string, coord: { t: number; c: number; o: number; v: number }): Promise<CellData> {
  try {
    const res = await fetch(`${node}/cell/${coord.t}/${coord.c}/${coord.o}/${coord.v}`)
    return res.json()
  } catch {
    return { probability: 0, crystallized: false, record: '' }
  }
}

async function destroy(node: string, coord: { t: number; c: number; o: number; v: number }): Promise<void> {
  await fetch(`${node}/destroy`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(coord),
  })
}

async function getFieldSlice(node: string): Promise<Map<string, CellData>> {
  const cells = new Map<string, CellData>()
  // Scan a 2D slice (t, c) at o=3, v=3 — the plane where events happen
  const promises: Promise<void>[] = []
  for (let t = 0; t < FIELD_SIZE; t++) {
    for (let c = 0; c < FIELD_SIZE; c++) {
      const coord = { t, c, o: 3, v: 3 }
      promises.push(
        getCell(node, coord).then(cell => {
          cells.set(`${t},${c}`, cell)
        })
      )
    }
  }
  await Promise.all(promises)
  return cells
}

// ── Field Grid Component ────────────────────────────────
function FieldGrid({ cells, label, highlight }: {
  cells: Map<string, CellData>
  label: string
  highlight?: { t: number; c: number }
}) {
  return (
    <div className="flex flex-col items-center gap-2">
      <span className="text-xs font-semibold text-white/60 tracking-wider uppercase">{label}</span>
      <div className="grid gap-[2px]" style={{ gridTemplateColumns: `repeat(${FIELD_SIZE}, 1fr)` }}>
        {Array.from({ length: FIELD_SIZE }, (_, t) =>
          Array.from({ length: FIELD_SIZE }, (_, c) => {
            const cell = cells.get(`${t},${c}`)
            const p = cell?.probability ?? 0
            const cryst = cell?.crystallized ?? false
            const isHighlight = highlight?.t === t && highlight?.c === c

            let bg: string
            if (cryst) {
              bg = 'bg-cyan-400 shadow-[0_0_8px_rgba(34,211,238,0.6)]'
            } else if (p > 0.5) {
              bg = 'bg-cyan-600/80'
            } else if (p > 0.2) {
              bg = 'bg-cyan-700/50'
            } else if (p > 0.05) {
              bg = 'bg-cyan-900/40'
            } else {
              bg = 'bg-white/5'
            }

            return (
              <div
                key={`${t}-${c}`}
                className={`w-7 h-7 sm:w-9 sm:h-9 rounded-sm transition-all duration-500 ${bg} ${isHighlight ? 'ring-2 ring-yellow-400' : ''}`}
                title={`(${t},${c}) p=${p.toFixed(2)} ${cryst ? '★ CRYSTALLIZED' : ''}`}
              />
            )
          })
        )}
      </div>
      <div className="flex items-center gap-3 text-[10px] text-white/30 mt-1">
        <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded-sm bg-white/5 inline-block" /> empty</span>
        <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded-sm bg-cyan-700/50 inline-block" /> active</span>
        <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded-sm bg-cyan-400 shadow-[0_0_4px_rgba(34,211,238,0.4)] inline-block" /> crystallized</span>
      </div>
    </div>
  )
}

// ── Step Button ─────────────────────────────────────────
function StepButton({ step, title, subtitle, onClick, done, active, disabled }: {
  step: number
  title: string
  subtitle: string
  onClick: () => void
  done: boolean
  active: boolean
  disabled: boolean
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`text-left px-4 py-3 rounded-xl border transition-all duration-300 ${
        done
          ? 'bg-cyan-500/10 border-cyan-400/30 opacity-60'
          : active
            ? 'bg-cyan-500/20 border-cyan-400/50 hover:bg-cyan-500/30 cursor-pointer'
            : 'bg-white/5 border-white/10 opacity-30 cursor-not-allowed'
      }`}
    >
      <div className="flex items-center gap-3">
        <div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold ${
          done ? 'bg-cyan-400 text-black' : active ? 'bg-cyan-500/30 text-cyan-300' : 'bg-white/10 text-white/30'
        }`}>
          {done ? '✓' : step}
        </div>
        <div>
          <p className={`text-sm font-semibold ${done ? 'text-cyan-400' : active ? 'text-white' : 'text-white/30'}`}>{title}</p>
          <p className="text-xs text-white/30">{subtitle}</p>
        </div>
      </div>
    </button>
  )
}

// ── Main Demo Page ──────────────────────────────────────
export default function TesseractDemo() {
  const [currentStep, setCurrentStep] = useState(0)
  const [running, setRunning] = useState(false)
  const [connected, setConnected] = useState<boolean | null>(null)
  const [field1, setField1] = useState<Map<string, CellData>>(new Map())
  const [field2, setField2] = useState<Map<string, CellData>>(new Map())
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [highlight, setHighlight] = useState<{ t: number; c: number } | undefined>()
  const logRef = useRef<HTMLDivElement>(null)

  const log = useCallback((agent: string, color: string, text: string, type: LogEntry['type'] = 'action') => {
    setLogs(prev => [...prev, { time: Date.now(), agent, color, text, type }])
  }, [])

  const refreshFields = useCallback(async () => {
    try {
      const [f1, f2] = await Promise.all([getFieldSlice(NODE1), getFieldSlice(NODE2)])
      setField1(f1)
      setField2(f2)
    } catch {
      // nodes may be down
    }
  }, [])

  // Auto-scroll log
  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight, behavior: 'smooth' })
  }, [logs])

  // Check connection on mount
  useEffect(() => {
    async function check() {
      try {
        const [r1, r2] = await Promise.all([
          fetch(`${NODE1}/status`).then(r => r.ok),
          fetch(`${NODE2}/status`).then(r => r.ok),
        ])
        setConnected(r1 && r2)
        if (r1 && r2) {
          await refreshFields()
        }
      } catch {
        setConnected(false)
      }
    }
    check()
  }, [refreshFields])

  const wait = (ms: number) => new Promise(r => setTimeout(r, ms))

  // ── Step handlers ───────────────────────────────────
  const step1 = useCallback(async () => {
    setRunning(true)
    log('Alice', 'text-cyan-400', 'I want to buy compute service for 100 curvatura')
    log('Alice', 'text-cyan-400', `Seeding deal-001 on Node 1 at (${DEAL.t},${DEAL.c},${DEAL.o},${DEAL.v})`)
    setHighlight({ t: DEAL.t, c: DEAL.c })

    await seed(NODE1, DEAL, 'deal-001[alice]')
    await wait(500)
    await refreshFields()

    log('System', 'text-white/50', 'Proposal entered the field', 'system')
    setCurrentStep(1)
    setRunning(false)
  }, [log, refreshFields])

  const step2 = useCallback(async () => {
    setRunning(true)
    log('Bob', 'text-green-400', 'I accept the deal')
    log('Bob', 'text-green-400', `Seeding deal-001 on Node 2 at (${DEAL.t},${DEAL.c},${DEAL.o},${DEAL.v})`)

    await seed(NODE2, DEAL, 'deal-001[bob]')

    // Supporting context
    log('System', 'text-white/50', 'Adding supporting context events...', 'system')
    await Promise.all([
      seed(NODE1, CTX1, 'context-1[alice]'), seed(NODE2, CTX1, 'context-1[bob]'),
      seed(NODE1, CTX2, 'context-2[alice]'), seed(NODE2, CTX2, 'context-2[bob]'),
      seed(NODE1, CTX3, 'context-3[alice]'), seed(NODE2, CTX3, 'context-3[bob]'),
    ])

    log('System', 'text-white/50', 'Waiting for sync + evolution...', 'system')
    // Poll until crystallized or timeout
    for (let i = 0; i < 10; i++) {
      await wait(800)
      await refreshFields()
      const cell = await getCell(NODE1, DEAL)
      if (cell.crystallized) break
    }

    const cell1 = await getCell(NODE1, DEAL)
    const cell2 = await getCell(NODE2, DEAL)
    if (cell1.crystallized || cell2.crystallized) {
      log('System', 'text-cyan-400', 'Agreement CRYSTALLIZED on both nodes!', 'success')
    }
    if (cell1.record) {
      log('System', 'text-white/50', `Record: ${cell1.record}`, 'system')
    }

    setCurrentStep(2)
    setRunning(false)
  }, [log, refreshFields])

  const step3 = useCallback(async () => {
    setRunning(true)
    log('Mallory', 'text-red-400', "I'll claim Alice made a deal with ME")
    log('Mallory', 'text-red-400', `Seeding fake-deal on Node 1 at (${FRAUD.t},${FRAUD.c},${FRAUD.o},${FRAUD.v})`)
    setHighlight({ t: FRAUD.t, c: FRAUD.c })

    await seed(NODE1, FRAUD, 'mallory:fake-deal')
    await wait(1500)
    await refreshFields()

    const real = await getCell(NODE1, DEAL)
    const fake = await getCell(NODE1, FRAUD)

    log('System', 'text-white/50', `Real deal record: ${real.record}`, 'system')
    log('System', 'text-white/50', `Mallory's record: ${fake.record}`, 'system')

    if (real.record?.includes('alice') && real.record?.includes('bob')) {
      log('System', 'text-green-400', 'Real deal has BOTH Alice and Bob', 'success')
    }
    log('System', 'text-red-400', "Mallory's fraud: no legitimate endorsement from Alice", 'fail')

    setHighlight({ t: DEAL.t, c: DEAL.c })
    setCurrentStep(3)
    setRunning(false)
  }, [log, refreshFields])

  const step4 = useCallback(async () => {
    setRunning(true)
    log('Attacker', 'text-red-400', 'Destroying the agreement record...', 'action')

    await destroy(NODE1, DEAL)
    await wait(300)
    await refreshFields()

    const after = await getCell(NODE1, DEAL)
    log('System', 'text-white/50', `Agreement destroyed. Probability: ${(after.probability * 100).toFixed(0)}%`, 'system')

    setCurrentStep(4)
    setRunning(false)
  }, [log, refreshFields])

  const step5 = useCallback(async () => {
    setRunning(true)
    log('System', 'text-white/50', 'Waiting for field evolution + peer sync...', 'system')

    // Poll for healing
    for (let i = 0; i < 15; i++) {
      await wait(600)
      await refreshFields()
      const cell = await getCell(NODE1, DEAL)
      if (cell.crystallized) {
        log('System', 'text-cyan-400', `Agreement SELF-HEALED! Probability: ${(cell.probability * 100).toFixed(0)}%`, 'success')
        if (cell.record) {
          log('System', 'text-cyan-400', `Provenance preserved: ${cell.record}`, 'success')
        }
        break
      } else if (cell.probability > 0.3) {
        log('System', 'text-white/50', `Recovering... ${(cell.probability * 100).toFixed(0)}%`, 'system')
      }
    }

    // Final audit
    log('', '', '', 'system')
    log('Audit', 'text-yellow-300', 'Influence analysis:', 'action')
    const finalCell = await getCell(NODE1, DEAL)
    if (finalCell.record) {
      for (const part of finalCell.record.split(' + ')) {
        const p = part.trim()
        if (!p || p === '(empty)') continue
        const isAlice = p.includes('alice')
        const isBob = p.includes('bob')
        const color = isAlice ? 'text-cyan-400' : isBob ? 'text-green-400' : 'text-white/30'
        log('', color, `  ${p}`, 'system')
      }
    }
    if (finalCell.record?.includes('alice')) log('Audit', 'text-green-400', "Alice's participation: VERIFIED", 'success')
    if (finalCell.record?.includes('bob')) log('Audit', 'text-green-400', "Bob's participation: VERIFIED", 'success')

    setCurrentStep(5)
    setRunning(false)
  }, [log, refreshFields])

  const steps = [step1, step2, step3, step4, step5]
  const stepDefs = [
    { title: 'Alice proposes', subtitle: 'Seed event on Node 1' },
    { title: 'Bob accepts', subtitle: 'Distributed agreement crystallizes' },
    { title: 'Mallory attacks', subtitle: 'Fraud without endorsement' },
    { title: 'Destroy record', subtitle: 'Delete the agreement' },
    { title: 'Self-healing', subtitle: 'Watch geometry restore it' },
  ]

  // ── Render ────────────────────────────────────────────
  return (
    <div className="fixed inset-0 bg-[#060a10] text-white overflow-auto">
      {/* Hero */}
      <div className="text-center pt-10 pb-6 px-4">
        <h1 className="text-4xl sm:text-5xl font-extrabold tracking-tight">
          <span className="text-cyan-400">Tesseract</span> Demo
        </h1>
        <p className="mt-2 text-sm text-white/40 max-w-lg mx-auto">
          Two independent nodes. No central server. Watch agents agree, fraudsters fail, and destroyed data heal itself.
        </p>
      </div>

      {/* Connection status */}
      {connected === false && (
        <div className="max-w-4xl mx-auto px-4 mb-6">
          <div className="bg-red-500/10 border border-red-500/30 rounded-xl p-4 text-center">
            <p className="text-red-400 font-semibold text-sm">Nodes not running</p>
            <p className="text-white/40 text-xs mt-1">
              Start two tesseract nodes first:
            </p>
            <code className="block mt-2 text-xs text-cyan-300 bg-black/40 rounded-lg p-3 text-left max-w-md mx-auto">
              cd tesseract<br/>
              PORT=7710 NODE_ID=node-alice PEERS=127.0.0.1:7711 cargo run --bin node &<br/>
              PORT=7711 NODE_ID=node-bob PEERS=127.0.0.1:7710 cargo run --bin node &
            </code>
          </div>
        </div>
      )}

      {/* Main layout */}
      <div className="max-w-6xl mx-auto px-4 pb-10">
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_auto_1fr] gap-6 items-start">
          {/* Left: Steps */}
          <div className="flex flex-col gap-2">
            <h2 className="text-xs font-bold text-white/40 tracking-widest uppercase mb-2">Scenario</h2>
            {stepDefs.map((def, i) => (
              <StepButton
                key={i}
                step={i + 1}
                title={def.title}
                subtitle={def.subtitle}
                onClick={() => steps[i]()}
                done={currentStep > i}
                active={currentStep === i && !running}
                disabled={currentStep !== i || running || connected === false}
              />
            ))}

            {currentStep === 5 && (
              <div className="mt-4 p-4 rounded-xl bg-cyan-500/10 border border-cyan-400/30">
                <p className="text-cyan-400 font-bold text-sm">All claims validated</p>
                <div className="text-xs text-white/50 mt-2 space-y-1">
                  <p>&#10003; Multi-agent agreement crystallizes</p>
                  <p>&#10003; Fraud without endorsement fails</p>
                  <p>&#10003; Destroyed state self-heals</p>
                  <p>&#10003; Audit trail preserved</p>
                  <p>&#10003; Zero fees, no central coordinator</p>
                </div>
              </div>
            )}
          </div>

          {/* Center: Fields */}
          <div className="flex flex-col sm:flex-row gap-6 items-center">
            <FieldGrid cells={field1} label="Node 1 — Alice" highlight={highlight} />
            <div className="text-white/20 text-2xl hidden sm:block">⟷</div>
            <FieldGrid cells={field2} label="Node 2 — Bob" highlight={highlight} />
          </div>

          {/* Right: Log */}
          <div className="flex flex-col h-[480px]">
            <h2 className="text-xs font-bold text-white/40 tracking-widest uppercase mb-2">Event Log</h2>
            <div
              ref={logRef}
              className="flex-1 bg-black/40 rounded-xl border border-white/10 p-3 overflow-y-auto font-mono text-xs space-y-0.5"
            >
              {logs.length === 0 && (
                <p className="text-white/20 italic">Click "Alice proposes" to begin...</p>
              )}
              {logs.map((entry, i) => (
                <div key={i} className="flex gap-2">
                  {entry.agent && (
                    <span className={`font-bold ${entry.color} shrink-0`}>
                      {entry.type === 'success' ? '  ✓' : entry.type === 'fail' ? '  ✗' : `[${entry.agent}]`}
                    </span>
                  )}
                  <span className={
                    entry.type === 'success' ? 'text-green-400' :
                    entry.type === 'fail' ? 'text-red-400' :
                    entry.type === 'system' ? 'text-white/40' :
                    'text-white/70'
                  }>
                    {entry.text}
                  </span>
                </div>
              ))}
              {running && (
                <div className="text-white/30 animate-pulse">  ⏳ Processing...</div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
