# Operator training divergence & corrective plan (2026-05-29)

**Status: canonical post-divergence record.** This is the single source of truth
for what diverged in operator training, since when, and the fix. Other docs
forward-link here. Forensic evidence:
`operator-experiments/audits/training_divergence_audit_20260529.json`.

> **CORRECTION (2026-05-29, later same day).** A follow-up diagnostic
> (`operator-experiments/audits/DIAGNOSTIC_promptfix_20260529.md`) **overturned the
> original cliff attribution.** The read-nav generalization cliff was NOT caused by
> un-anonymized refs — it was a **system-prompt build bug**: the Tier 2/3 eval rows
> were authored with a 356-char prompt that OMITS the MCP/API tool schema (training
> uses the 3741-char full-schema prompt). Re-predicting the *unchanged* v8.1.8
> adapter on a prompt-corrected eval (raw refs, no anonymization, no retrain) lifted
> read-nav structural validity from 0% to 100% (parse failures 51→5; NEW-rows exact
> 16.9%→64.9%). So: the cliff is a measurement artifact; **Directive B (MCP/API in
> context) is the load-bearing fix**; anonymization (Directive A) is a real design
> requirement but did NOT cause the cliff. Sections below are kept for the verified
> timeline but read §3 RANK-1/RANK-2 with this correction in mind.

## 1. Intended design (the North Star)

From the kernel design plan (`rehydration-kernel/docs/product/kernel-tool-operator-model-plan.md`):

- **"Operator 0.5B: only learns to use KMP"** (plan:243-244). It **must not own
  answer semantics for benchmark-specific tasks** (plan:289).
- **Reference anonymization is mandatory**: model-facing refs are rewritten to
  opaque ids (`ref_0001` / `about_0001`), stripping `about:` / `evidence:` /
  `question:run:` / `turn:run:` (and the realistic domain topics) so that "long
  raw refs" never reach model-facing state (plan:182-186).
- The model receives a **compact MCP/tool schema serialized into the prompt**
  (plan:349, :663-667) plus structured, non-gold `visible_state` features
  (operator_state). The schema-in-prompt is necessary but insufficient on its
  own (the prompt-only v9 experiment did not fix the inspect-vs-near boundary,
  plan:169-175); the model is SFT-trained to *operate* from that state.
- The **teacher is used OFFLINE only** to generate trajectory structure /
  semantics. Domain interpretation (rich relations, `why`, cited evidence,
  derivations) is delegated to the teacher and **must never be baked into the
  model's learned answer content** (plan:262-265).

Net: schema in prompt + structured visible-state features + **anonymized refs** +
offline teacher = a **topic-agnostic KMP operator**. The V6 holdout built this
way reached **1.000 exact action accuracy** (2026-05-12), with a clean live MCP
replay on 2026-05-14.

## 2. What diverged, and since when

| date | commit | change | intended vs actual |
|---|---|---|---|
| 2026-05-11 | `7448b85` (kernel) | first SFT pipeline; refs = opaque benchmark IDs | ✅ aligned |
| 2026-05-12 | (V6) | 1.000 exact acc with compact prompt + anonymized refs | ✅ aligned |
| **2026-05-16** | **`46bd76f`** (kernel) | domain prefixes (`incident:`/`about:`/`evidence:`…) added to `looks_like_ref()` | ⚠️ divergence begins — intended to let anonymization *cover* them, but normalized domain refs into the corpus |
| 2026-05-18 | `8edb1b2` (operator) | SFT/LoRA scripts migrated kernel→operator (schema-in-prompt preserved; domain-ref patterns carried along) | partial |
| 2026-05-20 | `6ac5cb2`/`f04881f` | realistic-v7 corpus formalized (domain narratives); compact `kind` schema | ⚠️ diverge |
| **2026-05-22** | **`d9cb0a0`/`19d1e97`** | teacher-backed v7.3 corpus; literal domain topics codified in refs (`build_realistic_scenarios.py` ABOUT_PREFIXES_BY_THEME) | ⚠️ divergence hardens |
| 2026-05-25/26 | `884875e`/`3d3b2b5` | runtime vLLM policy; `DEFAULT_SYSTEM_PROMPT` = 163 chars, **no schema** | ⚠️ latent train/inference skew |
| **2026-05-29** | v8.1.8 Tier 4 | trained with anonymization **OFF**; 1523/1933 targets carry literal domain refs | ❌ active divergence |

**Answer to "since when are we diverging":** the divergence began **2026-05-16**
(`46bd76f`) and **hardened 2026-05-22** (v7.3 teacher corpus). Every v8.1.x model,
including the v8.1.8 Tier-4 LoRA trained on 2026-05-29, was trained on
un-anonymized domain refs.

