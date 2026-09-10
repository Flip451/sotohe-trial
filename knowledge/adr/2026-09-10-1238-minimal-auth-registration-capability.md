---
adr_id: "2026-09-10-1238-minimal-auth-registration-capability"
decisions:
  - id: D1
    review_finding_ref: "ref-verify:chain-1:IN-001,AC-001:registration-capability"
    status: proposed
---
# 最小認証 API のユーザー登録能力

## Context

既存 ADR の Context では初期スコープにユーザー登録を含めているが、外部へ提供する能力としての決定にはなっていなかった。そのため、HTTP API を通じたユーザー登録を明示する判断が必要になった。

## Decision

### D1: HTTP API によるユーザー登録

最小認証 API は、HTTP API を通じてユーザー登録を外部に提供する能力を持つ。

この決定は、最小認証 API の HTTP 面を axum による REST API とする既存 ADR を補完し、その Context にある「ユーザー登録」の in-scope 項目を明確化する。

## Rejected Alternatives

- ユーザー登録を HTTP API の外部能力として定めない: API の利用者に提供する責務が不明確になるため。

## Consequences

- 正: ユーザー登録を要求する API の振る舞いを、ADR の決定として参照できる。
- 中立: HTTP の具体的な経路、要求形式、応答形式はこの決定では定めない。

## Reassess When

- ユーザー登録を HTTP API 以外の経路でも提供する必要が生じたとき。

## Related

- [2026-09-10-1126-minimal-auth-api.md](./2026-09-10-1126-minimal-auth-api.md) — HTTP 面の決定を補完し、Context の「ユーザー登録」を明確化する。
