# Deployment and CD — options

**Date:** 2026-08-21 · **Owner:** architect, at the owner's request ·
**Status:** proposal, nothing settled

The ask: somewhere cheap — Fly.io or Cloudflare by preference — with the app
reachable by the owner and nobody else, and a deploy that happens on its own
when `trunk` goes green.

Nothing here is a decision. `docs/decisions.md` is untouched; if the owner
picks a shape, the row gets appended then.

## What the codebase has already decided for us

Four facts in the tree rule most of the option space out before preferences
enter into it.

**1. One process, one file, one writer.** `T-sqlite-sqlx`: a single SQLite
file in WAL mode, opened from the local filesystem. That needs a *persistent
disk attached to exactly one running instance*. It rules out every
scale-to-N runtime, every ephemeral-filesystem runtime, and every
edge-replicated one. There is no version of this that runs in two places at
once.

**2. The app has no authentication and is not going to get any.**
`R-multi-tenancy` rejects accounts and auth outright, `D-single-user` is why.
Every route in `platform/app.rs` — including `POST /captures` and
`POST /captures/{id}/triage` — is unauthenticated and will stay that way.
**So "only reachable by me" is entirely a property of the network in front of
the process, and the app can never be the backstop.** That is the single
biggest constraint on this choice: a front door that fails open exposes a
writable database to the internet, and nothing in the process will object.

**3. The artifact is already right.** `release_binary.feature` gates a static
musl binary, and `platform/assets.rs` embeds HTMX, the stylesheet and the
font into it. The deploy artifact is one file with no runtime dependencies —
`FROM scratch` works, the image is a few tens of MB, and cold start is
milliseconds. That makes scale-to-zero genuinely viable rather than a
latency tax.

**4. Migrations run themselves.** `main.rs` runs `connect_and_migrate` before
binding, on both `migrate` and `serve`. CD needs no migration step and no
`release_command`. The corollary bites on rollback, though — see
*Rolling back* below.

One more, smaller: **`--addr` defaults to `127.0.0.1:8080`.** In a container
that means nothing outside can ever reach it. Pass `--addr '[::]:8080'` —
not `0.0.0.0` — anywhere on Fly, because Fly's private networking
(`.internal`, flycast) is IPv6-only and a v4-only bind is invisible to it.
A dual-stack `[::]` listener accepts both.

## Two axes, chosen separately

The recurring mistake here is treating "where it runs" and "who can reach it"
as one decision. They are independent, and the second one is the one that
matters.

### Axis 1 — where the process runs

| | Fits SQLite-on-disk | ~cost/mo | Notes |
|---|---|---|---|
| **Fly.io machine + volume** | yes | ~$2 compute + ~$0.15/GB volume, under a ~$5 plan minimum | Scale-to-zero works; volume pins it to one region and one machine |
| **Hetzner CX22 (or similar VPS)** | yes | ~€4 | 2 vCPU / 4 GB / 40 GB — an order of magnitude more machine per euro; you own the patching |
| **Home box / Pi** | yes | £0 + power | Availability is your home internet's availability |
| **Cloudflare Workers + D1** | **no** | — | See below |
| **Cloudflare Containers** | not really | — | Container disk is not durable in the way a WAL SQLite file needs |

Prices are mid-2026 list and drift; check before committing.

**Cloudflare as the runtime is a rewrite, not a deploy target.** Workers has
no filesystem, no `tokio`, and no `sqlx`; D1 is a queryable HTTP-fronted
service that happens to speak SQLite dialect, not a file you open. The
sqlx migration runner, the WAL, the connection pool and `platform/db.rs`
would all go. `T-sqlite-sqlx` argues a Postgres swap stays mechanical
because every query is behind a capability's `store.rs` — a D1 swap is *not*
mechanical in that sense, because it changes the shape of every call site
from "await a pooled connection" to "batch statements over HTTP". If the
owner wants this evaluated seriously it deserves its own brief and probably
an `R-` row. **Cloudflare's real role here is the front door, not the
runtime** — which is the next axis, and there it is excellent.

