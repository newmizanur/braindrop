# BrainDrop frontend

React 19 + TypeScript + Vite SPA, built into `frontend/dist` and served by Loco.

## Table of Contents

<!-- toc:start -->
- [Stack](#stack)
- [Development](#development)
- [Checks](#checks)
- [Layout](#layout)
<!-- toc:end -->
## Stack

- [React](https://react.dev/) 19 and [React Router](https://reactrouter.com/)
- [Vite](https://vite.dev/) 8
- [Mantine](https://mantine.dev/) 9 — core, hooks, dates, notifications, form
- [Tabler Icons](https://tabler.io/icons)
- [TanStack Query](https://tanstack.com/query/latest)
- [Vitest](https://vitest.dev/) and Testing Library
- ESLint (flat config)

## Development

```sh
pnpm install
pnpm dev        # http://localhost:5173, proxies /api to the backend
```

Run the backend separately with `cargo loco start` from the repository root.
Vite reads `PORT` (default `5150`) for the proxy target.

## Checks

```sh
pnpm typecheck
pnpm lint
pnpm test
pnpm build
```

`pnpm build` type-checks and writes `dist/`. Loco serves that directory in
development and production, so rebuild after frontend changes when you are not
using the Vite dev server.

## Layout

```text
src/
  api/          typed fetch client, current-user hook
  auth/         login, registration, token storage, route guard
  components/   AppShell pieces, auth layout, empty states, quick capture
  lib/          small pure helpers and their tests
  pages/        Dashboard, Inbox, Today, Upcoming, All items, Completed,
                Settings, Integrations
  bindings/     generated TypeScript DTOs (do not edit)
  main.tsx      Mantine + Query providers and router
  routes.tsx    route table
  theme.ts      Mantine theme
  styles.css    global styles
```
