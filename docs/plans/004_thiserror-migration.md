# thiserror導入計画

## Context

Phase 1-2ではゼロ依存方針に従い`std::error::Error`の手動実装を使用してきた。
Phase 3（テキスト抽出）着手に先立ち、移植計画（`docs/plans/000_pdfium-rs-migration-plan.md`）で定められた通り`thiserror`を導入する。

現状の`src/error.rs`には手動の`impl Display`/`impl std::error::Error`/`impl From<io::Error>`が約20行あり、`#[derive(thiserror::Error)]`で同等の機能を実現できる。

**目的**: thiserrorの導入と既存Error enumのderiveマクロ化。Phase 3以降でドメイン別エラー型を追加する基盤を整える。

## 変更内容

### 1. `Cargo.toml` — thiserror依存追加

```toml
thiserror = "2"
```

### 2. `src/error.rs` — deriveマクロに移行

**Before** (手動実装 ~40行):

```rust
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    InvalidPdf(String),
    Unsupported(String),
}

impl fmt::Display for Error { ... }      // 8行
impl std::error::Error for Error { ... } // 7行
impl From<io::Error> for Error { ... }   // 5行
```

**After** (thiserror derive ~12行):

```rust
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid PDF: {0}")]
    InvalidPdf(String),

    #[error("unsupported: {0}")]
    Unsupported(String),
}
```

- `use std::fmt` を削除（thiserrorがDisplayを生成するため不要）
- 手動の`Display`/`Error`/`From` implをすべて削除
- `#[error("...")]` でDisplayフォーマットを定義（既存と同一文字列）
- `#[from]` で`From<io::Error>`を自動生成
- `pub type Result<T>` はそのまま維持

### 3. テスト — 変更不要

既存テストはすべてそのまま通る:

- `error_display_*` — Displayフォーマット文字列が同一
- `error_from_io` — `#[from]`が同等の`From` implを生成
- `error_source_*` — thiserrorは`#[from]`フィールドに対して`source()`を正しく実装

## 対象ファイル

| ファイル | 変更内容 |
| ------- | ------- |
| `Cargo.toml` | `thiserror = "2"` 追加 |
| `src/error.rs` | deriveマクロ化、手動impl削除 |

他のファイル(`fdrm/`, `fxcodec/`, `fpdfapi/parser/`)は変更不要。`use crate::error::{Error, Result}` のインポートはそのまま動作する。

## コミット構成

純粋なリファクタリング（振る舞い変更なし）のため、RED/GREENサイクルではなくリファクタリングコミットとする:

1. `refactor(error): migrate to thiserror derive macros` — Cargo.toml + error.rs の変更

## 検証

```bash
cargo test --all-features              # 全テスト通過
cargo clippy --all-features --all-targets -- -D warnings  # lint通過
cargo fmt --all -- --check             # フォーマット通過
```
