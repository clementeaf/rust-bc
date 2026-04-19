import { Link } from 'react-router-dom'

export default function Home() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[85vh] px-6">
      {/* Logo mark */}
      <div className="w-16 h-16 rounded-2xl bg-main-500 flex items-center justify-center mb-8 shadow-lg shadow-main-500/20">
        <span className="text-white font-extrabold text-2xl tracking-tight">CL</span>
      </div>

      {/* Name */}
      <h1 className="text-5xl sm:text-7xl font-extrabold tracking-tight text-neutral-900 leading-none text-center">
        Cerulean Ledger
      </h1>

      {/* Tagline */}
      <p className="mt-6 text-lg sm:text-xl text-neutral-500 leading-relaxed text-center max-w-xl">
        Infraestructura blockchain post-cuantica para identidad descentralizada y credenciales verificables.
      </p>

      {/* Features line */}
      <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
        {['Consenso BFT', 'Canales privados', 'Smart Contracts Wasm', 'ML-DSA-65'].map((feat) => (
          <span
            key={feat}
            className="px-3.5 py-1.5 text-xs font-semibold text-main-700 bg-main-50 border border-main-100 rounded-full"
          >
            {feat}
          </span>
        ))}
      </div>

      {/* Tesseract link */}
      <Link
        to="/tesseract"
        className="mt-10 px-6 py-3 bg-main-500 text-white font-semibold text-sm rounded-xl
                   hover:bg-main-600 transition-colors shadow-md shadow-main-500/20"
      >
        Tesseract Prototype
      </Link>

      {/* Subtle bottom line */}
      <p className="mt-12 text-sm text-neutral-400">
        Criptografia lista para el futuro.
      </p>
    </div>
  )
}
