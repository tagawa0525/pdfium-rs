# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

C++版PDFium（Google）のRust移植。FFIバインディングではなく純粋な再実装。Rust edition 2024。
単一crateで内部モジュール分割（ワークスペース不採用）。

## ビルド・テスト・リント

```bash
cargo check --all-features
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test bytestring       # 特定テスト
```

## アーキテクチャ

### モジュール依存関係

```text
Level 0: fxcrt（基盤型: 座標、バイト文字列、ストリーム）
Level 1: fdrm（暗号）, fxge（グラフィックス）, fxcodec（画像コーデック）, fpdfapi/cmaps
Level 2: fpdfapi/parser（PDFオブジェクトモデル・構文解析）
Level 3: fpdfapi/page（ページコンテンツ）, fpdfapi/font（フォント）
Level 4: fpdfapi/render（レンダリング）, fpdfapi/edit（編集）
Level 5: fpdfdoc（ブックマーク・注釈・フォーム）, fpdftext（テキスト抽出）
```

下位レベルから順に移植する。上位モジュールは下位モジュールに依存するが逆はない。

### C++ → Rust 型マッピング

| C++                    | Rust                                     |
| ---------------------- | ---------------------------------------- |
| `ByteString`           | `PdfByteString`（`Vec<u8>`ラッパー）     |
| `WideString`           | `String`                                 |
| `RetainPtr<T>`         | 所有権 or `Arc<T>`                       |
| `pdfium::span<T>`      | `&[T]` / `&mut [T]`                      |
| `CPDF_Object`継承階層  | `PdfObject` enum                         |
| `CFX_FloatRect`        | `Rect { left, bottom, right, top: f32 }` |
| `CFX_Matrix`           | `Matrix { a, b, c, d, e, f: f32 }`       |
| `FX_ARGB` (packed u32) | `Color { r, g, b, a: u8 }`               |

### オブジェクトストア

間接オブジェクトはすべてDocumentが所有し、参照は`ObjectId`（`u32`）で行う。C++の`RetainPtr`参照カウントを排除し、循環参照問題を構造的に解消。

### エラー処理

Phase 1-2: `std::error::Error`のみ（外部依存ゼロ方針に準拠）。
Phase 3以降: 必要に応じて`thiserror`構造化enumに移行。core → `Error`/`Result<T>`、他ドメイン → `<Domain>Error`/`<Domain>Result<T>`。`#[from]`で自動伝播。

## PRワークフロー

以下のGit/TDDルールをプロジェクト標準として適用する。

### コミット構成

1. RED: テスト（`#[ignore = "not yet implemented"]` 付き）
2. GREEN: 実装（`#[ignore]` 除去）
3. REFACTOR: 必要に応じて
4. 全テスト・clippy・fmt通過を確認

### PR作成〜マージ

1. PR作成
2. `/gh-actions-check` でCopilotレビューワークフローが `completed/success` になるまで待つ
3. `/gh-pr-review` でコメント確認・対応
4. **レビュー修正は独立した `fix(<scope>):` コミットで積む（RED/GREENに混入させない）**
5. push後の再レビューサイクルも完了を確認（同じ手順を繰り返す）
6. `docs/plans/` の進捗ステータスを更新する（`docs:` コミット）
7. 全チェック通過後 `/gh-pr-merge --merge`

### PRやり直し時

- 元のRED/GREENをそのままcherry-pick（内容を改変しない）
- 過去PRのレビュー修正は独立 `fix(<scope>):` コミットとして積む
- 異なるPRの修正は別コミットにする

### 規約

- ブランチ命名: `feat/<module>-<機能>`, `test/<スコープ>`, `refactor/<スコープ>`, `docs/<スコープ>`
- コミット: Conventional Commits、scopeにモジュール名
- マージコミット: `## Why` / `## What` / `## Impact` セクション
- 計画書 (`docs/plans/`) を実装着手前にコミットすること

## 外部依存方針

Phase 1（fxcrt + parser）は外部依存ゼロ。stdlibのみで完結。以降のフェーズで必要最小限を追加。

## リファレンス

C++版ソース: `reference/pdfium/`（.gitignore対象）
移植計画書: `docs/plans/000_pdfium-rs-migration-plan.md`
