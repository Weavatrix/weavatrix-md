import { spawn } from 'node:child_process'
import { resolveBinary } from './resolve-binary.mjs'

export function runNative() {
    const binary = resolveBinary()
    const args = process.argv.slice(2)
    if (['darwin', 'linux'].includes(process.platform) && typeof process.execve === 'function') {
        process.execve(binary, [binary, ...args], process.env)
    }
    const child = spawn(binary, args, { stdio: 'inherit', windowsHide: true })
    child.on('error', (error) => {
        console.error(`weavatrix-md: failed to start native binary: ${error.message}`)
        process.exit(1)
    })
    child.on('exit', (code, signal) => {
        if (signal) process.kill(process.pid, signal)
        process.exit(code ?? 1)
    })
    for (const signal of ['SIGINT', 'SIGTERM']) {
        process.on(signal, () => child.kill(signal))
    }
}
