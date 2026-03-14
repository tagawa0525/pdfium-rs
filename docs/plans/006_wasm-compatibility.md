# Wasm互換性対応

Status: **IN PROGRESS**

## Context

pdfium-rsは純粋Rustで実装されており、`Document<R: Read + Seek>` のジェネリック設計により本質的にWasm互換なアーキテクチャになっている。重大なブロッカーはないが、ブラウザWasm環境（`wasm32-unknown-unknown`）でのコンパイルを通すために軽微な条件コンパイル対応が必要。

## 障害調査結果

### 対応が必要な項目

1. **必須** — `document.rs:115`: `impl Document<BufReader<File>>` がファイルシステムAPI依存
2. **推奨** — `decode.rs:7`: `MAX_DECODED_SIZE = 256 MiB` がWasmメモリ上限に近い
3. **軽微** — `main.rs`: バイナリターゲットがWasmで不要（`--lib`でビルドすれば回避可）

### 問題なしの項目

- unsafeコード、FFI/libc、スレッド/Mutex/Arc、async/await — いずれもなし
- 推移的依存（`sha2`, `aes`, `flate2`）はWasmフォールバック実装を持つ
- `flate2`は`rust_backend`（`miniz_oxide`、純粋Rust）を使用、Cライブラリ依存なし
- テストのファイルシステム依存はライブラリコードに影響しない

## Steps

### Step 1: `document.rs` の File impl を条件コンパイル

- `impl Document<BufReader<File>>` ブロックと関連import（`std::fs::File`, `std::path::Path`）を `#[cfg(not(target_arch = "wasm32"))]` で囲む
- コア機能（`from_reader`, `from_reader_with_password`）は影響なし

### Step 2: `MAX_DECODED_SIZE` のWasm向け調整

- `decode.rs` で `cfg` により Wasm環境では 64 MiB、それ以外では 256 MiB に設定

### Step 3: Wasmターゲットでのコンパイル検証

- `cargo check --lib --target wasm32-unknown-unknown` でコンパイルエラーがないことを確認
