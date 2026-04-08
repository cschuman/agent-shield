# Agent Shield — Prerequisites & Decision Register

## Stack Decisions

| Capability | Decision | Rationale | Lock-in Risk |
|---|---|---|---|
| **CLI language** | Rust | Single binary distribution, performance, already built | Low — CLI is standalone |
| **Dashboard framework** | SvelteKit 2 | Corey's primary framework, SSR + client, proven at scale | Low — standard web framework |
| **Backend API** | SvelteKit full-stack (API routes) | Avoid second server for MVP, migrate to Go if needed at scale | Low — standard REST, easy to extract |
| **Database** | Neon Postgres | Serverless, branching for dev, Corey has existing account + experience | Medium — Postgres is portable, Neon-specific features optional |
| **ORM** | Kysely | Type-safe, Corey uses in MSL Briefing, raw SQL escape hatch | Low — generates standard SQL |
| **Auth** | Better Auth | Library-based (not service), Corey uses in MSL Briefing, supports GitHub OAuth | Low — owns the auth data |
| **Payments** | Stripe | Industry standard, metered billing support, Checkout + Customer Portal | Medium — Stripe is sticky but well-documented |
| **Hosting (app)** | Fly.io | Docker-based, Corey has existing apps there, good Postgres proximity | Low — Docker image runs anywhere |
| **Hosting (static)** | Netlify | Already deployed, auto-deploy on push, CDN | Low — static files are portable |
| **Background jobs** | pg-boss | Postgres-backed, Corey uses in MSL Briefing, no extra infrastructure | Low — just Postgres |
| **PDF generation** | Typst or Headless Chrome | CLI-side generation for free tier, server-side for dashboard | Low |
| **File storage** | S3-compatible (Tigris on Fly.io) | Scan result storage, PDF reports | Low |

## Dependency Budget: 5 External Services Max

1. **Neon Postgres** — database
2. **Fly.io** — application hosting
3. **Stripe** — payments
4. **Netlify** — static site hosting
5. **Tigris/S3** — file storage (scan results, PDFs)

No additional services until one of these proves insufficient.

## Key Principles

- **Docker image that runs anywhere** > platform-specific deploy
- **"Can Postgres do this?"** before adding any new dependency
- **Library-based auth** > service-based auth (Better Auth, not Clerk/Auth0)
- **Payment integration is a launch requirement**, not post-launch
- **RLS from day 1** — multi-tenancy via Postgres Row Level Security

## Accounts & CLI Checklist

| Service | Account Status | CLI |
|---|---|---|
| Neon | Existing account | `neon` CLI installed |
| Fly.io | Existing account | `fly` CLI installed |
| Stripe | Needs setup | `stripe` CLI needed |
| Netlify | Existing account, site deployed | `netlify` CLI installed |
| GitHub | Existing account | `gh` CLI installed |

## Environment Variables Inventory

```env
# Database
DATABASE_URL=postgresql://...@...neon.tech/agent_shield

# Auth (Better Auth)
BETTER_AUTH_SECRET=<random-32-chars>
GITHUB_CLIENT_ID=<from-github-oauth-app>
GITHUB_CLIENT_SECRET=<from-github-oauth-app>

# Stripe
STRIPE_SECRET_KEY=sk_live_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PRICE_TEAM=price_...
STRIPE_PRICE_BUSINESS=price_...
STRIPE_PRICE_ENTERPRISE=price_...

# App
PUBLIC_APP_URL=https://app.agentshield.dev
PUBLIC_MARKETING_URL=https://agentshield.dev

# Storage
S3_ENDPOINT=https://fly.storage.tigris.dev
S3_ACCESS_KEY=...
S3_SECRET_KEY=...
S3_BUCKET=agent-shield-reports
```
