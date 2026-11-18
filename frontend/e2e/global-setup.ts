import { createServer, type IncomingMessage, type ServerResponse } from 'node:http'

// The app is started with OPENROUTER_BASE_URL pointing here. In Docker CI the
// Playwright container shares the app's network namespace, so 127.0.0.1 is the
// same interface for both processes. No real provider or credentials are used.
const port = Number(process.env.MOCK_OPENROUTER_PORT ?? 8099)

function readBody(request: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let body = ''
    request.setEncoding('utf8')
    request.on('data', (chunk: string) => {
      body += chunk
    })
    request.on('end', () => resolve(body))
    request.on('error', reject)
  })
}

function sendJson(response: ServerResponse, status: number, payload: unknown) {
  response.writeHead(status, { 'content-type': 'application/json' })
  response.end(JSON.stringify(payload))
}

function completion(items: unknown[]) {
  return {
    choices: [
      {
        message: {
          content: JSON.stringify({ items }),
        },
      },
    ],
  }
}

export default async function globalSetup() {
  const server = createServer(async (request, response) => {
    if (request.method !== 'POST' || !request.url?.endsWith('/chat/completions')) {
      response.writeHead(404).end()
      return
    }

    const body = await readBody(request)
    if (body.includes('E2E_AI_FAIL')) {
      sendJson(response, 500, {
        error: { code: 500, message: 'mock provider failure' },
      })
      return
    }

    sendJson(
      response,
      200,
      completion([
        {
          type: 'task',
          title: 'Buy oat milk',
          content: 'The barista edition',
          category: 'errands',
          tags: ['home', 'shopping'],
          priority: 'normal',
          due_at: null,
          remind_at: null,
        },
        {
          type: 'task',
          title: 'Renew the domain',
          content: null,
          category: 'work',
          tags: [],
          priority: 'high',
          due_at: '2026-12-15T09:00:00Z',
          remind_at: null,
        },
      ]),
    )
  })

  await new Promise<void>((resolve, reject) => {
    server.once('error', reject)
    server.listen(port, '127.0.0.1', resolve)
  })

  return async () => {
    await new Promise<void>((resolve) => server.close(() => resolve()))
  }
}
