import { StrictMode, Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import './index.css'
import Layout from './components/Layout'
import { routes } from './lib/routes'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <Routes>
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
