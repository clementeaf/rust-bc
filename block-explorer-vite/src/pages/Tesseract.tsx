import { useState, useEffect, lazy, Suspense } from 'react'
import { Link } from 'react-router-dom'

const FieldDemo = lazy(() => import('../components/FieldDemo'))

const concepts = [
  {
    label: 'Campo 4D',
    desc: 'Los datos no se almacenan \u2014 emergen. Cada evento genera una distribucion de probabilidad en un espacio toroidal de 4 dimensiones simetricas.',
    detail: 'Dimensiones: temporal, contexto, organizacion, version. Ninguna es privilegiada. La probabilidad converge hacia un punto fijo unico sin protocolo de consenso.',
    metric: 'Umbral de cristalizacion: \u03C3 \u2265 0.85 \u2014 convergencia determinista',
  },
  {
    label: 'Auto-sanacion',
    desc: 'Destruir un dato no lo elimina. El campo regenera las celdas destruidas desde la geometria de sus vecinos ortogonales, sin backup ni redundancia.',
    detail: 'Con soporte ortogonal \u03C3 = 4, la recuperacion toma \u2264 9 pasos. Observado en practica: 2-5 pasos. El atacante necesita destruir O(S\u2074) celdas \u2014 el campo completo.',
    metric: 'Costo de ataque exponencial: 10\u2076 celdas en campo 32\u2074',
  },
  {
    label: 'Rechazo de falsedad',
    desc: 'Un dato falso inyectado por fuerza no se propaga. Sin soporte orbital (\u03C3 \u2264 1), no genera resonancia y queda aislado geometricamente.',
    detail: 'La resonancia requiere attestaciones independientes en multiples ejes. Un Sybil con 20 identidades falsas no logra cascada \u2014 los datos reales sobreviven intactos.',
    metric: '7 ataques adversariales probados: Sybil, eclipse, timing, cuantico',
  },
  {
    label: 'Cristalizacion emergente',
    desc: 'Estados que nadie creo explicitamente emergen cuando las distribuciones de probabilidad de eventos cercanos se superponen y cruzan el umbral.',
    detail: 'Dos eventos a distancia d \u2264 2.71 producen cristalizacion en su punto medio. Es el primer mecanismo formal de estado emergente en sistemas distribuidos.',
    metric: 'Primer consenso basado en geometria, no en computacion',
  },
]

const comparisons = [
  {
    label: 'Bitcoin',
    items: [
      { feature: 'Seguridad', them: 'Computacional (hashrate)', us: 'Geometrica (convergencia)' },
      { feature: 'Finalidad', them: 'Probabilistica (nunca 100%)', us: 'Determinista (punto fijo)' },
      { feature: 'Costo de ataque', them: 'O(hashrate) \u2014 lineal', us: 'O(S\u2074) \u2014 exponencial' },
      { feature: 'Auto-sanacion', them: 'Requiere nodos backup', us: 'Automatica desde geometria' },
      { feature: 'Post-cuantico', them: 'Vulnerable (preimagen)', us: 'Inmune (sin primitiva computacional)' },
    ],
  },
  {
    label: 'BFT',
    items: [
      { feature: 'Supuesto', them: 'Mayoria honesta (2f+1)', us: 'Sin supuesto de mayoria' },
      { feature: 'Finalidad', them: 'Condicional a honestidad', us: 'Incondicional' },
      { feature: 'Tolerancia', them: 'Menos de n/3 bizantinos', us: 'Cualquier numero de destrucciones' },
      { feature: 'Estado emergente', them: 'No \u2014 todo explicito', us: 'Si \u2014 estados emergen por proximidad' },
      { feature: 'FLP', them: 'Evitado via sincronia parcial', us: 'No aplica (no es protocolo)' },
    ],
  },
  {
    label: 'DAGs',
    items: [
      { feature: 'Tiempo', them: 'Dimension privilegiada', us: '4 ejes simetricos' },
      { feature: 'Validacion', them: 'Computacional (PoW/tip)', us: 'Convergencia geometrica' },
      { feature: 'Estructura', them: 'Grafo dirigido aciclico', us: 'Campo probabilistico toroidal' },
      { feature: 'Emergencia', them: 'No', us: 'Cristalizacion sin creacion explicita' },
      { feature: 'Sanacion', them: 'Manual / redundancia', us: 'Automatica desde el campo' },
    ],
  },
]

