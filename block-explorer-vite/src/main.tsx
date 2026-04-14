import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import './index.css'
import Layout from './components/Layout'
import Home from './pages/Home'
import BlockDetail from './pages/BlockDetail'
import WalletDetail from './pages/WalletDetail'
import Contracts from './pages/Contracts'
import ContractDetail from './pages/ContractDetail'
import Validators from './pages/Validators'
import Airdrop from './pages/Airdrop'
import Identity from './pages/Identity'
import Credentials from './pages/Credentials'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<Home />} />
          <Route path="/block/:hash" element={<BlockDetail />} />
          <Route path="/wallet/:address" element={<WalletDetail />} />
          <Route path="/contracts" element={<Contracts />} />
          <Route path="/contract/:address" element={<ContractDetail />} />
          <Route path="/validators" element={<Validators />} />
          <Route path="/airdrop" element={<Airdrop />} />
          <Route path="/identity" element={<Identity />} />
          <Route path="/credentials" element={<Credentials />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </StrictMode>,
)
