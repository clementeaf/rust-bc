import type { ReactElement, ReactNode } from 'react'

/**
 * Título de página y párrafo introductorio para que quede claro qué muestra cada sección.
 */
interface PageIntroProps {
  title: string
  children: ReactNode
}

export default function PageIntro({ title, children }: PageIntroProps): ReactElement {
  return (
    <header className="mb-8">
      <h1 className="text-2xl font-bold text-white tracking-tight">{title}</h1>
      <p className="text-gray-400 text-sm mt-2 max-w-3xl leading-relaxed">{children}</p>
    </header>
  )
}
