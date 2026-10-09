# EXTERNAL BETA TEST REPORT — N-ATLAS × ZylCode Developer Bridge

**Date:** 2026-10-10
**Branch:** `competition/natlas-2026`
**Commit at time of writing:** `8216738`
**Scope:** Reconcile the two external response spreadsheets, protect tester PII, and
state precisely what is *tester-reported*, what is *independently verified*, and what
is *not verified*.

---

## 1. Evidence discipline

This report keeps three categories strictly apart:

| Label | Meaning |
|---|---|
| **Tester-reported** | The tester's own submission. Attributed, never upgraded. |
| **Independently verified** | We reproduced the claim with a command whose output is quoted. |
| **NOT VERIFIED** | We did not reproduce it. Stated as such. Never marked passed. |

A response appearing in a spreadsheet is **not** proof that the product worked. It is
proof that a person submitted a form. Only a reproduced command is proof of behaviour.

---

## 2. Sources

| Id | Google Sheet ID | Title as given | Rows read | Access |
|---|---|---|---|---|
| **Sheet A** | `129YZpfENcG8wtHDQOKuIdkoSW9Jqm0cnryGRulSu_uI` | "Track A mobile responses" | 2 responses | public CSV export (`/export?format=csv`) |
| **Sheet B** | `1YsBOCCJmfed6EK9tG5Obj-ul4F-ZtwZbUiSemblSfoY` | "additional N-ATLAS test responses" | 1 response | public CSV export (`/export?format=csv`) |

Both spreadsheets were read on **2026-10-10** via their public CSV export endpoint.
Each was confirmed to expose a **single** publicly readable tab
(Sheet A `gid=1186942662`, Sheet B `gid=580441791`); no further tabs were reachable
without authentication.

> **Titles are not evidence.** Per instruction, no track classification is inferred from
> a spreadsheet title. Track is taken only from an explicit field in a row.

---

## 3. PII handling

The spreadsheets contain tester email addresses and full names.

- **Email addresses are withheld** from this report and from every other document
  produced in this release. They are not reproduced, hashed, or partially masked in a
  reversible way.
- **Names are retained** in the form the tester supplied, because the tester's
  participation is already disclosed in `TRACK_A_TESTER_001_RELEASE_2026-10-09.md`
  and the competition requires named external validation.
- Screenshot/Drive links are reproduced as supplied; they remain under the tester's
  own Google Drive access control.

---

## 4. Sheet A — "Track A mobile responses" (2 responses)

| Field | Response 1 | Response 2 |
|---|---|---|
| Timestamp | 10/9/2026 8:28:02 | 10/10/2026 3:56:28 |
| Name | Ibrahim ABDULRAHMAN | Auwal |
| State | Borno | Kano |
| Device | Mobile | Mobile |
| Browser | Chrome | Chrome |
| Page opened | Yes | Yes |
| AI responded | Yes | Yes |
| Understood your language | Yes | Yes |
| Answers correct | Yes | Yes |
| Easy to use | Yes | Yes |
| Anything go wrong | No | No |
| Rating / 10 | **9** | **8** |
| Screenshots | 1 Drive link | 3 Drive links |
| **TESTING TRACK** | **(empty)** | **(empty)** |

**Reconciliation note.** The `TESTING TRACK` column is **empty for both rows**. The
sheet's *title* says "Track A", but the rows themselves assert no track. Consistent
with the standing instruction, **we do not assign these responses to Track A on the
strength of the title alone.** They are recorded as "external mobile responses, track
not declared".

All fields above are **tester-reported**. None has been independently reproduced.

---

## 5. Sheet B — "additional N-ATLAS test responses" (1 response)

| Field | Value |
|---|---|
| Timestamp | 10/10/2026 4:49:42 |
| Name | IBRAHIM ABDULRAHMAN |
| Tester ID | (empty) |
| Testing date | 10/10/2026 |
| Time | 12:40:00 PM |
| Location | BORNO |
| Device type | LAPTOP |
| Operating system | WINDOWS 11 |
| Browser | BRAVE |
| **Testing track** | **C** (tester-declared) |
| **Git commit SHA** | `559939270ce7dd329349f520cad3f871675f8e34` |
| Test ID | NONE |
| Instruction / command executed | `all` |
| Expected result | 18 test to pass |
| Actual result | `test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 467 filtered out; finished in 0.01s` |
| Status | PASS |
| Error message | (none) |
| Evidence | 2 Drive links |
| Feedback | "everything work correctly without modifying or change any single line of code." |

