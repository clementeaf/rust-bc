import { StrictMode, Suspense, lazy } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import './index.css'
import Layout from './components/Layout'
import { routes } from './lib/routes'

const Landing = lazy(() => import('./pages/Landing'))

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <Routes>
        <Route
          path="/"
          element={
            <Suspense fallback={<div className="py-12 text-center text-neutral-400">Cargando...</div>}>
              <Landing />
            </Suspense>
          }
        />
        <Route element={<Layout />}>
          {routes.map(({ path, component: Page }) => (
            <Route
              key={path}
              path={path}
              element={
                <Suspense fallback={<div className="py-12 text-center text-neutral-400">Cargando...</div>}>
                  <Page />
                </Suspense>
              }
            />
          ))}
        </Route>
      </Routes>
    </BrowserRouter>
  </StrictMode>,
)
