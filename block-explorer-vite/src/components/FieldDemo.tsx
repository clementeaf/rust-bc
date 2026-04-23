import { useState, useCallback, useEffect, useRef } from 'react'

const SIZE = 10
const THRESHOLD = 0.85
const INFLUENCE = 0.15

type CellState = { p: number; crystal: boolean; destroyed: boolean; fake: boolean }

function makeGrid(): CellState[][] {
  return Array.from({ length: SIZE }, () =>
    Array.from({ length: SIZE }, () => ({ p: 0, crystal: false, destroyed: false, fake: false }))
  )
}

function dist(r1: number, c1: number, r2: number, c2: number): number {
  const dr = Math.min(Math.abs(r1 - r2), SIZE - Math.abs(r1 - r2))
  const dc = Math.min(Math.abs(c1 - c2), SIZE - Math.abs(c1 - c2))
  return Math.sqrt(dr * dr + dc * dc)
}

function seedAt(grid: CellState[][], row: number, col: number, fake?: boolean): CellState[][] {
  return grid.map((r, ri) =>
    r.map((cell, ci) => {
      if (cell.crystal) return cell
      const d = dist(ri, ci, row, col)
      const added = 1 / (1 + d)
      const newP = Math.min(1, cell.p + added)
      return {
        ...cell,
        p: newP,
        crystal: newP >= THRESHOLD,
        fake: fake && d < 0.5 ? true : cell.fake,
      }
    })
  )
}

function evolve(grid: CellState[][]): CellState[][] {
  return grid.map((r, ri) =>
    r.map((cell, ci) => {
      if (cell.crystal && !cell.destroyed) return cell
      if (cell.fake) return { ...cell, p: Math.max(0, cell.p * 0.7), crystal: false, fake: cell.p * 0.7 > 0.05 }
      let sum = 0
      let count = 0
      for (let dr = -1; dr <= 1; dr++) {
        for (let dc = -1; dc <= 1; dc++) {
          if (dr === 0 && dc === 0) continue
          const nr = (ri + dr + SIZE) % SIZE
          const nc = (ci + dc + SIZE) % SIZE
          const neighbor = grid[nr][nc]
          if (!neighbor.fake) {
            sum += neighbor.p
            count++
          }
        }
      }
      const avg = count > 0 ? sum / count : 0
      const newP = Math.min(1, cell.p + (avg - cell.p) * INFLUENCE + (avg > 0.5 ? 0.03 : 0))
      return {
        p: newP,
        crystal: newP >= THRESHOLD,
        destroyed: false,
        fake: false,
      }
    })
  )
}

type DemoStep = {
  label: string
  desc: string
  action: (grid: CellState[][]) => CellState[][]
  evolveSteps: number
}

const steps: DemoStep[] = [
  {
    label: '1. Sembrar evento',
    desc: 'Un evento genera probabilidad que se propaga radialmente por el campo.',
    action: (g) => seedAt(g, 4, 4),
    evolveSteps: 3,
  },
  {
    label: '2. Cristalizacion',
    desc: 'Las celdas con probabilidad >= 0.85 cristalizan y se vuelven permanentes.',
    action: (g) => g,
    evolveSteps: 8,
  },
  {
    label: '3. Destruir celda',
    desc: 'Un atacante destruye una celda cristalizada. Observa que pasa...',
    action: (g) => g.map((r, ri) => r.map((c, ci) => ri === 4 && ci === 4 ? { ...c, p: 0, crystal: false, destroyed: true } : c)),
    evolveSteps: 0,
  },
  {
    label: '4. Auto-sanacion',
    desc: 'El campo regenera la celda destruida desde sus vecinos. Sin backup.',
    action: (g) => g,
    evolveSteps: 6,
  },
  {
    label: '5. Dato falso',
    desc: 'Un atacante inyecta un dato falso. Sin soporte orbital, no se propaga.',
    action: (g) => seedAt(g, 1, 8, true),
    evolveSteps: 5,
  },
]

