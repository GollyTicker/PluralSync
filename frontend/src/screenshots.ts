import type { Screenshot } from './components/AboutPage.vue'

// Vite's glob import with eager: true to actually import the images
// This gives us the processed asset URLs with hashes
const screenshotModules = import.meta.glob<{ default: string }>(
  '../screenshots/*.{png,jpg,jpeg,gif,webp,svg}',
  { eager: true },
)

// Convert to Screenshot array
export const screenshots: Screenshot[] = Object.entries(screenshotModules).map(([path, module]) => {
  // Extract filename from path
  const filename = path.split('/').pop() || ''
  const name = filename.replace(/\.[^/.]+$/, '')

  // Use the actual imported URL (Vite-processed with hash)
  const imageUrl = module.default

  return {
    filename,
    path: imageUrl,
    name,
  }
})

// Fail build if no screenshots found
if (screenshots.length === 0) {
  throw new Error(
    'Screenshots directory is empty - at least one screenshot is required for the About page',
  )
}
