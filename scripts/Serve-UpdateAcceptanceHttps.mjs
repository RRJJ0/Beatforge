import { createReadStream, readFileSync, statSync } from 'node:fs';
import { createServer } from 'node:https';
import { basename, join, resolve } from 'node:path';

const [bundleDirectory, portText] = process.argv.slice(2);
const port = Number(portText);
if (!bundleDirectory || !Number.isInteger(port) || port < 1024 || port > 65535) {
  throw new Error('Usage: node Serve-UpdateAcceptanceHttps.mjs <bundle-directory> <port>');
}
const passphrase = process.env.BEATFORGE_UPDATE_ACCEPTANCE_PFX_PASSWORD;
const pfxPath = process.env.BEATFORGE_UPDATE_ACCEPTANCE_PFX_PATH;
if (!passphrase || !pfxPath) throw new Error('The temporary local HTTPS certificate is missing');

const hostingDirectory = resolve(bundleDirectory, 'hosting');
const manifest = JSON.parse(readFileSync(join(hostingDirectory, 'latest.json'), 'utf8'));
const installerName = basename(new URL(manifest.platforms['windows-x86_64'].url).pathname);
if (!/^Beatforge_\d+\.\d+\.\d+_x64-setup\.exe$/.test(installerName)) {
  throw new Error('The acceptance manifest has an unexpected installer filename');
}
const allowedFiles = new Map([
  ['/latest.json', 'application/json; charset=utf-8'],
  [`/${installerName}`, 'application/octet-stream'],
  [`/${installerName}.sig`, 'text/plain; charset=utf-8'],
]);
const tls = { pfx: readFileSync(pfxPath), passphrase, minVersion: 'TLSv1.2' };
const servers = [];

function serve(request, response) {
  const pathname = new URL(request.url, 'https://localhost').pathname;
  const contentType = allowedFiles.get(pathname);
  if (!contentType || !['GET', 'HEAD'].includes(request.method)) {
    response.writeHead(404);
    response.end();
    return;
  }
  const path = join(hostingDirectory, pathname.slice(1));
  const size = statSync(path).size;
  response.writeHead(200, {
    'Content-Type': contentType,
    'Content-Length': size,
    'Cache-Control': 'no-store',
    'X-Content-Type-Options': 'nosniff',
  });
  if (request.method === 'HEAD') response.end();
  else createReadStream(path).pipe(response);
}

async function listen(host) {
  const server = createServer(tls, serve);
  await new Promise((resolveReady, reject) => {
    server.once('error', reject);
    server.listen(port, host, resolveReady);
  });
  servers.push(server);
}

try {
  await listen('127.0.0.1');
  await listen('::1');
  console.log(`Serving exactly three acceptance files at https://localhost:${port}/`);
  console.log('Loopback only. Press Ctrl+C after the updater test.');
} catch (error) {
  for (const server of servers) server.close();
  throw error;
}

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    for (const server of servers) server.close();
    process.exitCode = 0;
  });
}