function cellColor(cell: CellState): string {
  if (cell.destroyed) return 'bg-red-400'
  if (cell.fake) return 'bg-red-300'
  if (cell.crystal) return 'bg-main-500'
  if (cell.p > 0.6) return 'bg-main-300'
  if (cell.p > 0.3) return 'bg-main-200'
  if (cell.p > 0.1) return 'bg-main-100'
  return 'bg-neutral-100'
}

export default function FieldDemo() {
  const [grid, setGrid] = useState(makeGrid)
  const [step, setStep] = useState(-1)
  const [evolving, setEvolving] = useState(false)
  const evolveRef = useRef<number>(0)

  const reset = useCallback(() => {
    setGrid(makeGrid())
    setStep(-1)
    setEvolving(false)
    evolveRef.current = 0
  }, [])

  const runStep = useCallback(() => {
    const nextStep = step + 1
    if (nextStep >= steps.length) return
    setStep(nextStep)
    const s = steps[nextStep]
    setGrid(prev => s.action(prev))
    if (s.evolveSteps > 0) {
      setEvolving(true)
      evolveRef.current = s.evolveSteps
    }
  }, [step])

  useEffect(() => {
    if (!evolving || evolveRef.current <= 0) {
      setEvolving(false)
      return
    }
    const timer = setTimeout(() => {
      setGrid(prev => evolve(prev))
      evolveRef.current--
      if (evolveRef.current <= 0) setEvolving(false)
    }, 250)
    return () => clearTimeout(timer)
  }, [evolving, grid])

  const crystals = grid.flat().filter(c => c.crystal).length
  const active = grid.flat().filter(c => c.p > 0.05).length

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between mb-3">
        <div className="flex gap-1.5">
          <button
            onClick={runStep}
            disabled={evolving || step >= steps.length - 1}
            className="px-3 py-1 rounded-lg text-xs font-semibold bg-main-500 text-white hover:bg-main-600 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {step < 0 ? 'Iniciar' : 'Siguiente'}
          </button>
          <button
            onClick={reset}
            className="px-3 py-1 rounded-lg text-xs font-semibold bg-neutral-100 text-neutral-600 hover:bg-neutral-200 transition-colors cursor-pointer"
          >
            Reset
          </button>
        </div>
        <div className="flex gap-3 text-[10px] text-neutral-400">
          <span>Activas: {active}</span>
          <span className="text-main-500 font-semibold">Cristales: {crystals}</span>
        </div>
      </div>

      <div className="flex flex-col items-center gap-px flex-1 justify-center">
        {grid.map((row, ri) => (
          <div key={ri} className="flex gap-px">
            {row.map((cell, ci) => (
              <div
                key={ci}
                className={'w-5 h-5 rounded-sm transition-colors duration-200 ' + cellColor(cell)}
                title={'P: ' + cell.p.toFixed(2)}
              />
            ))}
          </div>
        ))}
      </div>

      <div className="mt-3 min-h-[36px]">
        {step >= 0 && (
          <div>
            <p className="text-neutral-900 text-xs font-semibold">{steps[step].label}</p>
            <p className="text-neutral-500 text-[10px] leading-relaxed">{steps[step].desc}</p>
          </div>
        )}
        {step < 0 && (
          <p className="text-neutral-400 text-[10px]">Presiona &quot;Iniciar&quot; para simular el campo de probabilidad.</p>
        )}
      </div>

      <div className="mt-2 pt-2 border-t border-neutral-100 flex gap-3 text-[9px] text-neutral-300">
        <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-sm bg-main-500 inline-block" /> Cristalizado</span>
        <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-sm bg-main-200 inline-block" /> Probabilidad</span>
        <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-sm bg-red-400 inline-block" /> Destruido</span>
        <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-sm bg-red-300 inline-block" /> Falso</span>
      </div>
    </div>
  )
}
