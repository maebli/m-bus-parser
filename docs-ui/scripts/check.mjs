import { gzipSync } from 'node:zlib'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import githubLightHighContrast from '@shikijs/themes/github-light-high-contrast'
import githubDarkHighContrast from '@shikijs/themes/github-dark-high-contrast'

const REQUIRED_RATIO = 4.5
const MAX_GZIP_BYTES = 100 * 1024

function channels(color) {
  const match = /^#([\da-f]{6})(?:[\da-f]{2})?$/i.exec(color)
  if (!match) return null
  return [0, 2, 4].map((offset) => Number.parseInt(match[1].slice(offset, offset + 2), 16))
}

function luminance(color) {
  const rgb = channels(color)
  if (!rgb) return null
  const linear = rgb.map((value) => {
    const channel = value / 255
    return channel <= 0.04045
      ? channel / 12.92
      : ((channel + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
}

function contrast(foreground, background) {
  const foregroundLuminance = luminance(foreground)
  const backgroundLuminance = luminance(background)
  if (foregroundLuminance === null || backgroundLuminance === null) return null
  const lighter = Math.max(foregroundLuminance, backgroundLuminance)
  const darker = Math.min(foregroundLuminance, backgroundLuminance)
  return (lighter + 0.05) / (darker + 0.05)
}

function themeColors(theme) {
  const colors = new Set()
  if (theme.fg) colors.add(theme.fg)
  for (const setting of theme.settings || []) {
    if (setting.settings?.foreground) colors.add(setting.settings.foreground)
  }
  return colors
}

function checkTheme(theme) {
  const background = theme.bg || theme.colors?.['editor.background']
  const failures = []
  for (const color of themeColors(theme)) {
    const ratio = contrast(color, background)
    if (ratio !== null && ratio + Number.EPSILON < REQUIRED_RATIO) {
      failures.push(`${color} on ${background}: ${ratio.toFixed(2)}:1`)
    }
  }
  if (failures.length) {
    throw new Error(`${theme.name} has sub-AA token colors:\n${failures.join('\n')}`)
  }
}

checkTheme(githubLightHighContrast)
checkTheme(githubDarkHighContrast)

for (const [foreground, background, label] of [
  ['#4b5563', '#ffffff', 'light line numbers'],
  ['#c7cdd5', '#0d1117', 'dark line numbers'],
]) {
  const ratio = contrast(foreground, background)
  if (ratio < REQUIRED_RATIO) {
    throw new Error(`${label} contrast is ${ratio.toFixed(2)}:1`)
  }
}

const bundlePath = resolve(import.meta.dirname, '../../docs/assets/highlighter.js')
const gzipBytes = gzipSync(readFileSync(bundlePath)).byteLength
if (gzipBytes > MAX_GZIP_BYTES) {
  throw new Error(`highlighter bundle is ${gzipBytes} gzip bytes; budget is ${MAX_GZIP_BYTES}`)
}

console.log(`Syntax themes meet WCAG AA; highlighter bundle is ${gzipBytes} gzip bytes.`)
