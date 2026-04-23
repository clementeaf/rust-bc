import { StrictMode, Suspense, lazy } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import './index.css'
import Layout from './components/Layout'
import ServicesLayout from './components/ServicesLayout'
import { routes } from './lib/routes'

const Landing = lazy(() => import('./pages/Landing'))
const TesseractPage = lazy(() => import('./pages/Tesseract'))
const Services = lazy(() => import('./pages/Services'))

const Fallback = <div className="py-12 text-center text-neutral-400">Cargando...</div>

// Service pages reused under /services/*
const Demo = lazy(() => import('./pages/Demo'))
const Identity = lazy(() => import('./pages/Identity'))
const Credentials = lazy(() => import('./pages/Credentials'))
const Governance = lazy(() => import('./pages/Governance'))
const Home = lazy(() => import('./pages/Home'))
const Wallets = lazy(() => import('./pages/Wallets'))
const Transactions = lazy(() => import('./pages/Transactions'))
const Staking = lazy(() => import('./pages/Staking'))
const Channels = lazy(() => import('./pages/Channels'))
const Mining = lazy(() => import('./pages/Mining'))
const Contracts = lazy(() => import('./pages/Contracts'))

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <Routes>
        {/* Landing — standalone, no layout */}
        <Route path="/" element={<Suspense fallback={Fallback}><Landing /></Suspense>} />
        <Route path="/tesseract" element={<Suspense fallback={Fallback}><TesseractPage /></Suspense>} />

        {/* Services — all under ServicesLayout for consistent header */}
        <Route element={<ServicesLayout />}>
          <Route path="/services" element={<Suspense fallback={Fallback}><Services /></Suspense>} />
          <Route path="/services/demo" element={<Suspense fallback={Fallback}><Demo /></Suspense>} />
          <Route path="/services/identity" element={<Suspense fallback={Fallback}><Identity /></Suspense>} />
          <Route path="/services/credentials" element={<Suspense fallback={Fallback}><Credentials /></Suspense>} />
          <Route path="/services/governance" element={<Suspense fallback={Fallback}><Governance /></Suspense>} />
          <Route path="/services/dashboard" element={<Suspense fallback={Fallback}><Home /></Suspense>} />
          <Route path="/services/wallets" element={<Suspense fallback={Fallback}><Wallets /></Suspense>} />
          <Route path="/services/transactions" element={<Suspense fallback={Fallback}><Transactions /></Suspense>} />
          <Route path="/services/staking" element={<Suspense fallback={Fallback}><Staking /></Suspense>} />
          <Route path="/services/channels" element={<Suspense fallback={Fallback}><Channels /></Suspense>} />
          <Route path="/services/mining" element={<Suspense fallback={Fallback}><Mining /></Suspense>} />
          <Route path="/services/contracts" element={<Suspense fallback={Fallback}><Contracts /></Suspense>} />
        </Route>

        {/* Legacy layout routes (sidebar) — kept for backward compat */}
        <Route element={<Layout />}>
          {routes.map(({ path, component: Page }) => (
            <Route
              key={path}
              path={path}
              element={<Suspense fallback={Fallback}><Page /></Suspense>}
            />
          ))}
        </Route>
      </Routes>
    </BrowserRouter>
  </StrictMode>,
)