### Axis 2 — who can reach it

This is the one that answers the actual requirement.

| | How it keeps others out | Client needed | Cost | Weakness |
|---|---|---|---|---|
| **Tailscale** | No public address exists at all | Tailscale app on each device | free (personal) | Any device on the tailnet reaches it |
| **Cloudflare Tunnel + Access** | Public hostname, but every request needs a valid Access JWT; unauthenticated ones never reach the origin | none — any browser | free under 50 users | Cloudflare terminates TLS and sees plaintext; extra process in the container |
| **Fly private networking (flycast + WireGuard)** | No public IP allocated | WireGuard profile per device | free, built in | Per-device profile setup; `.internal` DNS needs Fly's resolver |
| **mTLS client certs** | No cert, no handshake | cert installed per device | free | Certificate management by hand; awkward on iOS |
| **Reverse-proxy basic auth** | password | none | free | A replayable credential in front of a writable DB — not sufficient alone |

Whichever is chosen, **the process itself must bind loopback or the private
interface, never a public one**, so that a misconfigured proxy is a broken
deploy rather than a silent exposure. On Fly that means allocating no public
IP; on a VPS it means `--addr 127.0.0.1:8080` with the proxy in front.

## Three recipes worth costing

### R1 — Fly, no public IP, private networking (fewest moving parts)

One machine, one volume, `fly ips allocate-v6 --private` (a flycast address)
and **no public IPv4 or IPv6 at all**. Reach it over Fly's WireGuard: `fly
proxy 8080:8080` from the laptop, or an imported WireGuard profile on the
phone pointing at `http://trellis.internal:8080`.

- Nothing is on the internet — not "protected", *absent*. No third party in
  the request path, no TLS termination anyone else performs.
- No sidecar. The image is `FROM scratch` + the binary, exactly the artifact
  `release_binary.feature` already gates.
- Flycast routes via the Fly proxy, so **autostop/autostart still works** and
  the machine can sleep at zero cost between uses. A direct-to-machine
  address would not wake it.
