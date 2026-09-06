// Assembles the weavatrix-md npm package around prebuilt binaries.
// Node built-ins only: no third-party code, no install scripts, no network.
//
//   node scripts/build-npm-packages.mjs current <platform-key> <binary-path> [version]
import {
    chmodSync,
    copyFileSync,
    cpSync,
    mkdirSync,
    readFileSync,
    rmSync,
    writeFileSync,
} from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const WRAPPER = join(ROOT, 'npm', 'weavatrix-md')
const DIST = join(ROOT, 'npm', 'dist')

const PLATFORMS = {
    'win32-x64': { os: 'win32', cpu: 'x64', binary: 'weavatrix-md.exe' },
    'win32-arm64': { os: 'win32', cpu: 'arm64', binary: 'weavatrix-md.exe' },
    'darwin-x64': { os: 'darwin', cpu: 'x64', binary: 'weavatrix-md' },
    'darwin-arm64': { os: 'darwin', cpu: 'arm64', binary: 'weavatrix-md' },
    'linux-x64': { os: 'linux', cpu: 'x64', binary: 'weavatrix-md' },
    'linux-arm64': { os: 'linux', cpu: 'arm64', binary: 'weavatrix-md' },
}

const wrapperManifest = JSON.parse(
    readFileSync(join(WRAPPER, 'package.json'), 'utf8').replace(/^\uFEFF/, ''),
)
const [, , mode, ...rest] = process.argv
if (mode !== 'current') usage()

const [platform, binaryPath, versionArg] = rest
const entry = PLATFORMS[platform]
if (!entry || !binaryPath) usage()
assemble(versionArg || wrapperManifest.version, { [platform]: binaryPath })

function assemble(version, binaries) {
    const target = join(DIST, 'weavatrix-md')
    rmSync(target, { recursive: true, force: true })
    cpSync(WRAPPER, target, { recursive: true })
    const manifest = { ...wrapperManifest, version }
    writeFileSync(join(target, 'package.json'), `${JSON.stringify(manifest, null, 2)}\n`)
    copyFileSync(join(ROOT, 'LICENSE'), join(target, 'LICENSE'))
    copyFileSync(join(ROOT, 'README.md'), join(target, 'README.md'))
    cpSync(join(ROOT, 'skill'), join(target, 'skill'), { recursive: true })
    for (const [platformKey, source] of Object.entries(binaries)) {
        const { os, binary } = PLATFORMS[platformKey]
        const destination = join(target, 'bin', 'native', platformKey, binary)
        mkdirSync(dirname(destination), { recursive: true })
        copyFileSync(source, destination)
        if (os !== 'win32') chmodSync(destination, 0x1ed)
    }
    console.log(`assembled ${target} @ ${version}`)
}

function usage() {
    console.error('usage:')
    console.error(
        '  node scripts/build-npm-packages.mjs current <platform-key> <binary-path> [version]',
    )
    process.exit(1)
}
