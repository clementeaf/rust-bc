import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import './index.css'
import Layout from './components/Layout'
import Home from './pages/Home'
import BlockDetail from './pages/BlockDetail'
import WalletDetail from './pages/WalletDetail'
import Wallets from './pages/Wallets'
import Transactions from './pages/Transactions'
import Mining from './pages/Mining'
import Contracts from './pages/Contracts'
import ContractDetail from './pages/ContractDetail'
import Validators from './pages/Validators'
import Staking from './pages/Staking'
import Channels from './pages/Channels'
import Governance from './pages/Governance'
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
          <Route path="/wallets" element={<Wallets />} />
          <Route path="/wallet/:address" element={<WalletDetail />} />
          <Route path="/transactions" element={<Transactions />} />
          <Route path="/mining" element={<Mining />} />
          <Route path="/contracts" element={<Contracts />} />
          <Route path="/contract/:address" element={<ContractDetail />} />
          <Route path="/validators" element={<Validators />} />
          <Route path="/staking" element={<Staking />} />
          <Route path="/channels" element={<Channels />} />
          <Route path="/governance" element={<Governance />} />
          <Route path="/airdrop" element={<Airdrop />} />
          <Route path="/identity" element={<Identity />} />
          <Route path="/credentials" element={<Credentials />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </StrictMode>,
)
