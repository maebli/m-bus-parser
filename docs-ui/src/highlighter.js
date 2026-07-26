import json from '@shikijs/langs/json'
import yaml from '@shikijs/langs/yaml'
import csv from '@shikijs/langs/csv'
import xml from '@shikijs/langs/xml'
import githubLightHighContrast from '@shikijs/themes/github-light-high-contrast'
import githubDarkHighContrast from '@shikijs/themes/github-dark-high-contrast'
import { createHighlighterCore } from 'shiki/core'
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript'

const supportedLanguages = new Set(['json', 'yaml', 'csv', 'xml'])
const aliases = new Map([
  ['yml', 'yaml'],
  ['text', 'plaintext'],
  ['txt', 'plaintext'],
  ['table', 'plaintext'],
])

let highlighterPromise

function getHighlighter() {
  highlighterPromise ??= createHighlighterCore({
    themes: [githubLightHighContrast, githubDarkHighContrast],
    langs: [json, yaml, csv, xml],
    engine: createJavaScriptRegexEngine(),
  })
  return highlighterPromise
}

function normalizeLanguage(language) {
  const normalized = String(language || 'plaintext').toLowerCase()
  return aliases.get(normalized) || normalized
}

function createPlainCode(source) {
  const pre = document.createElement('pre')
  pre.className = 'code-source'
  const code = document.createElement('code')
  code.textContent = source
  pre.appendChild(code)
  return pre
}

function createLineGutter(source) {
  const gutter = document.createElement('div')
  gutter.className = 'line-gutter'
  gutter.setAttribute('aria-hidden', 'true')
  const lineCount = Math.max(1, source.split('\n').length)
  for (let line = 1; line <= lineCount; line += 1) {
    const number = document.createElement('span')
    number.textContent = String(line)
    gutter.appendChild(number)
  }
  return gutter
}

function createViewer(source, codeElement, label) {
  const viewer = document.createElement('div')
  viewer.className = 'code-viewer'

  const scroll = document.createElement('div')
  scroll.className = 'code-scroll'
  scroll.setAttribute('role', 'region')
  scroll.setAttribute('aria-label', label)
  scroll.tabIndex = 0

  codeElement.classList.add('code-source')
  codeElement.removeAttribute('tabindex')
  scroll.append(createLineGutter(source), codeElement)
  viewer.appendChild(scroll)
  return viewer
}

/**
 * Render output with deterministic line numbers and a safe plaintext fallback.
 * The returned promise resolves after Shiki has replaced the fallback.
 */
export async function renderCodeViewer(
  container,
  source,
  language = 'plaintext',
  label = 'Parser output',
) {
  source = String(source ?? '')
  const normalizedLanguage = normalizeLanguage(language)
  const fallback = createViewer(source, createPlainCode(source), label)
  container.replaceChildren(fallback)

  if (!supportedLanguages.has(normalizedLanguage)) return fallback

  try {
    const highlighter = await getHighlighter()
    const html = highlighter.codeToHtml(source, {
      lang: normalizedLanguage,
      themes: {
        light: 'github-light-high-contrast',
        dark: 'github-dark-high-contrast',
      },
      defaultColor: false,
    })
    const template = document.createElement('template')
    template.innerHTML = html
    const highlighted = template.content.querySelector('pre')
    if (!highlighted || !container.contains(fallback)) return fallback
    const viewer = createViewer(source, highlighted, label)
    container.replaceChildren(viewer)
    return viewer
  } catch (error) {
    console.warn('Syntax highlighting unavailable; using plaintext output.', error)
    return fallback
  }
}
