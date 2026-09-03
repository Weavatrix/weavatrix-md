import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const BUNDLED_BINARIES = {
    'win32 x64': ['win32-x64', 'weavatrix-md.exe'],
    'win32 arm64': ['win32-arm64', 'weavatrix-md.exe'],
    'darwin x64': ['darwin-x64', 'weavatrix-md'],
    'darwin arm64': ['darwin-arm64', 'weavatrix-md'],
    'linux x64': ['linux-x64', 'weavatrix-md'],
    'linux arm64': ['linux-arm64', 'weavatrix-md'],
}

export function resolveBinary() {
    const key = `${process.platform} ${process.arch}`
    const entry = BUNDLED_BINARIES[key]
    if (!entry) {
        fail(
            `Unsupported platform: ${key}.`,
            'Prebuilt binaries cover win32/darwin/linux on x64 and arm64.',
            'On other platforms: cargo install weavatrix-md --locked',
        )
    }
    const [directory, binaryName] = entry
    const binary = join(dirname(fileURLToPath(import.meta.url)), 'native', directory, binaryName)
    if (!existsSync(binary)) {
        fail(
            `The bundled native executable for ${key} is missing.`,
            'Install with cargo install weavatrix-md --locked,',
            'or reinstall the npm package from a complete multi-platform release.',
        )
    }
    return binary
}

function fail(...lines) {
    for (const line of lines) console.error(`weavatrix-md: ${line}`)
    process.exit(1)
}
