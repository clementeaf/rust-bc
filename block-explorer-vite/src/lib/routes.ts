import { lazy } from 'react'

export interface RouteEntry {
  path: string
  component: React.LazyExoticComponent<React.ComponentType>
}

export const routes: RouteEntry[] = [
  { path: '/dashboard', component: lazy(() => import('../pages/Home')) },
  { path: '/demo', component: lazy(() => import('../pages/Demo')) },
  { path: '/block/:hash', component: lazy(() => import('../pages/BlockDetail')) },
  { path: '/wallets', component: lazy(() => import('../pages/Wallets')) },
  { path: '/wallet/:address', component: lazy(() => import('../pages/WalletDetail')) },
  { path: '/transactions', component: lazy(() => import('../pages/Transactions')) },
  { path: '/mining', component: lazy(() => import('../pages/Mining')) },
  { path: '/contracts', component: lazy(() => import('../pages/Contracts')) },
  { path: '/contract/:address', component: lazy(() => import('../pages/ContractDetail')) },
  { path: '/validators', component: lazy(() => import('../pages/Validators')) },
  { path: '/staking', component: lazy(() => import('../pages/Staking')) },
  { path: '/channels', component: lazy(() => import('../pages/Channels')) },
  { path: '/airdrop', component: lazy(() => import('../pages/Airdrop')) },
  { path: '/identity', component: lazy(() => import('../pages/Identity')) },
  { path: '/credentials', component: lazy(() => import('../pages/Credentials')) },
  { path: '/governance', component: lazy(() => import('../pages/Governance')) },
  { path: '/crypto', component: lazy(() => import('../pages/Crypto')) },
]
