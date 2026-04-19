import { useState, useCallback, useRef, useEffect } from 'react'

// ── Config ──────────────────────────────────────────────
const NODE1 = '/tess1'
const NODE2 = '/tess2'
const FIELD_SIZE = 8
const POLL_INTERVAL = 600

// ── Types ───────────────────────────────────────────────
interface CellData {
  probability: number
  crystallized: boolean
  record: string
  support: number
}

interface LogEntry {
  text: string
  color: string
}

// ── API ─────────────────────────────────────────────────
async function apiSeed(node: string, t: number, c: number, eventId: string): Promise<any> {
  const res = await fetch(`${node}/seed`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ t, c, o: 3, v: 3, event_id: eventId }),
  })
  return res.json()
}

async function apiDestroy(node: string, t: number, c: number): Promise<void> {
  await fetch(`${node}/destroy`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ t, c, o: 3, v: 3 }),
  })
}

async function apiGetCell(node: string, t: number, c: number): Promise<CellData> {
  try {
    const res = await fetch(`${node}/cell/${t}/${c}/3/3`)
    return res.json()
  } catch {
    return { probability: 0, crystallized: false, record: '', support: 0 }
  }
}

async function apiGetField(node: string): Promise<CellData[][]> {
  const grid: CellData[][] = Array.from({ length: FIELD_SIZE }, () =>
    Array.from({ length: FIELD_SIZE }, () => ({ probability: 0, crystallized: false, record: '', support: 0 }))
  )
  const promises: Promise<void>[] = []
  for (let t = 0; t < FIELD_SIZE; t++) {
    for (let c = 0; c < FIELD_SIZE; c++) {
      const tt = t, cc = c
      promises.push(apiGetCell(node, tt, cc).then(cell => { grid[tt][cc] = cell }))
    }
  }
  await Promise.all(promises)
  return grid
}

async function apiStatus(node: string): Promise<any> {
  try {
    const res = await fetch(`${node}/status`)
    return res.json()
  } catch {
    return null
  }
}

// ── Cell color ──────────────────────────────────────────
function cellColor(p: number, cryst: boolean): string {
  if (cryst) return 'rgba(34, 211, 238, 0.95)'
  if (p > 0.7) return 'rgba(34, 211, 238, 0.6)'
  if (p > 0.4) return 'rgba(34, 211, 238, 0.35)'
  if (p > 0.15) return 'rgba(34, 211, 238, 0.15)'
  if (p > 0.05) return 'rgba(34, 211, 238, 0.07)'
  return 'rgba(255, 255, 255, 0.03)'
}

function cellGlow(cryst: boolean): string {
  return cryst ? '0 0 10px rgba(34,211,238,0.5)' : 'none'
}

// ── Field Grid ──────────────────────────────────────────
function FieldGrid({ grid, label, onCellClick, onCellRightClick, selectedCell }: {
  grid: CellData[][]
  label: string
  onCellClick: (t: number, c: number) => void
  onCellRightClick: (t: number, c: number) => void
  selectedCell: { t: number; c: number } | null
}) {
  return (
    <div className="flex flex-col items-center gap-2">
      <span className="text-xs font-semibold text-white/50 tracking-wider uppercase">{label}</span>
      <div
        className="grid gap-[2px] select-none"
        style={{ gridTemplateColumns: `repeat(${FIELD_SIZE}, 1fr)` }}
        onContextMenu={e => e.preventDefault()}
      >
        {grid.map((row, t) =>
          row.map((cell, c) => {
            const isSelected = selectedCell?.t === t && selectedCell?.c === c
            return (
              <div
                key={`${t}-${c}`}
                onClick={() => onCellClick(t, c)}
                onContextMenu={e => { e.preventDefault(); onCellRightClick(t, c) }}
                className="w-8 h-8 sm:w-10 sm:h-10 rounded-sm cursor-pointer transition-all duration-300 hover:scale-110 relative"
                style={{
                  backgroundColor: cellColor(cell.probability, cell.crystallized),
                  boxShadow: cellGlow(cell.crystallized),
                  outline: isSelected ? '2px solid rgba(250,204,21,0.8)' : 'none',
                  outlineOffset: '1px',
                }}
                title={`(${t},${c}) p=${(cell.probability * 100).toFixed(0)}% ${cell.crystallized ? '★' : ''}\n${cell.record || '(empty)'}`}
              >
                {cell.crystallized && (
                  <div className="absolute inset-0 flex items-center justify-center text-[8px] font-bold text-black/60">
                    ★
                  </div>
                )}
              </div>
            )
          })
        )}
      </div>
      <div className="flex items-center gap-3 text-[10px] text-white/25 mt-1">
        <span>click = seed</span>
        <span>right-click = destroy</span>
      </div>
    </div>
  )
}

