import { useEffect, useRef, useState } from 'react'

// --- Crystal lattice tesseract: infinite geometry in all directions ---
// You are inside a 4D crystal lattice. Cubes everywhere —
// above, below, diagonal, overlapping, fading into infinity.

function TesseractCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const dpr = window.devicePixelRatio || 1
    let W = window.innerWidth
    let H = window.innerHeight
    function resize() {
      W = window.innerWidth
      H = window.innerHeight
      canvas.width = W * dpr
      canvas.height = H * dpr
      canvas.style.width = `${W}px`
      canvas.style.height = `${H}px`
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    }
    resize()
    window.addEventListener('resize', resize)

    let time = 0
    let frameId: number

    // Cube template: 8 verts, 12 edges
    const cubeV: [number,number,number][] = [
      [-1,-1,-1],[1,-1,-1],[1,1,-1],[-1,1,-1],
      [-1,-1,1],[1,-1,1],[1,1,1],[-1,1,1],
    ]
    const cubeE: [number,number][] = [
      [0,1],[1,2],[2,3],[3,0],
      [4,5],[5,6],[6,7],[7,4],
      [0,4],[1,5],[2,6],[3,7],
    ]

    // Grid of cubes: a 5×5×5 lattice centered on the camera
    const GRID = 5
    const SPACING = 2.4
    const HALF = Math.floor(GRID / 2)

    // Camera position drifts slowly through the lattice
    let camX = 0, camY = 0, camZ = 0

    // Project 3D world → 2D screen
    function proj(wx: number, wy: number, wz: number): [number, number, number] | null {
      // Camera-relative position
      let rx = wx - camX
      let ry = wy - camY
      let rz = wz - camZ

      // Slow world rotation (gives the 4D feeling)
      const a = time * 0.15
      const b = time * 0.1
      // Rotate XZ
      const rx2 = rx * Math.cos(a) - rz * Math.sin(a)
      const rz2 = rx * Math.sin(a) + rz * Math.cos(a)
      rx = rx2; rz = rz2
      // Rotate YZ
      const ry2 = ry * Math.cos(b) - rz * Math.sin(b)
      const rz3 = ry * Math.sin(b) + rz * Math.cos(b)
      ry = ry2; rz = rz3

      // Behind camera
      if (rz < 0.5) return null

      const fov = 1.8
      const scale = fov / rz
      return [
        W / 2 + rx * scale * Math.min(W, H) * 0.3,
        H / 2 + ry * scale * Math.min(W, H) * 0.3,
        scale,
      ]
    }

    // Pulses: events happening at cube centers
    interface Pulse {
      gx: number; gy: number; gz: number
      born: number
    }
    const pulses: Pulse[] = []

    function draw() {
      ctx.fillStyle = '#040608'
      ctx.fillRect(0, 0, W, H)

      // Drift camera gently
      camX = Math.sin(time * 0.08) * 1.5
      camY = Math.sin(time * 0.06 + 1) * 1.0
      camZ = time * 0.3

      // Spawn pulse
      if (Math.random() < 0.025) {
        pulses.push({
          gx: (Math.floor(Math.random() * GRID) - HALF) * SPACING + camX,
          gy: (Math.floor(Math.random() * GRID) - HALF) * SPACING + camY,
          gz: (Math.floor(Math.random() * GRID) - HALF) * SPACING + camZ + SPACING * 2,
          born: time,
        })
      }
      while (pulses.length > 12) pulses.shift()

      // Collect all edges to draw, sorted by depth
      interface DrawEdge {
        x1: number; y1: number; x2: number; y2: number
        depth: number; alpha: number
      }
      const drawList: DrawEdge[] = []

      for (let gx = -HALF; gx <= HALF; gx++) {
        for (let gy = -HALF; gy <= HALF; gy++) {
          for (let gz = -HALF; gz <= HALF; gz++) {
            const ox = gx * SPACING
            const oy = gy * SPACING
            const oz = gz * SPACING + camZ

            // Distance from camera for LOD / fade
            const dx = ox - camX
            const dy = oy - camY
            const dz = oz - camZ
            const dist = Math.sqrt(dx*dx + dy*dy + dz*dz)
            if (dist > GRID * SPACING) continue

            const distFade = Math.max(0, 1 - dist / (GRID * SPACING * 0.8))

            // Project all 8 verts
            const projected = cubeV.map(([vx,vy,vz]) =>
              proj(ox + vx * 0.45, oy + vy * 0.45, oz + vz * 0.45)
            )

            // Draw edges
            for (const [i, j] of cubeE) {
              const p1 = projected[i]
              const p2 = projected[j]
              if (!p1 || !p2) continue

              drawList.push({
                x1: p1[0], y1: p1[1],
                x2: p2[0], y2: p2[1],
                depth: (p1[2] + p2[2]) / 2,
                alpha: distFade,
              })
            }
          }
        }
      }

      // Sort back to front
      drawList.sort((a, b) => a.depth - b.depth)

      // Draw edges
      for (const edge of drawList) {
        const brightness = edge.alpha * Math.min(edge.depth * 0.5, 0.7)
        if (brightness < 0.01) continue

        ctx.beginPath()
        ctx.moveTo(edge.x1, edge.y1)
        ctx.lineTo(edge.x2, edge.y2)
        ctx.strokeStyle = `rgba(80, 150, 220, ${brightness})`
        ctx.lineWidth = Math.max(edge.depth * 0.5, 0.3)
        ctx.lineCap = 'round'
        ctx.stroke()
      }

      // Draw pulses
      for (const pulse of pulses) {
        const age = time - pulse.born
        if (age > 3) continue
        const p = proj(pulse.gx, pulse.gy, pulse.gz)
        if (!p) continue
        const [px, py, ps] = p
        const radius = age * ps * Math.min(W, H) * 0.04
        const fade = Math.max(0, 1 - age / 3)

        // Ring
        ctx.beginPath()
        ctx.arc(px, py, radius, 0, Math.PI * 2)
        ctx.strokeStyle = `rgba(140, 210, 255, ${0.25 * fade * ps})`
        ctx.lineWidth = ps * 1.2
        ctx.stroke()

        // Flash
        if (age < 0.4) {
          const f = 1 - age / 0.4
          const g = ctx.createRadialGradient(px, py, 0, px, py, ps * 15 * f)
          g.addColorStop(0, `rgba(200, 235, 255, ${0.3 * f})`)
          g.addColorStop(1, 'rgba(200, 235, 255, 0)')
          ctx.beginPath()
          ctx.arc(px, py, ps * 15 * f, 0, Math.PI * 2)
          ctx.fillStyle = g
          ctx.fill()
        }
      }

      time += 0.008
      frameId = requestAnimationFrame(draw)
    }

    ctx.fillStyle = '#040608'
    ctx.fillRect(0, 0, W, H)
    draw()

    return () => {
      cancelAnimationFrame(frameId)
      window.removeEventListener('resize', resize)
    }
  }, [])

  return <canvas ref={canvasRef} className="fixed inset-0 w-full h-full" />
}

