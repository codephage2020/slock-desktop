import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Agentation } from 'agentation'
import './index.css'
import './design-system.css'
import App from './App.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
    {import.meta.env.DEV && <Agentation />}
  </StrictMode>,
)
