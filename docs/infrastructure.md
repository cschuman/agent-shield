# Agent Shield — Infrastructure Plan

## Architecture

```
                    ┌─────────────────┐
                    │    Netlify      │
                    │  (Marketing)    │
                    │  agentshield.dev│
                    └─────────────────┘

                    ┌─────────────────┐
     ┌──────────────│    Fly.io       │──────────────┐
     │              │  (App Server)   │              │
     │              │ app.agentshield │              │
     │              │     .dev        │              │
     │              └─────────────────┘              │
     │                                               │
     ▼                                               ▼
┌─────────────────┐                    ┌─────────────────┐
│  Neon Postgres  │                    │   Tigris S3     │
│  (Database)     │                    │  (File Storage) │
│  us-east-1      │                    │                 │
└─────────────────┘                    └─────────────────┘
```

## Hosting

### Application Server (Fly.io)
- **App:** SvelteKit Node.js adapter
- **Region:** iad (us-east-1, same as Neon)
- **Machine:** shared-cpu-1x, 512MB RAM (start small)
- **Scaling:** Scale to 2 machines at 50+ concurrent users
- **Deploy:** `fly deploy` from GitHub Actions on push to main

### Database (Neon Postgres)
- **Plan:** Free tier initially, Scale plan at launch ($19/mo)
- **Region:** us-east-1
- **Branching:** Dev branch for development, main for production
- **Connection:** Pooled connection string (PgBouncer)
- **Autoscale:** 0.25-2 CU (compute units)

### Static Site (Netlify)
- **Content:** Marketing site (landing, pricing, docs)
- **Deploy:** Auto-deploy on push to main
- **Domain:** agentshield.dev

### File Storage (Tigris S3 on Fly.io)
- **Use:** PDF reports, scan result archives
- **Access:** Private, pre-signed URLs for downloads
- **Retention:** Matches org tier retention policy

## Cost Projections

### At Launch (0 customers)
| Service | Cost/mo |
|---|---|
| Fly.io (1 machine, shared-cpu-1x) | $5 |
| Neon (Free tier) | $0 |
| Netlify (Free tier) | $0 |
| Tigris S3 (minimal) | $1 |
| Domain (agentshield.dev) | ~$1 |
| **Total** | **~$7/mo** |

### At 10 Customers (~$15K MRR)
| Service | Cost/mo |
|---|---|
| Fly.io (2 machines, 1GB each) | $30 |
| Neon (Scale plan) | $19 |
| Netlify (Free tier still) | $0 |
| Tigris S3 | $5 |
| Stripe fees (2.9% + $0.30) | ~$450 |
| **Total** | **~$504/mo (3.4% of revenue)** |

### At 100 Customers (~$200K MRR)
| Service | Cost/mo |
|---|---|
| Fly.io (4 machines, 2GB each) | $120 |
| Neon (Scale plan, higher CU) | $69 |
| Netlify (Pro) | $19 |
| Tigris S3 | $30 |
| Stripe fees | ~$6,000 |
| **Total** | **~$6,238/mo (3.1% of revenue)** |

## CI/CD Pipeline

```yaml
# .github/workflows/deploy.yml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm ci
      - run: npm run test
      - run: npm run build

  deploy:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: superfly/flyctl-actions/setup-flyctl@master
      - run: flyctl deploy --remote-only
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```

## Day-1 Deployment Checklist

```
Bring-up sequence (order matters):

1. [ ] Register domain: agentshield.dev
2. [ ] Create Neon project + database
3. [ ] Run initial migration (0001_initial_schema.sql)
4. [ ] Create Fly.io app
5. [ ] Set Fly.io secrets (DATABASE_URL, BETTER_AUTH_SECRET, etc.)
6. [ ] Deploy SvelteKit app to Fly.io
7. [ ] Configure custom domain on Fly.io (app.agentshield.dev)
8. [ ] Set up Stripe products + prices
9. [ ] Configure Stripe webhook to app.agentshield.dev/api/webhooks/stripe
10. [ ] Deploy marketing site to Netlify
11. [ ] Configure Netlify domain (agentshield.dev)
12. [ ] Create GitHub OAuth app (for auth)
13. [ ] Test full flow: signup → generate API key → CLI scan → upload → dashboard
14. [ ] Publish CLI to crates.io (cargo publish)
```

## Backup & Recovery

| What | Strategy | RPO | RTO |
|---|---|---|---|
| Database | Neon PITR (point-in-time recovery), 7 days | <1 min | <5 min |
| Scan results (S3) | Tigris replication (built-in) | ~0 | <1 min |
| Application code | Git (GitHub) | 0 | <5 min (redeploy) |
| Stripe data | Stripe is the source of truth | N/A | N/A |

## Monitoring Strategy

| Layer | Tool | Cost |
|---|---|---|
| Uptime | Fly.io built-in health checks | Free |
| Application errors | Sentry (free tier, 5K events/mo) | $0 |
| Database | Neon dashboard + pg_stat_statements | Free |
| Logs | Fly.io log drain → stdout | Free |
| Billing | Stripe Dashboard + webhook event log | Free |

No external monitoring services until traffic justifies it. Postgres + stdout + Sentry covers MVP.