const rules = [
  { number: '01', title: 'El espacio es la verdad', desc: 'Los acuerdos deforman un espacio 4D. La deformacion es el registro.' },
  { number: '02', title: 'Nadie valida', desc: 'El estado cristaliza cuando las probabilidades convergen.' },
  { number: '03', title: 'Lo falso no se sostiene', desc: 'Sin soporte ortogonal no hay cristalizacion.' },
  { number: '04', title: 'Lo verdadero no se destruye', desc: 'Los orbitales vecinos reconstituyen lo destruido.' },
  { number: '05', title: 'Escasez geometrica', desc: 'Capacidad finita de curvatura. Fisica, no reglas.' },
  { number: '06', title: 'Post-computacional', desc: 'Cero criptografia. Seguridad por geometria.' },
]

const tabs = [
  {
    id: 'transaction',
    label: 'Transaccion',
    content: {
      title: 'Como se genera una transaccion',
      steps: [
        {
          icon: '1',
          head: 'Dos personas acuerdan',
          body: 'Alice quiere enviar 10 tokens a Bob. Ambos deciden que el trato es real. Ese acuerdo es un evento.',
        },
        {
          icon: '2',
          head: 'El espacio se deforma',
          body: 'El evento no se "registra" en una base de datos. Dobla el espacio 4D en la region de Alice. Como una huella en arena mojada.',
        },
        {
          icon: '3',
          head: 'El orbital se expande',
          body: 'La deformacion se esparce como una onda. Cada celda del espacio recibe probabilidad proporcional a su distancia al evento.',
        },
        {
          icon: '4',
          head: 'La convergencia cristaliza',
          body: 'Cuando suficiente probabilidad converge desde direcciones independientes, el estado se vuelve permanente. Nadie lo aprueba. Es inevitable.',
        },
        {
          icon: '5',
          head: 'Bob recibe curvatura',
          body: 'La region de Alice pierde capacidad geometrica. La de Bob la gana. No es dinero moviéndose — es el espacio redistribuyendo su capacidad de doblarse.',
        },
      ],
    },
  },
  {
    id: 'rules',
    label: 'Reglas',
    content: null, // uses the rules array
  },
  {
    id: 'evolution',
    label: 'Evolucion',
    content: null, // paradigm comparison
  },
]

