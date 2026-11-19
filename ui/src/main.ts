import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { initializeAppearance } from './lib/appearance.svelte'

initializeAppearance()
const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
