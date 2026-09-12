---
adr_id: "2026-09-12-0139-clean-architecture-layer-rename"
decisions:
  - id: D1
    user_decision_ref: "chat_segment:sotohe-trial-adr:2026-09-12:clean-architecture-layer-rename:D1"
    candidate_selection: "from:[textbook-6-crate-rename, collapse-to-4-layers, custom] chose:textbook-6-crate-rename"
    status: proposed
  - id: D2
    user_decision_ref: "chat_segment:sotohe-trial-adr:2026-09-12:clean-architecture-layer-rename:D2"
    candidate_selection: "from:[keep-current-deps, allow-adapters-to-frameworks, custom] chose:keep-current-deps"
    status: proposed
  - id: D3
    user_decision_ref: "chat_segment:sotohe-trial-adr:2026-09-12:clean-architecture-layer-rename:D3"
    candidate_selection: "from:[rename-only, rename-plus-structural-textbook-placement, custom] chose:rename-plus-structural-textbook-placement"
    status: proposed
---
# クリーンアーキテクチャ層へのリネーム

## Context

本リポジトリは WEB システムであるのに、delivery 層が `cli` / `cli_driver` / `cli_composition` のまま残っている。

すでに main にある最小認証 API の外向き契約と認証の振る舞い・境界は変えず、層の名前と配置を教科書的クリーンアーキテクチャの語彙へ寄せたい。あわせて SoTOHE の architecture-customizer 経路（enforcement → crates → docs）を消費者リポで実践検証する。

## Decision

### D1: 教科書CAの6 crate へリネーム

次の crate map にリネームする（括弧内は現行）。

- `libs/entities`（`libs/domain`）
- `libs/use_cases`（`libs/usecase`）
- `libs/interface_adapters`（`apps/cli-driver`）
- `libs/frameworks`（`libs/infrastructure`）
- `apps/web-composition`（`apps/cli-composition`）
- `apps/web`（`apps/cli`）

Rust crate id は `entities` / `use_cases` / `interface_adapters` / `frameworks` / `web_composition` / `web` とする。

### D2: 依存方向は現状のリングを維持

依存方向は次のとおり維持する。

- `entities` → （なし）
- `use_cases` → `entities`
- `interface_adapters` → `use_cases`
- `frameworks` → `entities`, `use_cases`
- `web_composition` → `entities`, `use_cases`, `frameworks`, `interface_adapters`
- `web` → `web_composition`, `interface_adapters`

`architecture-rules.json` と `deny.toml` の wrappers もこの方向に同期する。

### D3: リネームに加え、置き場を教科書CAへ寄せる

外向きの register / login HTTP 契約と認証振る舞いは維持する。内部配置は次に寄せる。

1. Port / インターフェースは `use_cases` に置く
2. HTTP コントローラと DTO 変換は `interface_adapters` のみ
3. 永続化・暗号・axum サーバ配線などの実装は `frameworks`
4. 組み立ては `web_composition` のみ。`web` bin は薄い入口に留める

## Rejected Alternatives

- A. `cli` → `api` のリネームだけ: delivery 向け命名に留まり、教科書CAの Entities / Use Cases / Interface Adapters / Frameworks 語彙に届かないため。
- B. composition と bin を `frameworks` に畳む4層: 本 harness では composition root を明示しておきたいため。
- D. リネームのみで Port / HTTP の置き場を触らない: 認証契約は維持しつつ、より厳格な教科書的配置を採る方針を優先するため。

## Consequences

- 正: 教科書的クリーンアーキテクチャの語彙に揃い、Port / HTTP / 実装の置き場が明確になる。architecture-customizer 経路も検証できる。
- 負: crate・カタログ・レビュープロンプトにまたがる大規模リネームで差分とレビューが重い。
- 中立: 外向き auth HTTP 契約は維持するが、内部モジュール配置は移動する。

## Reassess When

- WEB API 以外の第2 delivery（バッチ / worker / 管理 UI 等）を足すとき。
- 永続化をプロセス内から外部 RDB / 共有ストアへ出すとき。
- 複数 bounded context / デプロイ単位へ分割するとき。
- `frameworks` と `interface_adapters` の境界が実務上あいまいになったとき。

## Related

- `.claude/skills/architecture-customizer/SKILL.md`
- `knowledge/adr/` 配下の最小認証 API 系 ADR（プロダクト決定は据え置き。本 ADR は層リネームと配置）
- Clean Architecture（Robert C. Martin）— 同心円の語彙
