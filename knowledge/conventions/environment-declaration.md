# Project Environment Declaration

この文書は **本プロジェクト（consumer）が所有する**環境前提の記入インスタンスである。
枠と記入指針は `environment-assumptions.md` が提供し、ここにはその宣言欄の実値のみを置く。

> **強制先**: review 観点 — harness-policy scope

## Supported Platforms

- 対応する platform、architecture、runtime: Linux (kernel 5.x+), `x86_64-unknown-linux-gnu`, Rust edition 2024 / rust-toolchain channel `1.94.0`, host process (no WASM / no bare-metal). OS CSPRNG via `getrandom`/`OsRng` must be available for Argon2 salt and opaque-token issuance.
- 対応外または条件付きの範囲: Windows / macOS / non-x86_64 / no-std / browser Wasm は未検証。コンテナは上記 Linux ABI と同等の CSPRNG を提供する場合に限り可。
- platform 差が入力、ファイル、時刻、プロセス、または終了処理に与える条件: 認証アダプタは時刻やファイル I/O に依存しない。CSPRNG 失敗は hashing / token issuance を fail-closed (`Unavailable`) にする。HTTP サーバ既定 bind は loopback `127.0.0.1:3000`（`frameworks::http::HttpServer::serve` / `HTTP_BIND_ADDRESS`）。当該アドレス・ポートが利用できない場合は `HttpServerOutcome::Failed`。本番公開面や別ポートは後続設定導入まで対象外。

## Input-Encoding Policy

- 受け付ける入力経路と encoding: REST JSON bodies は UTF-8 (CN-004)。平文パスワードは UTF-8 バイト列として application boundary (`PlaintextPassword`) で受け取り、格納パスワードハッシュは PHC 文字列 (`$argon2id$v=19$m=…,t=…,p=…$<salt>$<digest>`) で salt/digest は Base64 unpadded。
- decode、正規化、改行や byte order などの扱い: JSON 文字列値は UTF-8 decode 後に Unicode 正規化しない (CN-004)。パスワードバイトは受け取りどおり保持する。PHC の salt/digest は Base64 unpadded のみ受理する。
- encoding が不正、未指定、または判定できない場合の扱い: 不正 UTF-8 / 不正 JSON は HTTP 400、非 `application/json` は HTTP 415 (CN-004)。PHC/Base64 不正または非 argon2id ハッシュは verification `Unavailable`。空パスワードは `PasswordInputError::Empty` (CN-005)。超過長は `PasswordInputError::TooLong`。

## Resource Limits

- 入力サイズ、メモリ、保存領域、処理時間、同時実行数などの上限: (1) 平文パスワード最大 1024 UTF-8 バイト (`PlaintextPassword::parse`)。(2) Argon2id パラメータ上限は hashing 既定と同一: `m≤19456` KiB、`t≤2`、`p≤1`、出力長 32 バイト、salt 16 バイト。PHC 全文は最大 256 UTF-8 バイト。verify は格納ハッシュの PHC 長・パラメータまたは salt/digest 長がこの上限を超える場合に割り当て/走査前に拒否する。(3) インメモリ User/AccessToken リポジトリの件数上限は設けない（プロセスメモリが境界）。(4) 認証 Argon2 同期ジョブはプロセス全体で同時 8 件 (`AUTH_BLOCKING_CONCURRENCY_LIMIT`)。`AuthHttpApi` 全インスタンス共有の `Semaphore` で入場し、追加リクエストは `spawn_blocking` 投入前に待機する（新規 HTTP ステータスなし。待ち行列は Tokio タスクとして非同期待ち）。
- 上限の単位、適用範囲、超過時の失敗動作: パスワード超過は `PasswordInputError::TooLong`。Argon2 パラメータ超過・salt/digest 過長または不正 PHC は `PasswordVerificationError::Unavailable`。CSPRNG 失敗は hashing/issuance `Unavailable`。認証同時実行が 8 を超える場合は HTTP 応答を変えず許可取得まで待機する。
- 上限を設けない項目がある場合の理由と、代わりに置く境界: インメモリ件数は置換可能な repository port (CN-002) の責務とし、本トラックではプロセスメモリを事実上の境界とする。HTTP リクエストボディ上限: `AuthHttpApi` は axum `DefaultBodyLimit::max(64 KiB)` (`HTTP_MAX_BODY_BYTES`) を適用する。超過はフレームワーク既定どおり HTTP 413。既定 HTTP bind は Supported Platforms の `127.0.0.1:3000` に固定し、ポート競合は bind 失敗として表面化する。汎用 HTTP 接続上限は設けず、認証 blocking 入場のみ上記 8。

## Concurrency Model

- thread、task、process などの実行単位と、同時実行の上限: 同期 API (`Send + Sync` ports)。インメモリリポジトリはプロセス内共有。HTTP 配信は Tokio + axum (`frameworks::http::HttpServer::serve`) で単一プロセス内の非同期タスクとして動作する。既定 listen は `127.0.0.1:3000`。汎用ワーカー数・同時接続上限の明示設定は後続。register/login の Argon2 同期実行は `spawn_blocking` へ外し、プロセス全体で同時 8 件 (`AUTH_BLOCKING_CONCURRENCY_LIMIT` / 共有 `Semaphore`) に制限する。飽和時の queue/saturation 挙動は許可取得まで待機（HTTP ステータス変更なし）。Argon2 `p` は 1 に固定。
- shared state の所有、同期、順序、再入可能性: `InMemoryUserRepository` / `InMemoryAccessTokenRepository` は `std::sync::RwLock` で HashMap を保護。ロック毒は repository `Unavailable`。同一 username の二重 save は `DuplicateUser`。順序保証はリポジトリ操作単位。HTTP ハンドラは usecase ports を `Arc` 共有する。
- cancellation、shutdown、失敗時の処理: usecase 同期呼び出しに協調 cancellation なし。`frameworks::http::HttpServer::serve` は既定 bind `127.0.0.1:3000` の後、`axum::serve(...).with_graceful_shutdown` で Ctrl-C (`tokio::signal::ctrl_c`) を待つ。Ctrl-C 受信でグレースフル停止し `HttpServerOutcome::Stopped`（web presenter が success exit に写像）。`ctrl_c` の handler インストール失敗は shutdown 完了後に検出し `Failed("failed to install Ctrl-C handler: …")`（web presenter が failure exit に写像）。bind 失敗も `Failed`。axum 0.8 の graceful-shutdown 経路では accept ループが正常終了するため、serve 本体の `Err` は通常到達しない（到達した場合のみ `Failed`）。Ctrl-C 以外の強制プロセス終了では future が drop され outcome は返らない。インメモリ状態はプロセス終了で消える。部分失敗は型付きエラーで fail-closed（トークン発行後の persist 失敗は `LoginError::Persistence`）。
