# Phase 2: 暗号化PDF + ストリームデコード

Status: **DONE** (全6ステップ完了)

## 目標

パスワード保護PDFの復号と、全主要ストリームフィルタのデコードを実装する。
Phase 1 のパーサ基盤の上に暗号化・圧縮レイヤーを積み、実用的なPDF読み込みを可能にする。

## デモ

```rust
let doc = pdfium_rs::Document::open_with_password("encrypted.pdf", "secret")?;
let stream = doc.object(42)?.as_stream()?.decode()?;
```

## モジュール構成

```text
src/
  fxcodec/
    mod.rs
    flate.rs          # FlateDecode (zlib/deflate)
    ascii85.rs        # ASCII85Decode
    ascii_hex.rs      # ASCIIHexDecode
    lzw.rs            # LZWDecode
  fdrm/
    mod.rs
    rc4.rs            # RC4 (自前実装 ~70行)
  fpdfapi/parser/
    decode.rs         # ストリームフィルタパイプライン
    security.rs       # PDF暗号化ハンドラ
```

## 実装ステップ

依存方向（下位→上位）に沿って実装する。各ステップは RED → GREEN コミット。

### Step 1: fxcodec — ストリームコーデック（外部依存なし）✅ DONE (PR #7)

純粋Rustで実装できるコーデックを先に作る。

| コーデック     | 対応フィルタ   | 概要                            |
| -------------- | -------------- | ------------------------------- |
| `ascii_hex.rs` | ASCIIHexDecode | 16進文字列 → バイト列。最も単純 |
| `ascii85.rs`   | ASCII85Decode  | Base-85エンコーディング         |
| `lzw.rs`       | LZWDecode      | GIF/TIFF由来のLZW圧縮           |

### Step 2: fxcodec/flate — Flateデコード（外部依存追加）✅ DONE (PR #8)

`flate2` crateを追加し、FlateDecode（zlib/deflate）を実装。
PDFストリームの大半がこのフィルタを使うため、最重要コーデック。

### Step 3: fpdfapi/parser/decode — フィルタパイプライン✅ DONE (PR #9)

ストリーム辞書の `/Filter` と `/DecodeParms` を読み、
コーデックをチェーンして適用するパイプラインを構築。

```rust
// 単一フィルタ
<< /Filter /FlateDecode /Length 1234 >>

// チェーンフィルタ
<< /Filter [/ASCII85Decode /FlateDecode] /DecodeParms [null << /Predictor 12 >>] >>
```

対応するPredictor:

- None (1)
- TIFF Predictor 2
- PNG predictors (10-15)

### Step 4: fdrm — 暗号プリミティブ✅ DONE (PR #10)

| アルゴリズム | 実装方法            | 理由                   |
| ------------ | ------------------- | ---------------------- |
| RC4          | 自前実装            | ~70行、外部依存不要    |
| MD5          | `md-5` crate        | 標準的なRustCrypto実装 |
| AES-128-CBC  | `aes` + `cbc` crate | PDF 1.6+ の暗号化      |
| AES-256-CBC  | 同上                | PDF 2.0 の暗号化       |
| SHA-256      | `sha2` crate        | PDF 2.0 のキー導出     |

### Step 5: fpdfapi/parser/security — PDF暗号化ハンドラ✅ DONE (PR #12)

PDF Standard Security Handler (revision 2-6) を実装:

1. **パスワード検証**: ユーザーパスワード / オーナーパスワードの照合
2. **暗号化キー導出**: MD5/SHA256ベースのキー計算
3. **オブジェクト復号**: RC4 or AES-CBCによるストリーム/文字列の復号
4. **権限フラグ**: 印刷・コピー等の権限情報の読み取り

対応するPDF暗号化バージョン:

| Revision    | アルゴリズム      | PDF Version |
| ----------- | ----------------- | ----------- |
| 2           | RC4 40-bit        | PDF 1.1+    |
| 3           | RC4 40-128bit     | PDF 1.4+    |
| 4           | RC4/AES-128       | PDF 1.5+    |
| 5 (AES)     | AES-128           | PDF 1.6+    |
| 6 (AES-256) | AES-256 + SHA-256 | PDF 2.0     |

### Step 6: 統合 — Document::open_with_password ✅ DONE (PR #14)

Document に暗号化対応を組み込み、End-to-End で動作させる:

- `Document::open_with_password(path, password)` API追加
- ストリームの `decode()` メソッド追加
- 暗号化検出 → キー導出 → オブジェクト復号の自動パイプライン

## 外部依存

```toml
[dependencies]
flate2 = "1"          # FlateDecode (miniz_oxide backend)
md-5 = "0.10"         # MD5 (PDF encryption key derivation)
sha2 = "0.10"         # SHA-256 (PDF 2.0 encryption)
aes = "0.8"           # AES block cipher
cbc = "0.1"           # CBC mode for AES
```

## テスト戦略

- 各コーデックの単体テスト（既知入出力ペア）
- RFC/仕様書のテストベクトルでの暗号プリミティブ検証
- テスト用暗号化PDFファイル（`tests/fixtures/`）での統合テスト
  - `encrypted_rc4_40.pdf` — RC4 40-bit
  - `encrypted_rc4_128.pdf` — RC4 128-bit
  - `encrypted_aes128.pdf` — AES-128
  - `encrypted_aes256.pdf` — AES-256
  - `encrypted_owner_only.pdf` — オーナーパスワードのみ

## 参照すべきC++ファイル

- `reference/pdfium/core/fpdfapi/parser/fpdf_parser_decode.{h,cpp}` — ストリームフィルタ
- `reference/pdfium/core/fpdfapi/parser/cpdf_security_handler.{h,cpp}` — 暗号化
- `reference/pdfium/core/fdrm/` — 暗号プリミティブ
- `reference/pdfium/core/fxcodec/` — コーデック実装
