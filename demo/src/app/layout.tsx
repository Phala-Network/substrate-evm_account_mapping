import type { Metadata } from 'next'
import '@fontsource/dejavu-mono'
import '@fontsource/dejavu-serif'
import './globals.css'

export const metadata: Metadata = {
  title: 'EVM Account Mapping Pallet for Substrate',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body className="font-sans">{children}</body>
    </html>
  )
}
