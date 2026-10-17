## 1. Approved design and regression contract

- [x] 1.1 Record proposal, design, independent review conditions, and specification deltas before implementation.
- [x] 1.2 Write failing regression tests for actual native branches, overrides, aggregation, strict diagnostics, staged roundtrip, and atomic rejection.

## 2. Implementation

- [x] 2.1 Add aggregated rule field findings without changing rendering or AgentPlugin behavior.
- [x] 2.2 Parse both stable reason codes in staged plans and identify approved rule losses in strict diagnostics.
- [x] 2.3 Update migration tests only where they depended on silent loss.

## 3. Validation

- [x] 3.1 Run focused tests and record results in the detailed temporary log.
- [x] 3.2 Run mise run check and mise run spec-validate in the current sandbox.
