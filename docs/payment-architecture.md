# Agent Shield — Payment Architecture

## Revenue Model

| Tier | Price | Agents | Billing |
|---|---|---|---|
| **Free (CLI)** | $0 | Unlimited local | No account needed |
| **Free (Dashboard)** | $0 | 3 agents, 1 scan | Account required |
| **Team** | $1,500/mo | 25 included, $60/agent overage | Monthly, credit card |
| **Business** | $5,000/mo | 100 included, $50/agent overage | Monthly or annual, credit card or invoice |
| **Enterprise** | $15,000/mo | Unlimited | Annual, invoice |

## Stripe Products & Prices

```
Product: Agent Shield Team
  Price: $1,500/month (recurring)
  Metered component: $60/agent/month (for overage above 25)

Product: Agent Shield Business
  Price: $5,000/month (recurring)
  Metered component: $50/agent/month (for overage above 100)

Product: Agent Shield Enterprise
  Price: $15,000/month (recurring)
  No metered component (unlimited)
```

## Billing Lifecycle State Machine

```
                    ┌──────────┐
                    │  trialing │ (14-day free trial, credit card required)
                    └─────┬────┘
                          │ trial ends + card valid
                          ▼
    ┌─────────────── active ◄───────────────┐
    │                  │                     │
    │ payment fails    │ user cancels        │ user reactivates
    │                  │                     │
    ▼                  ▼                     │
┌──────────┐    ┌────────────┐              │
│ past_due  │    │  canceled  │──────────────┘
│ (grace    │    │            │
│  3 days)  │    └────────────┘
└─────┬────┘
      │ 3 days, still failed
      ▼
┌──────────┐
│ suspended │  (read-only access, no new scans)
└─────┬────┘
      │ 30 days, still failed
      ▼
┌──────────┐
│  churned  │  (data retained per retention policy, account locked)
└──────────┘
```

## Metered Billing Flow

```
1. Scan uploaded via CLI → POST /api/v1/scans
2. Server counts unique agents in scan
3. Server checks: agent_count > tier.included_agents?
4. If yes: overage = agent_count - tier.included_agents
5. Hourly job: report overage usage to Stripe
   stripe.subscriptionItems.createUsageRecord(
     subscriptionItemId,
     { quantity: overage_agents, timestamp: now }
   )
6. Stripe includes overage on next invoice
```

## Stripe Integration Points

### Checkout Flow
```
User clicks "Start Team Trial" on pricing page
  → POST /api/billing/checkout
  → Server creates Stripe Checkout Session
    - mode: 'subscription'
    - line_items: [team_price]
    - subscription_data: { trial_period_days: 14 }
    - success_url: /dashboard?session_id={CHECKOUT_SESSION_ID}
    - cancel_url: /pricing
  → Redirect to Stripe Checkout
  → User enters payment info
  → Stripe redirects to success_url
  → Webhook: checkout.session.completed
  → Server: create organization, set tier, link stripe_customer_id
```

### Customer Portal
```
User clicks "Manage Billing" in settings
  → POST /api/billing/portal
  → Server creates Stripe Customer Portal Session
    - customer: org.stripe_customer_id
    - return_url: /settings/billing
  → Redirect to Stripe Customer Portal
  → User can: update card, cancel, view invoices
  → Webhook events update local state
```

### Webhook Handling

```typescript
// /api/webhooks/stripe/+server.ts

const HANDLED_EVENTS = [
  'checkout.session.completed',      // New subscription
  'customer.subscription.updated',   // Plan change, trial end
  'customer.subscription.deleted',   // Cancellation
  'invoice.paid',                    // Successful payment
  'invoice.payment_failed',         // Failed payment
] as const;

// For each event:
// 1. Verify Stripe signature
// 2. Extract subscription/customer data
// 3. Update organization.tier + organization.stripe_subscription_id
// 4. Log to billing_event table (append-only)
// 5. Return 200
```

### Tier Enforcement

```typescript
// Middleware on scan upload endpoint

async function enforceTier(orgId: string, agentCount: number) {
  const org = await getOrganization(orgId);

  if (org.tier === 'free' && agentCount > 3) {
    throw new TierLimitError('Free tier limited to 3 agents. Upgrade to Team.');
  }

  if (org.tier === 'team' && agentCount > 25) {
    // Allow but report overage to Stripe
    await reportOverage(org, agentCount - 25);
  }

  if (org.tier === 'business' && agentCount > 100) {
    await reportOverage(org, agentCount - 100);
  }

  // Enterprise: unlimited, no enforcement
}
```

## Upgrade / Downgrade Rules

| Transition | Behavior |
|---|---|
| Free → Team | Stripe Checkout, 14-day trial |
| Free → Business | Stripe Checkout, 14-day trial |
| Team → Business | Proration (credit for unused Team days) |
| Business → Team | Downgrade at period end (no immediate change) |
| Any → Enterprise | Contact sales → manual Stripe subscription creation |
| Any → Free (cancel) | Access until period end, then downgrade to free limits |

## Revenue Recognition

- Subscription revenue: recognized monthly
- Overage revenue: recognized in the billing period incurred
- Annual contracts (Business/Enterprise): recognized monthly (1/12 per month)
- Trial periods: no revenue recognized during trial

## Key Implementation Notes

1. **Stripe is the source of truth for subscription state.** Local `organization.tier` is a cache, synced via webhooks.
2. **Never trust client-side tier checks.** All tier enforcement happens server-side.
3. **Metered usage is reported hourly**, not per-scan. This smooths out burst usage.
4. **Free tier does NOT require Stripe.** No credit card, no customer record. Stripe only involved when upgrading.
5. **API keys are tied to organizations, not users.** Billing is per-org, not per-seat.