const physicsRules = [
  {
    name: 'Causalidad',
    icon: '\u27F6',
    color: 'bg-emerald-500',
    tagline: 'Nada viaja mas rapido que la luz',
    physics: 'Como los conos de luz en relatividad: un evento solo puede influir lo que podria haber alcanzado. Dos eventos que no se ven son concurrentes \u2014 no necesitan orden.',
    crypto: 'Cada evento tiene un EventId = SHA-256(origen + tiempo + padres + datos). El orden causal se deriva del grafo de dependencias, no de un reloj global.',
    unbreakable: 'Reordenar la causalidad requiere invertir SHA-256 (resistencia a preimagen). Reemplaza el orden total de blockchain con un orden parcial derivado de la fisica.',
    module: 'causality.rs',
    security: 'SHA-256 preimage resistance (128-bit)',
  },
  {
    name: 'Conservacion',
    icon: '\u229C',
    color: 'bg-violet-500',
    tagline: 'La energia no se crea ni se destruye',
    physics: 'Como la conservacion de energia: la cantidad total en el campo es invariante. Las transferencias son operaciones de suma cero \u2014 lo que sale de una celda llega a otra.',
    crypto: 'Compromisos Pedersen sobre Curve25519: C(v,r) = v*G + r*H. La propiedad homomorfica garantiza que sum(inputs) == sum(outputs) sin revelar valores.',
    unbreakable: 'El doble gasto no es una violacion detectada sino una imposibilidad fisica. Romper los compromisos requiere resolver el logaritmo discreto en Ristretto255.',
    module: 'conservation.rs',
    security: 'ECDLP en Curve25519 (128-bit)',
  },
  {
    name: 'Entropia',
    icon: '\u21BB',
    color: 'bg-blue-500',
    tagline: 'La flecha del tiempo solo apunta en una direccion',
    physics: 'Como la segunda ley de la termodinamica: el sistema cristaliza cuando es energeticamente favorable. La temperatura baja con la evidencia. Cristales viejos son permanentes.',
    crypto: 'Cada cristalizacion produce un sello: S(n) = SHA-256(S(n-1) || evidencia). La cadena ES la historia \u2014 reescribir un eslabon invalida todo lo posterior.',
    unbreakable: 'Revertir cuesta energia proporcional a binding_energy x edad. Cuando la energia libre F es negativa, cristalizar es termodinamicamente inevitable.',
    module: 'entropy.rs',
    security: 'SHA-256 hash chain (preimage)',
  },
  {
    name: 'Gravedad',
    icon: '\u25C9',
    color: 'bg-amber-500',
    tagline: 'La masa curva el espacio',
    physics: 'La masa ES el conteo de eventos causales de un participante. Como la gravedad real: la masa es el objeto, no una etiqueta. No se falsifica porque ES la historia.',
    crypto: 'Funcion pura sobre el grafo causal. Sin registro. Influencia decae con el cuadrado de la distancia, previniendo monopolio.',
    unbreakable: 'Sin registro que hackear, sin balance que forjar. Recomputa desde el DAG cada vez. Falsificar masa = forjar pruebas causales = romper SHA-256.',
    module: 'gravity.rs',
    security: 'Sin registro \u2014 computada desde el DAG',
  },
]

type RightTab = 'conceptos' | 'comparativa' | 'leyes' | 'demo'

