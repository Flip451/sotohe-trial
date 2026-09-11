---
adr_id: "2026-09-10-1126-minimal-auth-api"
decisions:
  - id: D1
    user_decision_ref: "chat_segment:sotohe-trial-adr:2026-09-10:D1"
    candidate_selection: "from:[JWT Bearer, session cookie, Opaque] chose:Opaque"
    status: proposed
  - id: D2
    user_decision_ref: "chat_segment:sotohe-trial-adr:2026-09-10:D2"
    candidate_selection: "from:[argon2id, bcrypt, plaintext-trial-only] chose:argon2id"
    status: proposed
  - id: D3
    user_decision_ref: "chat_segment:sotohe-trial-adr:2026-09-10:D3"
    candidate_selection: "from:[in-memory+Port, embedded DB, external RDB] chose:in-memory+Port"
    status: proposed
  - id: D4
    user_decision_ref: "chat_segment:sotohe-trial-adr:2026-09-10:D4"
    candidate_selection: "from:[axum+REST, actix-web+REST, skip] chose:axum+REST"
    status: proposed
---
# 最小認証API

## Context

将来拡張できる認証の最小土台を、SoTOHE の実践投入題材として先に固定する必要がある。

初期スコープに含めるもの:

- ユーザー登録
- ログイン
- アクセストークン発行

初期スコープ外:

- リフレッシュトークン
- OAuth / 外部 IdP
- RBAC / 権限モデル

参照: [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)

## Decision

### D1: Opaque アクセストークン

認証成功後のクライアント証明には Opaque アクセストークンを用いる。

### D2: パスワードは argon2id でハッシュ保存

パスワードは平文保存せず、argon2id でハッシュして保存する。

### D3: インメモリ実装 + リポジトリ Port

ユーザーとトークンの永続化は、当面インメモリ実装とし、リポジトリ Port 経由で後から差し替え可能にする。

### D4: axum で REST API

HTTP 面は axum による REST API とする。

## Rejected Alternatives

- 平文パスワード: 拡張土台として不適切であるため。
- 初手の外部 RDB: 最小土台には重いため。

## Consequences

- 正: リポジトリ Port で永続化を後から差し替えやすい。
- 正: Opaque トークンはサーバー側失効が単純。
- 負: インメモリなので再起動でユーザー / トークンが消える。
- 負: トークン検証が常にストア参照になる。
- 中立: track 題材として十分小さく、拡張ポイントも見える。

## Reassess When

- プロセス再起動をまたぐ永続化が必要になったとき。
- リフレッシュトークンや長期セッションが必要になったとき。
- 複数プロセス / ホストでトークンを共有する必要が出たとき。
- OAuth や外部 IdP を導入するとき。

## Related

- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
