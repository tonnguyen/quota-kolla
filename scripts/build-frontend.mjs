import { cp, mkdir, readdir, rm } from 'node:fs/promises'
import { join } from 'node:path'

const root = process.cwd()
const srcDir = join(root, 'src')
const distDir = join(root, 'dist')

const filesToCopy = [
  'menu.html',
  'menu.css',
  'menu.js',
  'preferences.html',
  'preferences.css',
  'preferences.js',
  'index.html',
  'main.js',
  'styles.css',
]

console.log('Building frontend assets...')

await rm(distDir, { recursive: true, force: true })
await mkdir(distDir, { recursive: true })

for (const file of filesToCopy) {
  await cp(join(srcDir, file), join(distDir, file), { force: true })
}

await cp(join(srcDir, 'assets'), join(distDir, 'assets'), { recursive: true, force: true })

console.log('Build complete! dist/ directory contents:')
for (const entry of await readdir(distDir)) {
  console.log(`- ${entry}`)
}