export default function Tesseract() {
  const [selected, setSelected] = useState(0)
  const [selectedRival, setSelectedRival] = useState(0)
  const [rightTab, setRightTab] = useState<RightTab>('conceptos')
  const [selectedRule, setSelectedRule] = useState(0)
  const [showSimple, setShowSimple] = useState(false)

  useEffect(() => { document.title = 'Tesseract' }, [])

  const rule = physicsRules[selectedRule]

  const drawerOverlay = showSimple
    ? 'fixed inset-0 z-50 flex transition-opacity duration-300 opacity-100 pointer-events-auto'
    : 'fixed inset-0 z-50 flex transition-opacity duration-300 opacity-0 pointer-events-none'

  const drawerPanel = showSimple
    ? 'relative ml-auto bg-white shadow-2xl flex flex-col w-full max-w-xl h-full transition-transform duration-300 ease-out translate-x-0'
    : 'relative ml-auto bg-white shadow-2xl flex flex-col w-full max-w-xl h-full transition-transform duration-300 ease-out translate-x-full'

  return (
    <div className="min-h-screen flex flex-col">
      <section className="flex-1 flex items-center px-6 max-w-screen-xl mx-auto w-full">
        <div className="flex flex-col lg:flex-row gap-12 lg:gap-24 w-full">
          <div className="text-left flex flex-col justify-center lg:w-1/2">
            <p className="text-3xl sm:text-4xl font-bold text-main-500 tracking-tight mb-2">Tesseract</p>
            <p className="text-sm text-neutral-500 mb-4">Consenso por convergencia geometrica</p>
            <h1 className="text-xl sm:text-2xl font-bold text-neutral-900 tracking-tight leading-tight">
              Seguridad que no depende de computacion
            </h1>
            <p className="text-neutral-500 text-base mt-4 leading-relaxed">
              Campo de probabilidad 4D donde los estados cristalizan por convergencia geometrica,
              sin protocolo de consenso, sin validacion computacional, sin supuestos de confianza.
              La verdad emerge porque la geometria converge &mdash; no porque alguien la verifique.
            </p>
            <div className="flex gap-3 mt-8">
              <Link
                to="/"
                className="bg-neutral-100 text-neutral-600 px-5 py-2.5 rounded-xl text-sm font-semibold hover:bg-neutral-200 transition-colors cursor-pointer inline-block"
              >
                Cerulean Ledger
              </Link>
              <a
                href="https://github.com/clementeaf/rust-bc/tree/tesseract-prototype"
                target="_blank"
                rel="noopener noreferrer"
                className="bg-main-500 text-white px-5 py-2.5 rounded-xl text-sm font-semibold hover:bg-main-600 transition-colors shadow-sm hover:shadow-md cursor-pointer inline-block"
              >
                Ver prototipo
              </a>
              <button
                onClick={() => setShowSimple(true)}
                className="bg-neutral-100 text-neutral-600 px-5 py-2.5 rounded-xl text-sm font-semibold hover:bg-neutral-200 transition-colors cursor-pointer"
              >
                En simples palabras
              </button>
            </div>
          </div>

          <div className="hidden lg:flex flex-col justify-center min-h-[340px] lg:w-1/2">
            <div className="flex gap-1 mb-4 relative z-20">
              {([['conceptos', 'Conceptos'], ['leyes', 'Leyes fisicas'], ['comparativa', 'Comparativa'], ['demo', 'Demo']] as const).map(([tab, label]) => (
                <button
                  key={tab}
                  onClick={() => setRightTab(tab)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                    rightTab === tab ? 'bg-main-500 text-white' : 'text-neutral-400 hover:text-neutral-600'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>

            <div className="h-[280px] flex flex-col relative">
              {rightTab === 'conceptos' && (
                <div className="flex-1 flex flex-col">
                  <div className="flex flex-wrap gap-2">
                    {concepts.map((t, i) => (
                      <button
                        key={t.label}
                        onClick={() => setSelected(i)}
                        className={`px-4 py-2 rounded-xl text-sm font-semibold transition-all duration-150 cursor-pointer ${
                          selected === i ? 'bg-main-500 text-white' : 'bg-neutral-100 text-neutral-600 hover:bg-neutral-200'
                        }`}
                      >
                        {t.label}
                      </button>
                    ))}
                  </div>
                  <div className="mt-4 bg-white border border-neutral-200 rounded-2xl px-5 py-4 flex-1 flex flex-col justify-between">
                    <div>
                      <p className="text-neutral-700 text-sm leading-relaxed">{concepts[selected].desc}</p>
                      <p className="text-neutral-500 text-xs leading-relaxed mt-3">{concepts[selected].detail}</p>
                    </div>
                    <div className="mt-4 pt-3 border-t border-neutral-100">
                      <p className="text-main-600 text-xs font-semibold">{concepts[selected].metric}</p>
                    </div>
                  </div>
                </div>
              )}

              {rightTab === 'comparativa' && (
                <div className="flex-1 flex flex-col">
                  <div className="flex flex-wrap gap-2">
                    {comparisons.map((r, i) => (
                      <button
                        key={r.label}
                        onClick={() => setSelectedRival(i)}
                        className={`px-4 py-2 rounded-xl text-sm font-semibold transition-all duration-150 cursor-pointer ${
                          selectedRival === i ? 'bg-main-500 text-white' : 'bg-neutral-100 text-neutral-600 hover:bg-neutral-200'
                        }`}
                      >
                        vs {r.label}
                      </button>
                    ))}
                  </div>
                  <div className="mt-4 bg-white border border-neutral-200 rounded-2xl px-5 py-4 flex-1">
                    <div className="space-y-2.5">
                      {comparisons[selectedRival].items.map((item) => (
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
                        <span className="text-main-500">Tesseract</span>
                        <span>{comparisons[selectedRival].label}</span>
                      </span>
                    </div>
                  </div>
                </div>
              )}

              {rightTab === 'demo' && (
                <div className="flex-1 flex flex-col">
                  <Suspense fallback={<div className="flex-1 flex items-center justify-center text-neutral-400 text-xs">Cargando...</div>}>
                    <FieldDemo />
                  </Suspense>
                </div>
              )}

              {rightTab === 'leyes' && (
                <div className="flex-1 flex flex-col">
                  <div className="flex flex-wrap gap-2">
                    {physicsRules.map((r, i) => (
                      <button
                        key={r.name}
                        onClick={() => setSelectedRule(i)}
                        className={`flex items-center gap-1.5 px-3 py-2 rounded-xl text-sm font-semibold transition-all duration-150 cursor-pointer ${
                          selectedRule === i ? 'bg-main-500 text-white' : 'bg-neutral-100 text-neutral-600 hover:bg-neutral-200'
                        }`}
                      >
                        <span className="text-xs">{r.icon}</span>
                        {r.name}
                      </button>
                    ))}
                  </div>
                  <div className="mt-4 bg-white border border-neutral-200 rounded-2xl px-5 py-4 flex-1 flex flex-col justify-between overflow-y-auto">
                    <div>
                      <p className="text-neutral-900 font-semibold text-sm flex items-center gap-2 mb-3">
                        <span className={'w-2 h-2 rounded-full ' + rule.color + ' inline-block'} />
                        {rule.tagline}
                      </p>
                      <div className="space-y-2.5">
                        <div>
                          <p className="text-neutral-400 text-[10px] uppercase tracking-wider font-semibold mb-0.5">Fisica</p>
                          <p className="text-neutral-700 text-xs leading-relaxed">{rule.physics}</p>
                        </div>
                        <div>
                          <p className="text-neutral-400 text-[10px] uppercase tracking-wider font-semibold mb-0.5">Criptografia</p>
                          <p className="text-neutral-700 text-xs leading-relaxed">{rule.crypto}</p>
                        </div>
                        <div>
                          <p className="text-neutral-400 text-[10px] uppercase tracking-wider font-semibold mb-0.5">Inquebrantable</p>
                          <p className="text-neutral-700 text-xs leading-relaxed">{rule.unbreakable}</p>
                        </div>
                      </div>
                    </div>
                    <div className="mt-3 pt-2 border-t border-neutral-100 flex items-center gap-4 text-[10px]">
                      <span className="text-neutral-400 font-mono">{rule.module}</span>
                      <span className="text-main-600 font-semibold">{rule.security}</span>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </section>

      <section className="border-t border-neutral-200 px-6 py-4 flex-shrink-0">
        <div className="max-w-4xl mx-auto flex items-center justify-center gap-8 text-xs text-neutral-400">
          <div className="text-center">
            <p className="font-semibold text-neutral-300">Pre-Bitcoin</p>
            <p>Confia en la institucion</p>
          </div>
          <span className="text-neutral-300">{'\u2192'}</span>
          <div className="text-center">
            <p className="font-semibold text-neutral-300">Bitcoin 2009</p>
            <p>No confies &mdash; verifica</p>
          </div>
          <span className="text-neutral-300">{'\u2192'}</span>
          <div className="text-center">
            <p className="font-semibold text-main-500">Tesseract</p>
            <p>Nada que verificar &mdash; el estado es o no es</p>
          </div>
        </div>
      </section>

      <footer className="border-t border-neutral-200 px-6 py-3">
        <div className="max-w-4xl mx-auto flex items-center justify-between text-[11px] text-neutral-400">
          <span>Tesseract &mdash; Cerulean Ledger</span>
          <div className="flex items-center gap-4">
            <span>Rust</span>
            <span>Campo 4D</span>
            <span>Prototipo</span>
          </div>
        </div>
      </footer>

      <div className={drawerOverlay}>
        <div className="absolute inset-0 bg-black/40" onClick={() => setShowSimple(false)} />
        <div className={drawerPanel}>
          <div className="flex items-center justify-between px-6 py-4 border-b border-neutral-100 shrink-0">
            <p className="text-neutral-900 font-bold text-base">En simples palabras</p>
            <button
              onClick={() => setShowSimple(false)}
              className="text-neutral-400 hover:text-neutral-600 transition-colors cursor-pointer text-xl leading-none"
            >
              &times;
            </button>
          </div>
          <div className="flex-1 overflow-y-auto px-6 py-6">
            <div>
              <h2 className="text-xl font-bold text-neutral-900 mb-5">
                Imagina un mundo donde la informacion no se guarda &mdash; simplemente existe.
              </h2>
              <div className="space-y-4 text-sm text-neutral-600 leading-relaxed">
                <p>
                  Hoy, cuando envias dinero o firmas un documento digital, alguien tiene que <strong className="text-neutral-800">verificar</strong> que
                  todo esta bien: un banco, un servidor, una red de mineros. Si ese verificador falla, miente o es hackeado, el sistema se rompe.
                </p>
                <p>
                  Tesseract elimina al verificador. En vez de confiar en alguien que revise,
                  usa <strong className="text-neutral-800">geometria matematica</strong> para que la informacion correcta emerja sola &mdash; como
                  la gravedad hace caer las cosas sin que nadie lo ordene.
                </p>
                <p>
                  Cada dato existe como una <strong className="text-neutral-800">nube de probabilidad</strong> en un espacio de 4 dimensiones.
                  Cuando suficiente evidencia independiente apunta a lo mismo, el dato <strong className="text-neutral-800">cristaliza</strong> y
                  se vuelve permanente. No porque alguien lo aprobo, sino porque la matematica convergio.
                </p>
              </div>
              <div className="mt-8 flex flex-col gap-3">
                <div className="bg-surface-alt border border-neutral-200 rounded-xl px-4 py-3">
                  <p className="text-neutral-900 font-semibold text-sm mb-1">Si lo destruyes, vuelve</p>
                  <p className="text-neutral-500 text-xs leading-relaxed">
                    Borrar un dato no sirve. El campo lo regenera desde sus vecinos en 2-5 pasos, sin backup.
                    Como borrar una ola del mar &mdash; el oceano la recrea.
                  </p>
                </div>
                <div className="bg-surface-alt border border-neutral-200 rounded-xl px-4 py-3">
                  <p className="text-neutral-900 font-semibold text-sm mb-1">Si mientes, no se propaga</p>
                  <p className="text-neutral-500 text-xs leading-relaxed">
                    Un dato falso inyectado a la fuerza queda aislado. Sin evidencia desde multiples
                    angulos, no puede expandirse. La mentira muere sola.
                  </p>
                </div>
                <div className="bg-surface-alt border border-neutral-200 rounded-xl px-4 py-3">
                  <p className="text-neutral-900 font-semibold text-sm mb-1">Si hackeas, no hay que hackear</p>
                  <p className="text-neutral-500 text-xs leading-relaxed">
                    No hay servidor central, no hay clave maestra, no hay registro que alterar.
                    La seguridad no viene de un mecanismo &mdash; viene de la estructura del espacio.
                  </p>
                </div>
              </div>
              <div className="mt-8 p-4 bg-main-50 border border-main-100 rounded-xl">
                <p className="text-main-800 text-sm font-semibold mb-2">La analogia mas simple</p>
                <p className="text-main-700 text-sm leading-relaxed">
                  Piensa en como la gravedad mantiene los planetas en orbita. Nadie lo ordena, nadie lo vigila,
                  nadie lo puede hackear. Funciona porque es una propiedad del espacio mismo.
                  Tesseract hace lo mismo con la informacion: la verdad se mantiene porque la geometria
                  del campo no permite otra cosa.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