export default function Tesseract() {
  const [activeTab, setActiveTab] = useState('transaction')

  return (
    <div className="fixed inset-0 z-50 overflow-hidden bg-[#040608]">
      <TesseractCanvas />
      <div className="absolute inset-0 z-10 flex flex-col justify-between">
        {/* Title */}
        <div className="pt-10 sm:pt-14 text-center px-6 pointer-events-none">
          <h1 className="text-5xl sm:text-7xl lg:text-8xl font-extrabold tracking-tighter text-white leading-none drop-shadow-[0_0_40px_rgba(80,150,220,0.3)]">
            Tesseract
          </h1>
          <p className="mt-4 text-sm sm:text-base text-white/50 font-medium max-w-md mx-auto leading-relaxed">
            Consenso por convergencia geometrica en espacio 4D
          </p>
        </div>

        {/* Bottom panel with tabs */}
        <div className="pb-6 sm:pb-8 px-4 sm:px-8">
          {/* Tab buttons */}
          <div className="max-w-3xl mx-auto flex items-center justify-center gap-1 mb-3">
            {tabs.map(tab => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`px-4 py-1.5 rounded-full text-xs font-semibold transition-all duration-200 ${
                  activeTab === tab.id
                    ? 'bg-main-500/20 text-main-300 border border-main-400/30'
                    : 'text-white/30 hover:text-white/50'
                }`}
              >
                {tab.label}
              </button>
            ))}
          </div>

          {/* Tab content */}
          <div className="max-w-4xl mx-auto backdrop-blur-sm bg-black/40 rounded-2xl p-5 pointer-events-none">
            {activeTab === 'transaction' && (
              <div>
                <p className="text-xs font-bold text-main-400 tracking-widest mb-4">
                  {tabs[0].content!.title.toUpperCase()}
                </p>
                <div className="grid grid-cols-1 sm:grid-cols-5 gap-4">
                  {tabs[0].content!.steps.map(step => (
                    <div key={step.icon}>
                      <div className="w-6 h-6 rounded-full bg-main-500/20 border border-main-400/30 flex items-center justify-center mb-2">
                        <span className="text-[10px] font-bold text-main-300">{step.icon}</span>
                      </div>
                      <p className="text-xs font-bold text-white/80 leading-tight">{step.head}</p>
                      <p className="text-[11px] text-white/35 mt-1 leading-snug">{step.body}</p>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {activeTab === 'rules' && (
              <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-x-6 gap-y-4">
                {rules.map(r => (
                  <div key={r.number}>
                    <span className="text-[10px] font-bold text-main-400 tracking-widest">{r.number}</span>
                    <p className="text-xs font-bold text-white/80 mt-1 leading-tight">{r.title}</p>
                    <p className="text-[11px] text-white/40 mt-1 leading-snug">{r.desc}</p>
                  </div>
                ))}
              </div>
            )}

            {activeTab === 'evolution' && (
              <div className="flex flex-col items-center gap-6 py-2">
                <p className="text-xs font-bold text-main-400 tracking-widest">EVOLUCION DEL CONSENSO DISTRIBUIDO</p>
                <div className="flex items-center gap-6 sm:gap-10">
                  <div className="text-center">
                    <p className="text-white/70 font-bold text-sm">Confianza</p>
                    <p className="text-white/30 text-[11px] mt-1">Pre-2009</p>
                    <p className="text-white/20 text-[10px] italic mt-0.5">Alguien custodia tu verdad</p>
                  </div>
                  <span className="text-white/20 text-lg">→</span>
                  <div className="text-center">
                    <p className="text-white/70 font-bold text-sm">Verificacion</p>
                    <p className="text-white/30 text-[11px] mt-1">Bitcoin, 2009</p>
                    <p className="text-white/20 text-[10px] italic mt-0.5">Tu verificas la verdad</p>
                  </div>
                  <span className="text-white/20 text-lg">→</span>
                  <div className="text-center bg-main-500/10 border border-main-400/20 rounded-xl px-4 py-2">
                    <p className="text-main-300 font-bold text-sm">Convergencia</p>
                    <p className="text-main-400/60 text-[11px] mt-1">Tesseract</p>
                    <p className="text-main-400/40 text-[10px] italic mt-0.5">La verdad existe o no existe</p>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
