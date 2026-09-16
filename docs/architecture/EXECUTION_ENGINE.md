# EXECUTION_ENGINE.md — ZylCode Execution Engine

**Status: PARTIAL — shell/tools at R3, all device/browser backends PROPOSED**
**Rung: R3 (shell, filesystem, git, search) · R0 (browser, desktop, container, Android, iOS, cloud)**
**Phases: 7 (browser) · 10 (Android) · 11 (macOS/iOS worker)**
**Governing docs:** `ZYLCODE_ARCHITECTURE_V2.md` §3.5

> ⚠️ Shell-level execution exists. **Browser, desktop, container, Android, iOS and cloud
> backends do not.**

---

## 1. Responsibility

Provide controlled **eyes and hands**: shell, browser, desktop, containers, Android, iOS, cloud.

The Execution Engine is what allows ZylCode to *run* what it builds — and therefore what allows
the loop to close.

---

## 2. The Uniform Backend Contract

Every backend — present and future — implements the same contract. **No backend is a special
case.**

```rust
trait ExecutionBackend {
    fn id(&self) -> &str;
    fn capabilities(&self) -> BackendCapabilities;   // what this backend can do
    fn health(&self) -> BackendHealth;
    async fn execute(&self, job: Job, ctx: &ExecContext) -> JobResult;
    async fn observe(&self, handle: JobHandle) -> Observation;  // screenshot, logs, DOM…
    async fn terminate(&self, handle: JobHandle) -> Result<()>;
}
```

### 2.1 Why uniformity matters

The Mac worker (Phase 11) must be **another `ExecutionBackend`**, not a special-cased RPC path.
If it is special-cased, every future remote backend needs its own special case.

### 2.2 Capability advertisement

A backend declares what it can do. The Agent Kernel queries capabilities before planning. A
plan that requires an unavailable capability **fails closed at planning time**, not at execution
time.

---

## 3. Backends

| Backend | Status | Rung | Phase |
|---|---|---|---|
| **Shell / process** | FUNCTIONAL | R3 | 1A |
| **Filesystem** | FUNCTIONAL | R3 | 1A |
| **Git** | FUNCTIONAL | R3 | 1A |
| **Search** | FUNCTIONAL | R3 | 1A |
| **Browser** | PROPOSED | R0 | 7 |
| **Desktop / GUI** | PARTIAL | R1 | — |
| **Container** | PROPOSED | R0 | — |
| **Android (SDK / emulator / ADB)** | PROPOSED | R0 | 10 |
| **macOS / iOS (remote worker)** | PROPOSED | R0 | 11 |
| **Cloud** | PROPOSED | R0 | — |

> A computer-use module exists in the tree at partial quality. It is **R1** — observed, not
> verified — and must not be described as a working desktop backend.

---

## 4. Phase 7A — Browser Backend

**Capabilities**

launch · navigate · click · type · inspect DOM · inspect console · inspect network · screenshot ·
record interaction · run test workflows

**The loop this enables**

```
Implement → Launch → Observe → Interact → Detect problem → Repair → Reload → Retest
```

**That loop is crucial.** It is the first time ZylCode closes the loop without a human in it.

### 4.1 Observation model

Each observation is an **artifact** (Phase 6A) with a producer and a lifetime:

| Observation | Artifact kind |
|---|---|
| Screenshot | `SCREENSHOT` |
| Console log | `DOCUMENT` (attributed to the step) |
| Network trace | `API_RESPONSE` / `DOCUMENT` |
| DOM snapshot | `DOCUMENT` |
| Interaction recording | `VIDEO` |

### 4.2 Requirements

1. Every browser session is attributable to a **mission step**.
2. A hung browser must not hang the host — browser runs in a killable worker.
3. Network access is permission-gated and recorded.
4. A failed navigation produces an explicit failure, never a blank observation.

---

## 5. Phase 10 — Android Device Lab

**Capabilities:** Android Studio/SDK · emulator · ADB · physical devices · logcat · screenshots ·
install APK · launch · tap/type/swipe · inspect crash · Gradle build/test · release artifact.

> **The agent must be able to build and actually use the Android application.**

*"Compiles successfully" is R2. Using the application is R3.*

**Evidence required:** a captured `adb` session showing install → launch → interaction →
screenshot of the running app.

---

## 6. Phase 11 — macOS / iOS Worker

Windows cannot provide native iOS simulator/Xcode execution. The solution is a remote backend,
not an exception.

```
ZylCode Windows/Linux
        │
        │ secure worker protocol
        ↓
ZylCode Mac Worker
        │
        ├── Xcode
        ├── Simulator
        ├── xcodebuild
        ├── signing
        ├── devices
        └── TestFlight
```

### 6.1 Requirements

| # | Requirement |
|---|---|
| 1 | **Authenticated transport** — mutual authentication, no implicit trust |
| 2 | **Capability advertisement** — the worker declares what it can do |
| 3 | **Job isolation** — one job cannot observe another |
| 4 | **Artifact return** — outputs come back as artifacts with provenance |
| 5 | **Attribution** — every job carries `mission_id` / `step_id` |
| 6 | **Fail closed** — an unreachable worker fails the job; it does not silently skip |
| 7 | **No secrets on the wire** — signing credentials stay on the worker |

**Point 7 matters:** the Mac worker holds signing identities. They must never be transmitted to
the host, and the host must never be able to extract them.

---

## 7. Sandboxing and Resource Limits

| Concern | Approach |
|---|---|
| Process isolation | worker per job class |
| Filesystem scope | declared paths only |
| Network | off by default; permission-gated |
| CPU / memory | per-worker limits |
| Wall clock | per-job timeout |
| Killability | any worker can be terminated without harming the host |

**The host must survive any worker death.** This is a hard requirement, not a robustness nicety.

---

## 8. What the Execution Engine Does *Not* Do

| Not its job | Owner |
|---|---|
| Decide whether an action is permitted | Permissions |
| Interpret results | Proof Engine |
| Decide what to run next | Agent Kernel |
| Store outputs | Artifact Bus |
| Own project state | Project System |

---

## 9. Benchmark / Acceptance

| Backend | Criterion | Rung |
|---|---|---|
| Shell | (existing) real effects with ledger entries | R3 ✔ |
| Browser | full implement→launch→observe→repair→retest loop, no human intervention | R3 |
| Browser | hung browser does not hang the host | R3 (fault injection) |
| Android | captured `adb` session: install, launch, interact, screenshot | R3 |
| Mac worker | job submitted from Windows builds and runs on a simulator; artifacts returned; attributable | R3 |
| Mac worker | signing credentials never leave the worker | R3 (verified by inspection) |
| Any | unreachable backend fails the job explicitly | R3 (negative test) |

---

## 10. Anti-Requirements

- Do not special-case any backend; all implement the same contract.
- Do not let a worker death affect the host.
- Do not transmit signing credentials.
- Do not treat "compiles" as "runs".
- Do not silently skip an unavailable backend.
- Do not describe the partial computer-use module as a working desktop backend.
