# BETA TEST GUIDE

This guide is for an **external developer** testing the ZylCode × N-ATLAS Developer Bridge.

The competition requires **at least two external developers** outside the core team to actually
test the toolkit. This guide is what you work from. You should not need verbal coaching.

---

## Before you start

You will need:

- the repository checked out at `competition/natlas-2026`;
- Ollama installed and the N-ATLAS model served (see [NATLAS_SETUP.md](NATLAS_SETUP.md));
- Node.js ≥ 18;
- a small repository to work on.

**Time:** about 60–90 minutes for all ten tests.

**You are testing the toolkit, not being tested.** Confusion you experience is a finding, not a
failure. Record it.

---

## Rules of engagement

1. **Do not fabricate anything.** If a test fails, record that it failed.
2. **Do not paste secrets** into the evidence record. Redact your Hugging Face token.
3. **Stop and report** if anything behaves unsafely (e.g. executes without approval).
4. If you use the offline demo or a test double, **say so** — it does not count as N-ATLAS evidence.

---

## The ten tests

### TEST A — Installation / onboarding
Can you get from the documentation to a running toolkit **without asking anyone**?
Record: which document you followed, where you got stuck, how long it took.

### TEST B — Connection
Can you prove the runtime is **genuine N-ATLAS**?
Record: the output of `curl http://127.0.0.1:11434/api/tags`, and the model identity the toolkit
reports. It must be the identity the **server** reports.

### TEST C — Repository workflow
Can you open/select a small repository?
Record: the repository you used, its size, and how you selected it.

### TEST D — Developer task
Can you give a bounded software-engineering instruction?
Record: the **exact** instruction you gave, and in which language.

### TEST E — N-ATLAS interpretation
Does a genuine response enter the structured intent boundary?
Record: the raw response, and whether it satisfied the intent contract.

### TEST F — Approval
Can you understand and approve/reject the proposed action?
Record: what you understood the proposal to be, and what you chose.

### TEST G — Controlled execution
Does the approved action execute through the intended tool path?
Record: what executed, and what did **not**.

### TEST H — Verification
Does the toolkit run a real test/verification and expose the **actual** result?
Record: the command and the real exit code.

### TEST I — Evidence
Can you locate the evidence record?
Record: the evidence id and the fields you found.

### TEST J — Usability
Can you explain what succeeded, what failed, and what confused you?
Record: verbatim, in your own words.

---

## Multilingual coverage

Where practical, testers are asked to cover different languages so that the multilingual claim is
measured rather than assumed. A suggested (not mandatory) split:

| Tester | Languages |
|---|---|
| Tester 1 | English (Nigerian-accented) + Yoruba |
| Tester 2 | Hausa + Igbo |

> **Do not translate a Nigerian-language prompt into English before sending it.** That would
> invalidate the direct-language claim. Send the prompt as-is and record what N-ATLAS actually did
> with it — including if it performed poorly.

If N-ATLAS performs badly in a language, **record that**. The competition checkboxes are worth
less than an honest result.

---

## Recording your results

Fill in **[../BETA_TEST_EVIDENCE_TEMPLATE.md](../BETA_TEST_EVIDENCE_TEMPLATE.md)**.

One row per test, per tester. Include:

- tester id and name (with consent);
- **external vs core-team** status;
- technical background;
- environment (OS, RAM, GPU, runtime version);
- repository and language;
- the exact task;
- start/end timestamps and measured elapsed time;
- the N-ATLAS evidence id;
- outcome (success / partial / failure);
- verification result;
- rating;
- observations and verbatim feedback;
- issues found and their resolution;
- consent for anonymised use of the evidence.

---

## What happens to your feedback

- It is recorded verbatim.
- It informs the competition submission's **External Beta Validation** section.
- Nothing is rewritten to sound better than it was.
- Core-team members are **never** counted as external testers.

---

## Reporting a problem

Open an issue on the public repository, or contact the maintainers via the details in
`SECURITY.md` / the repository README. For a security issue, do **not** open a public issue.