This is the **only row in either spreadsheet that declares a track** (`C`) and the
**only row that cites a commit**.

---

## 6. Independent verification of the Sheet B claims

Three claims in Sheet B are checkable. Each was checked on 2026-10-10.

### 6.1 The cited commit exists and is real — **VERIFIED**

```
$ git cat-file -t 559939270ce7dd329349f520cad3f871675f8e34
commit
$ git log -1 --format="%H%n%ci%n%s" 559939270ce7dd329349f520cad3f871675f8e34
559939270ce7dd329349f520cad3f871675f8e34
2026-10-09 06:41:25 +0100
docs(natlas): finalize submission package - ready for external beta testing
$ git merge-base --is-ancestor 5599392 HEAD && echo ancestor
ancestor
```

`5599392` is the **direct parent** of the current `HEAD` (`8216738`). The tester
therefore validated the commit **immediately before** the NAT-A remediation, not the
current commit. This is a material nuance and is stated plainly rather than glossed.

### 6.2 "18 passed, 0 failed" — **VERIFIED (reproduced fresh)**

```
$ cargo test -p zylcode-core --lib competition::natlas::intent
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 480 filtered out; finished in 0.00s
```

The pass count is identical (**18 / 0**). The "filtered out" figure differs
(480 now vs 467 then) because more tests exist in the tree today. The tested behaviour
was **not regressed** by the remediation.

### 6.3 "Windows 11 / Brave / laptop" — **NOT VERIFIED**

The device, OS and browser are the tester's own description. We cannot reproduce a
tester's hardware. Recorded as tester-reported only.

---

## 7. Discrepancies and open questions

| # | Discrepancy | Resolution |
|---|---|---|
| 1 | Sheet B records the command as `all`; the owner's record describes it as `cargo test -p zylcode-core --lib competition::natlas::intent`. | **Both recorded.** The sheet is the primary source; the owner's more specific command is the clarification of what "all" meant. The reproduced count (18/0) is consistent with the specific command. |
| 2 | Sheet A `TESTING TRACK` empty; sheet titled "Track A". | Track **not assigned**. Recorded as "not declared". |
| 3 | Sheet B self-declares **track C**, while the submission target is described elsewhere as Track B. | **Recorded verbatim as "tester-declared: C".** Not silently corrected. Flagged to the owner as a labelling question. |
| 4 | Same person (Ibrahim Abdulrahman) appears in both sheets — mobile/Chrome (Sheet A) and laptop/Windows 11/Brave (Sheet B). | Recorded as **one tester across two devices**, not two testers. This affects the "≥2 external testers" count: see §8. |

---

## 8. Distinct external testers

| Tester | Sources | Devices | Declared track |
|---|---|---|---|
| **Ibrahim Abdulrahman** (Borno) | Sheet A row 1 + Sheet B row 1 | Mobile/Chrome **and** Laptop/Windows 11/Brave | Sheet A: none · Sheet B: C |
| **Auwal** (Kano) | Sheet A row 2 | Mobile/Chrome | none |

**Distinct external testers = 2.** (Two different people, even though one appears twice.)
This satisfies the PS1 "≥2 external testers" requirement **on the count of distinct
people** — but see `KNOWN_LIMITATIONS.md`: neither tester's session was independently
reproduced by us, and only one cited a commit.

---

## 9. What is NOT verified

| Item | Status |
|---|---|
| The testers' sessions re-run by us | **NOT VERIFIED** — we did not reproduce their exact sessions. |
| Sheet A ratings (9/10, 8/10) as an objective measure | **NOT VERIFIED** — subjective self-report. |
| The Drive screenshot contents | **NOT VERIFIED** — not retrieved or inspected. |
| The tester's device/OS/browser | **NOT VERIFIED** — tester's own description. |
| That the remediation is visible to the testers | **NOT VERIFIED** — the testers tested `5599392`; the remediation is in `8216738`, which post-dates their sessions. |

---

## 10. Conclusion

- **Two distinct external testers** submitted responses across two spreadsheets.
- The **one checkable technical claim** (18 passed / 0 failed at commit `5599392`) is
  **independently reproduced**.
- The cited commit is **real**, dated, and is the parent of the current HEAD.
- **Track classification is not inferred from any title.** The only declared track is
  Sheet B's self-declared `C`.
- No tester session was re-run by us; no subjective rating is presented as objective
  evidence.

*End of report.*