- Cost: the plan minimum, effectively ~$5/mo.
- Against it: a WireGuard profile per device, and no public HTTPS hostname to
  hand a webhook later (`R-gcal-webhooks` already assumed we don't have one).

### R2 — Fly (or VPS) + `cloudflared` + Cloudflare Access

App binds loopback. `cloudflared` holds an outbound tunnel to Cloudflare; no
inbound port is open anywhere. A Cloudflare Access policy on the hostname
allows exactly one identity — email OTP to the owner's address, or Google
SSO — and everything else is rejected at Cloudflare's edge.

- **Any browser, any device, no client software.** This is the one that makes
  the phone pleasant, which matters for a capture box.
- It also gives a real public HTTPS hostname, which is what `R-gcal-webhooks`
  said self-hosting lacked. A single `/webhooks/*` path can be excluded from
  the Access policy later without opening anything else.
- Free under 50 users; a domain is ~$10/yr.
- Against it: Cloudflare terminates TLS and can see plaintext, and
  `cloudflared` is a **second process in the container**, which fights the
  single-static-binary shape — it needs a tiny supervisor or a second Fly
  machine, and either is a real complication to the image.

### R3 — Hetzner VPS + systemd + Tailscale

`trellis serve --addr 127.0.0.1:8080` under systemd, Tailscale on the host,
reached at `http://trellis-vps:8080` on the tailnet. Optionally Caddy in
front for TLS via Tailscale's certs.

- Cheapest per unit of machine by a wide margin, and the deploy is `scp` a
  binary and `systemctl restart` — no image, no registry.
- Same box can hold anything else later.
- Against it: you own kernel updates, disk, and the backup story; and CD
  needs an SSH key in GitHub Actions rather than a scoped deploy token.

**Recommendation:** **R1 for now, R2 when phone access starts to matter.**
R1 is the fewest new concepts, has no third party in the path, and keeps the
image identical to the artifact CI already builds. The moment "capture on my
phone without opening a VPN first" becomes the thing that decides whether the
product gets used, R2 is worth the sidecar — and it composes with R1 rather
than replacing it, since the app never had a public IP either way.

## Continuous deployment

The shape is the same for all three, and it hangs off the existing workflow
rather than duplicating it.

```
push to trunk ──▶ gate ──┐
                         ├──▶ deploy   (needs: [gate, quality])
              ──▶ quality ┘
```

Points that are specific to this repo:

- **`needs: [gate, quality]`, not just `gate`.** The acceptance suite and the
  analyzers are the bar the project actually holds itself to; deploying on
  the fast job alone would ship things `quality` would have caught, ~20
  minutes later, to the machine the owner is dogfooding on.
- **Reuse the musl binary `gate` already builds.** Upload it with
  `actions/upload-artifact` and have `deploy` download it into a
  `FROM scratch` image. Building Rust again inside a Docker layer, or on
  Fly's remote builder, is ten minutes of paying twice for one artifact.
- **Guard it:** `if: github.ref == 'refs/heads/trunk' && github.event_name == 'push'`,
  plus a `workflow_dispatch` for manual redeploys.
- **`concurrency: { group: deploy-trunk, cancel-in-progress: false }`.** Note
  this differs from the CI concurrency rule at the top of `ci.yml`, and for
  the same underlying reason: two deploys racing onto one machine with one
  volume is worse than one deploy waiting.
- **Secrets:** a deploy-scoped Fly token (`fly tokens create deploy`) as
  `FLY_API_TOKEN`, never a personal one. For R3, a dedicated SSH key with a
  forced command.
- **Health check:** `GET /` renders the inbox and therefore touches the
  database, so it is an honest readiness probe — a machine that boots but
  cannot open the volume fails it. No new endpoint needed.
- **One machine, and keep it that way.** `fly scale count 1` and a deploy
  `strategy = "immediate"`. A volume cannot be shared, and a second machine
  would either fail to start or — worse — start against a second, empty
  volume. Deploys cost a few seconds of downtime, which for one user is not a
  cost at all.
- If the repo is private, note the Actions minutes: `quality` is budgeted at
  45 minutes and every trunk push would now also pay for a deploy job.

### Rolling back

`fly releases` + `fly deploy --image <previous>` rolls the binary back in
seconds — but **`T-migrations-append-only` means the database does not roll
back with it.** If the release that is being reverted added a migration, the
old binary meets a schema it does not know. In practice that is usually
harmless (added tables and columns it ignores), and occasionally is not.
Rolling back across a migration boundary should mean *restore the snapshot
too*, and that is worth writing down before it is needed at speed.

### Backups

`T-sqlite-sqlx` gave us a single file, which makes this cheap. Two stages:

1. **Now:** Fly's automatic daily volume snapshots (free, ~5 day retention).
   Enough while the data is a few days of captures.
2. **Once the data is worth something:** Litestream replicating the WAL to
   Cloudflare R2 — continuous, point-in-time, and R2 charges no egress, so a
   restore costs nothing. That is a third process in the container, so it
   lands naturally alongside R2's `cloudflared` rather than before it.

`.gitignore` already excludes `*.db`, `*.db-wal`, `*.db-shm` — the database
must never be an image layer, and it will not become one by accident.

## What this needs from the owner

1. **R1, R2 or R3** — really: is browser-anywhere worth a second process in
   the container, and is a third party terminating TLS acceptable?
2. **Domain or no domain.** R2 needs one; R1 and R3 do not.
3. Whether deployment gets QA procedures of its own. Every other
   user-facing behaviour in this repo has a `.feature` and a `qa/*.md`
   driving it through a real interface, and "the deployed thing is reachable
   by me and by nobody else" is exactly the kind of claim this project does
   not take on trust. A `qa/deployment.md` asserting *unauthenticated request
   from off-tailnet is refused* would be the honest version of that.