## 3. The two issues (ranked — corrected after the prompt-fix diagnostic)

### RANK 1 — missing MCP/API schema in the system prompt (the ACTUAL cliff cause)
The Tier 2/3 corpus builders authored their eval rows with a **356-char system
prompt that OMITS the tool schema**, while training and the base eval use the
**3741-char full-schema prompt**. 48/77 new eval rows (exactly the read-nav +
write families) had the stripped prompt. Without the schema in context the model
hallucinates the action envelope (`{"action":"call_memory"…}`,
`{"action":"navigate_to"…}`). **Proven by isolation:** re-predicting the unchanged
v8.1.8 adapter on a prompt-corrected eval (raw refs, no anonymization, no retrain)
took read-nav structural validity from 0% to 100% (parse failures 51→5; NEW exact
16.9%→64.9%). The **same defect previously existed at runtime** (a 163-char
schemaless `DEFAULT_SYSTEM_PROMPT`); it is now fixed — `vllm_openai_operator_policy.rs`
serves the canonical full-schema prompt (`include_str!` of
`prompts/operator_system_prompt_full_v1.txt`), kept byte-equal to the prep
pipeline's `FULL_SYSTEM_PROMPT`. This is Directive B, and it is the load-bearing
fix. Evidence:
`operator-experiments/audits/DIAGNOSTIC_promptfix_20260529.md`.

### RANK 2 — un-anonymized domain-ref leakage (real design divergence; NOT the cliff)
`--anonymize-refs` defaulted **OFF** (`prepare_operator_sft_dataset.py`, was
`store_true`) and was never used in v8.x, so literal domain topics flow into
model-facing state, violating plan:182-186 and the North Star (plan:243-244). This
is a **genuine design-correctness divergence** and Directive A — but the
prompt-fix diagnostic shows it did **not** cause the read-nav cliff (the model
handles raw domain refs and novel topics correctly once given the schema). Its
empirical performance benefit is unproven by current measurements; it is kept
mandatory as a design principle (don't ship a model that memorized domain content;
opaque refs).

### Notes on framing
- "The teacher bakes domain SEMANTICS into the targets" is **unsupported** — the
  prose targets are templated boilerplate, not GPT-authored interpretation.
- The "model memorizes topics / needs topic diversity" framing is also **wrong**:
  with the schema in context the model handles novel domain topics fine. The
  cliff was the missing-schema prompt, not topic exposure.

## 4. Corrective plan (re-prioritized after the diagnostic)

**Directive B — give the model the MCP/API in context (THE load-bearing fix):**
1. ⏳ **Corpus:** make ALL rows carry the canonical full-schema system prompt.
   Fix the Tier 2/3 builders (they used a 356-char schemaless prompt) and add a
   prep-time/parity check that every SFT row's system prompt byte-equals the
   profile's canonical prompt. This alone recovered read-nav (0%→100% struct).
2. ✅ **Runtime:** `DEFAULT_SYSTEM_PROMPT` (`vllm_openai_operator_policy.rs`) is now
   the canonical schema-bearing prompt, `include_str!`-ed from
   `prompts/operator_system_prompt_full_v1.txt` (single source of truth shared by
   the prep script and the runtime). The prep pipeline enforces parity:
   `prepare_operator_sft_dataset.py` `FULL_SYSTEM_PROMPT` byte-equals that asset via
   `assert_full_prompt_matches_runtime_asset()`.
3. ⏳ **Remaining real gaps** (small, argument-value policy, not schema collapse):
   `near.limit-pressure` (limit), `near.dimension-filter` (dimension selection),
   `kernel_trace.full-mode` (page). These are the genuine next training targets.

**Directive A — anonymization (design correctness; did NOT cause the cliff):**
4. ✅ `--anonymize-refs` now defaults **ON** (`--no-anonymize-refs` only for
   de-anonymized replay/debug). [`prepare_operator_sft_dataset.py`]
5. ✅ `looks_like_ref()` extended to cover `migration:`/`bug:`/`product:`/`docs:`
   via `DOMAIN_TOPIC_REF_PREFIXES`; prep-time guard fails the build on surviving
   domain refs; manifest records the anonymize status.
6. ⏳ Regenerate the corpus anonymized + retrain when prioritized. Anonymization is
   mandatory per the design plan, but the diagnostic shows it is not required to
   close the cliff; sequence it after the schema-prompt fix.

Sequencing: items 4-5 (anonymization hygiene) and Directive B item 2 (runtime
prompt parity) are **done**. The urgent remaining fix is Directive B item 1 (schema
in context for every corpus row). Do not draw model-quality conclusions from any
eval whose rows lack the full-schema prompt.