// ── Main ────────────────────────────────────────────────
export default function TesseractPlayground() {
  const [connected, setConnected] = useState<boolean | null>(null)
  const [field1, setField1] = useState<CellData[][]>(() => emptyGrid())
  const [field2, setField2] = useState<CellData[][]>(() => emptyGrid())
  const [agentName, setAgentName] = useState('alice')
  const [activeNode, setActiveNode] = useState<1 | 2>(1)
  const [selectedCell, setSelectedCell] = useState<{ t: number; c: number } | null>(null)
  const [cellInfo, setCellInfo] = useState<CellData | null>(null)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [stats, setStats] = useState({ n1: { active: 0, cryst: 0 }, n2: { active: 0, cryst: 0 } })
  const logRef = useRef<HTMLDivElement>(null)
  const polling = useRef<ReturnType<typeof setInterval> | null>(null)

  function emptyGrid(): CellData[][] {
    return Array.from({ length: FIELD_SIZE }, () =>
      Array.from({ length: FIELD_SIZE }, () => ({ probability: 0, crystallized: false, record: '', support: 0 }))
    )
  }

  const log = useCallback((text: string, color = 'text-white/50') => {
    setLogs(prev => {
      const next = [...prev, { text, color }]
      return next.length > 100 ? next.slice(-80) : next
    })
  }, [])

  // Auto-scroll log
  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight, behavior: 'smooth' })
  }, [logs])

  // Refresh fields
  const refresh = useCallback(async () => {
    try {
      const [f1, f2, s1, s2] = await Promise.all([
        apiGetField(NODE1), apiGetField(NODE2),
        apiStatus(NODE1), apiStatus(NODE2),
      ])
      setField1(f1)
      setField2(f2)
      if (s1 && s2) {
        setStats({
          n1: { active: s1.active_cells ?? 0, cryst: s1.crystallized ?? 0 },
          n2: { active: s2.active_cells ?? 0, cryst: s2.crystallized ?? 0 },
        })
      }
      return true
    } catch {
      return false
    }
  }, [])

  // Check connection + start polling
  useEffect(() => {
    async function init() {
      const ok = await refresh()
      setConnected(ok)
      if (ok && !polling.current) {
        polling.current = setInterval(refresh, POLL_INTERVAL)
      }
    }
    init()
    return () => { if (polling.current) clearInterval(polling.current) }
  }, [refresh])

  // Cell click → seed
  const handleCellClick = useCallback(async (t: number, c: number) => {
    const node = activeNode === 1 ? NODE1 : NODE2
    const eventId = `${agentName}:event@(${t},${c})`
    setSelectedCell({ t, c })

    log(`[${agentName}] seed (${t},${c}) on Node ${activeNode}`, 'text-cyan-400')
    await apiSeed(node, t, c, eventId)
    await refresh()

    const cell = await apiGetCell(node, t, c)
    setCellInfo(cell)
    if (cell.crystallized) {
      log(`  ★ Crystallized! p=${(cell.probability * 100).toFixed(0)}%`, 'text-green-400')
    } else {
      log(`  p=${(cell.probability * 100).toFixed(0)}%`, 'text-white/40')
    }
  }, [agentName, activeNode, log, refresh])

  // Right-click → destroy
  const handleCellRightClick = useCallback(async (t: number, c: number) => {
    const node = activeNode === 1 ? NODE1 : NODE2
    setSelectedCell({ t, c })

    log(`[destroy] (${t},${c}) on Node ${activeNode}`, 'text-red-400')
    await apiDestroy(node, t, c)
    await refresh()

    const cell = await apiGetCell(node, t, c)
    setCellInfo(cell)
    log(`  p=${(cell.probability * 100).toFixed(0)}% — ${cell.crystallized ? 'still crystallized!' : 'destroyed'}`, cell.crystallized ? 'text-yellow-400' : 'text-red-400')
  }, [activeNode, log, refresh])

  // Inspect cell on select
  const handleInspect = useCallback(async (t: number, c: number) => {
    const node = activeNode === 1 ? NODE1 : NODE2
    const cell = await apiGetCell(node, t, c)
    setCellInfo(cell)
    setSelectedCell({ t, c })
  }, [activeNode])

  // ── Render ────────────────────────────────────────────
  return (
    <div className="fixed inset-0 bg-[#060a10] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-white/5">
        <div>
          <h1 className="text-xl font-bold tracking-tight">
            <span className="text-cyan-400">Tesseract</span> Playground
          </h1>
          <p className="text-xs text-white/30">Sandbox — seed events, create agreements, try to break it</p>
        </div>
        <div className="flex items-center gap-3">
          <div className={`w-2 h-2 rounded-full ${connected ? 'bg-green-400' : 'bg-red-400'}`} />
          <span className="text-xs text-white/40">{connected ? '2 nodes online' : 'nodes offline'}</span>
        </div>
      </div>

      {connected === false && (
        <div className="max-w-2xl mx-auto mt-10 px-4">
          <div className="bg-red-500/10 border border-red-500/30 rounded-xl p-5 text-center">
            <p className="text-red-400 font-semibold text-sm mb-3">Start two tesseract nodes:</p>
            <code className="block text-xs text-cyan-300 bg-black/40 rounded-lg p-3 text-left">
              cd tesseract<br/>
              PORT=7710 NODE_ID=node-1 PEERS=127.0.0.1:7711 cargo run --bin node &<br/>
              PORT=7711 NODE_ID=node-2 PEERS=127.0.0.1:7710 cargo run --bin node &
            </code>
          </div>
        </div>
      )}

      {connected && (
        <div className="max-w-7xl mx-auto px-4 py-6">
          {/* Controls bar */}
          <div className="flex flex-wrap items-center gap-4 mb-6 bg-white/5 rounded-xl px-4 py-3">
            {/* Agent name */}
            <div className="flex items-center gap-2">
              <label className="text-xs text-white/40">Agent:</label>
              <input
                type="text"
                value={agentName}
                onChange={e => setAgentName(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, ''))}
                className="bg-black/40 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-cyan-300 w-28 focus:outline-none focus:border-cyan-400/50"
                placeholder="agent name"
              />
            </div>

            {/* Active node */}
            <div className="flex items-center gap-1">
              <label className="text-xs text-white/40 mr-1">Seed on:</label>
              <button
                onClick={() => setActiveNode(1)}
                className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition ${
                  activeNode === 1 ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-400/30' : 'text-white/30 hover:text-white/50'
                }`}
              >
                Node 1
              </button>
              <button
                onClick={() => setActiveNode(2)}
                className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition ${
                  activeNode === 2 ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-400/30' : 'text-white/30 hover:text-white/50'
                }`}
              >
                Node 2
              </button>
            </div>

            {/* Stats */}
            <div className="flex items-center gap-4 ml-auto text-xs text-white/30">
              <span>N1: {stats.n1.cryst} crystallized</span>
              <span>N2: {stats.n2.cryst} crystallized</span>
            </div>
          </div>

          {/* Main grid: fields + inspector + log */}
          <div className="grid grid-cols-1 xl:grid-cols-[1fr_auto_1fr_280px] gap-6 items-start">
            {/* Field 1 */}
            <FieldGrid
              grid={field1}
              label={`Node 1 ${activeNode === 1 ? '(active)' : ''}`}
              onCellClick={activeNode === 1 ? handleCellClick : (t, c) => handleInspect(t, c)}
              onCellRightClick={activeNode === 1 ? handleCellRightClick : () => {}}
              selectedCell={selectedCell}
            />

            {/* Sync indicator */}
            <div className="hidden xl:flex flex-col items-center justify-center gap-2 pt-10">
              <div className="text-white/15 text-2xl">⟷</div>
              <span className="text-[10px] text-white/20">auto-sync</span>
            </div>

            {/* Field 2 */}
            <FieldGrid
              grid={field2}
              label={`Node 2 ${activeNode === 2 ? '(active)' : ''}`}
              onCellClick={activeNode === 2 ? handleCellClick : (t, c) => handleInspect(t, c)}
              onCellRightClick={activeNode === 2 ? handleCellRightClick : () => {}}
              selectedCell={selectedCell}
            />

            {/* Log + inspector */}
            <div className="flex flex-col gap-4">
              {/* Cell inspector */}
              {cellInfo && selectedCell && (
                <div className="bg-black/40 border border-white/10 rounded-xl p-3">
                  <p className="text-[10px] font-bold text-white/40 tracking-widest uppercase mb-2">Cell Inspector</p>
                  <div className="text-xs space-y-1">
                    <p className="text-white/60">Coord: <span className="text-cyan-300">({selectedCell.t}, {selectedCell.c}, 3, 3)</span></p>
                    <p className="text-white/60">Probability: <span className={cellInfo.crystallized ? 'text-cyan-400 font-bold' : 'text-white/80'}>{(cellInfo.probability * 100).toFixed(0)}%</span></p>
                    <p className="text-white/60">Status: {cellInfo.crystallized
                      ? <span className="text-cyan-400 font-bold">★ CRYSTALLIZED</span>
                      : <span className="text-white/40">active</span>
                    }</p>
                    {cellInfo.record && cellInfo.record !== '(empty)' && (
                      <div className="mt-2 pt-2 border-t border-white/5">
                        <p className="text-[10px] text-white/30 mb-1">Influences:</p>
                        {cellInfo.record.split(' + ').map((inf, i) => (
                          <p key={i} className={`text-[11px] ${inf.includes(agentName) ? 'text-cyan-400' : 'text-white/40'}`}>
                            {inf.trim()}
                          </p>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* Log */}
              <div className="flex flex-col h-64 xl:h-80">
                <p className="text-[10px] font-bold text-white/30 tracking-widest uppercase mb-1">Log</p>
                <div
                  ref={logRef}
                  className="flex-1 bg-black/40 border border-white/10 rounded-xl p-2 overflow-y-auto font-mono text-[11px] space-y-0.5"
                >
                  {logs.length === 0 && (
                    <p className="text-white/15 italic">Click any cell to begin...</p>
                  )}
                  {logs.map((entry, i) => (
                    <p key={i} className={entry.color}>{entry.text}</p>
                  ))}
                </div>
              </div>

              {/* How to use */}
              <div className="bg-white/5 rounded-xl p-3 text-[11px] text-white/30 space-y-1">
                <p className="font-semibold text-white/50">How to use:</p>
                <p>1. Type an agent name (e.g. "alice")</p>
                <p>2. Click a cell to seed an event</p>
                <p>3. Switch agent + node, click same cell = agreement</p>
                <p>4. Right-click a ★ cell to destroy it — watch it heal</p>
                <p>5. Seed alone on one node — see it won't get endorsement</p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
