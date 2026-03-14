# pdfium-rs Wasm化 障害調査レポート

## 結論

**重大なブロッカーはない。** コードベースは純粋Rustで、`Document<R: Read + Seek>` のジェネリック設計により、すでにWasm互換のアーキテクチャになっている。対応が必要な項目はいずれも軽微。

---

## 障害候補一覧

### 1. `std::fs::File` の直接使用（軽微）

**箇所:** `src/fpdfapi/parser/document.rs:115-128`

```rust
impl Document<BufReader<File>> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> { ... }
    pub fn open_with_password(path: impl AsRef<Path>, password: &[u8]) -> Result<Self> { ... }
}
```

**影響:** ブラウザWasmではファイルシステムAPIが使えないため、このimplブロックはコンパイルエラーになる。

**対策:** `#[cfg(not(target_arch = "wasm32"))]` で条件コンパイルするだけで済む。コア機能は `Document::from_reader(Cursor::new(bytes))` で利用可能。

---

### 2. `main.rs` のバイナリターゲット（軽微）

**箇所:** `src/main.rs`（`println!("Hello, world!")`のみ）

**影響:** `wasm32-unknown-unknown` ターゲットでは `main()` が通常のエントリポイントとして機能しない。

**対策:** ライブラリとしてのみビルドする（`--lib`）か、`Cargo.toml` で `[[bin]]` セクションにターゲット制限を追加。

---

### 3. メモリ上限値（要検討）

**箇所:**

- `src/fpdfapi/parser/decode.rs:7` — `MAX_DECODED_SIZE = 256 MiB`
- `src/fpdfapi/parser/cross_ref.rs:27` — `MAX_XREF_READ = 2 MiB`

**影響:** Wasmのデフォルトメモリ上限は通常256 MiB〜数GiB（ブラウザ依存）。256 MiBのデコードバッファを確保すると、他のメモリと合わせてOOMになる可能性がある。

**対策:** Wasm向けにはより保守的な上限（例: 64 MiB）をfeature flagや`cfg`で切り替える。

---

### 4. 推移的依存のCPU機能検出（実害なし）

**関連crate:**

- `sha2`, `aes` → `cpufeatures`（ランタイムCPU機能検出）
- `flate2` → `miniz_oxide` → `simd-adler32`（SIMD最適化）

**影響:** Wasmでは `cpuid` 等が使えないが、これらのcrateはフォールバック実装を持っており、Wasmでは自動的にポータブル実装が使われる。パフォーマンスは若干低下するが、機能的には問題ない。

**`flate2`のバックエンド:** `rust_backend`（`miniz_oxide`、純粋Rust）が使われていることを確認済み。Cライブラリ依存なし。

---

### 5. テストのファイルシステム依存（テストのみ）

**箇所:**

- `tests/integration_text_extraction.rs`
- `tests/integration_security_handler.rs`

**影響:** `File::open()` + `env!("CARGO_MANIFEST_DIR")` でPDFフィクスチャを読み込んでいる。ブラウザWasm環境ではテスト実行不可。

**対策:** ライブラリコードには影響しない。Wasm用のテストが必要なら `include_bytes!()` でバイナリ埋め込みにする。

---

## 非該当（問題なし）の項目

| 項目                                     | 状態                       |
| ---------------------------------------- | -------------------------- |
| unsafeコード                             | なし                       |
| FFI / libc                               | なし                       |
| スレッド / Mutex / Arc                   | なし                       |
| async / await                            | なし                       |
| グローバル可変状態                       | なし                       |
| 乱数生成                                 | なし                       |
| プラットフォーム固有 `#[cfg]`            | なし（`#[cfg(test)]`のみ） |
| `std::env` / `std::process` / `std::net` | なし                       |
| `std::time`                              | なし                       |
| カスタムHasher                           | なし                       |
| build.rs                                 | なし                       |
| 外部C/C++依存                            | なし                       |

---

## 対応優先度

1. **必須:** `document.rs` のFile impl を `cfg(not(wasm32))` で囲む
2. **推奨:** `MAX_DECODED_SIZE` のWasm向け調整
3. **任意:** `wasm-bindgen` によるJSバインディング提供、`wasm-pack`対応
