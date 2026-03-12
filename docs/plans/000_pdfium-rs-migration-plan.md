# PDFium Rust移植 全体戦略

## Context

Google PDFiumをRustに移植する。FFIバインディングではなく、Rustのイディオムに沿った純粋な再実装。
C++のリファレンスコード（`reference/pdfium/`）を参照しつつ、所有権モデル・型システム・エラーハンドリングをRustネイティブに設計する。

移植はCore機能（`core/`配下の7モジュール）から段階的に進め、各フェーズで動作するデモが得られるようにする。

## モジュール依存関係

```text
Level 0: fxcrt（基盤型: 座標、バイト文字列、ストリーム）
Level 1: fdrm（暗号）, fxge（グラフィックス）, fxcodec（画像コーデック）, fpdfapi/cmaps
Level 2: fpdfapi/parser（PDFオブジェクトモデル・構文解析）
Level 3: fpdfapi/page（ページコンテンツ）, fpdfapi/font（フォント）
Level 4: fpdfapi/render（レンダリング）, fpdfapi/edit（編集）
Level 5: fpdfdoc（ブックマーク・注釈・フォーム）, fpdftext（テキスト抽出）
```

## 対象外

- **XFA Forms**（`xfa/`）: PDF 2.0で非推奨。巨大な複雑さに見合わない
- **JavaScript**（`fxjs/`）: JSランタイムの埋め込みが必要。スコープ外
- **Barcode**（`fxbarcode/`）: 後日追加可能な周辺機能
- **Form filling UI**（`fpdfsdk/pwl/`, `formfiller/`）: JS依存。延期
- **Skia backend**: オプショナル。初期移植には不要

## プロジェクト構成

単一crateで内部モジュール分割。ワークスペースは依存関係の密結合と孤立ルール制約のため不採用。

```text
src/
  lib.rs                     # 公開API再エクスポート
  error.rs                   # 統一エラー型
  fxcrt/
    mod.rs
    bytestring.rs            # PdfByteString（Vec<u8>ラッパー）
    coordinates.rs           # Point, Size, Rect, Matrix
    stream.rs                # PdfRead/PdfWriteトレイト
  fdrm/
    mod.rs                   # RC4, MD5, AES, SHA
  fxcodec/
    mod.rs
    flate.rs                 # Flate/zlib
    fax.rs                   # CCITT Group 3/4
    ascii.rs                 # ASCII85, ASCIIHex
    lzw.rs                   # LZW
  fxge/
    mod.rs
    dib.rs                   # デバイス非依存ビットマップ
    color.rs                 # ARGB, CMYK, ブレンドモード
    path.rs                  # ベクターパス
  fpdfapi/
    parser/
      mod.rs
      object.rs              # PdfObject enum（中核データ構造）
      dictionary.rs          # PdfDictionary
      syntax.rs              # PDF構文トークナイザ
      document.rs            # Document（オブジェクトストア+ページツリー）
      cross_ref.rs           # 相互参照テーブル
      security.rs            # 暗号化ハンドラ
      decode.rs              # ストリームフィルタパイプライン
    page/
      mod.rs
      content_parser.rs      # ページコンテンツストリーム解釈
      color_space.rs         # 色空間（DeviceRGB, ICCBased等）
      page_object.rs         # TextObject, PathObject, ImageObject
    font/
      mod.rs
      cmap.rs                # CMapパーサ
      encoding.rs            # フォントエンコーディング
      to_unicode.rs          # ToUnicodeマップ
    render/
      mod.rs
      context.rs             # レンダリングコンテキスト
      status.rs              # メインレンダリングループ
    edit/
      mod.rs
      creator.rs             # PDF書き出し
  fpdfdoc/
    mod.rs
    bookmark.rs              # ブックマーク/アウトライン
    annotation.rs            # 注釈
    form.rs                  # AcroForm
    metadata.rs              # ドキュメント情報
  fpdftext/
    mod.rs
    text_page.rs             # テキスト抽出
    text_find.rs             # テキスト検索
```

## C++ → Rust 型マッピング

| C++                    | Rust                                     | 理由                                        |
| ---------------------- | ---------------------------------------- | ------------------------------------------- |
| `ByteString`           | `PdfByteString`（`Vec<u8>`ラッパー）     | PDFバイト列はUTF-8不保証                    |
| `WideString`           | `String`                                 | Rust内部はUTF-8で統一                       |
| `RetainPtr<T>`         | 所有権 or `Arc<T>`                       | 大半はRustの所有権で解決。共有必要時のみArc |
| `pdfium::span<T>`      | `&[T]` / `&mut [T]`                      | 直接対応                                    |
| `UnownedPtr<T>`        | `&T` / ハンドル（`u32`）                 | ライフタイムまたはインデックス              |
| `CPDF_Object`継承階層  | `PdfObject` enum                         | Dictionary, Array, Stream, String, Number等 |
| `CFX_FloatRect`        | `Rect { left, bottom, right, top: f32 }` | PDF座標系に準拠                             |
| `CFX_Matrix`           | `Matrix { a, b, c, d, e, f: f32 }`       | アフィン変換行列                            |
| `FX_ARGB` (packed u32) | `Color { r, g, b, a: u8 }`               | 構造体で型安全に                            |

### オブジェクトストアパターン

C++の`RetainPtr`による参照カウントを排除し、ハンドルベースの設計を採用:

```rust
pub struct ObjectStore {
    objects: HashMap<u32, LazyObject>,
}

enum LazyObject {
    Unparsed { offset: u64, gen_num: u16 },
    Parsed(PdfObject),
    CompressedRef { stream_obj_num: u32, index: u32 },
}

pub enum PdfObject {
    Boolean(bool),
    Integer(i32),
    Real(f64),
    String(PdfByteString),
    Name(PdfByteString),
    Array(Vec<PdfObject>),
    Dictionary(PdfDictionary),
    Stream(PdfStream),
    Null,
    Reference(ObjectId),
}
```

間接オブジェクトはすべてDocumentが所有し、参照は`ObjectId`（`u32`）で行う。循環参照問題を構造的に排除。

## フェーズ別マイルストーン

### Phase 1: fxcrt基盤 + PDF解析とメタデータ取得

**成果物**: fxcrtモジュールを網羅的に移植した上で、PDFファイルを開きオブジェクトグラフを走査できるライブラリ

**Step 1: fxcrt基盤の充実**（parserに進む前に完了させる）

- `coordinates.rs`: Point, Size, Rect, Matrix + 演算メソッド（逆行列、変換、交差判定）
- `bytestring.rs`: PdfByteString（Vec<u8>ラッパー）+ hex encode、大文字小文字無視比較
- `stream.rs`: PdfRead（Read + Seek）、PdfWriteトレイト
- `binary_buffer.rs`: 必要に応じた薄いラッパー
- `xml/`: PDFメタデータ用XMLプルパーサ（C++の~500行を移植。外部XML crateは不使用）

**Step 2: PDFパーサ**

- `fpdfapi/parser`: object, syntax, cross_ref, document（暗号化なし）
- `error`

**デモ**:

```rust
let doc = pdfium_rs::Document::open("test.pdf")?;
println!("Pages: {}", doc.page_count());
println!("Title: {:?}", doc.info().title());
```

**外部依存**: なし（stdlibのみ）

**参照すべきC++ファイル**:

- `reference/pdfium/core/fxcrt/fx_coordinates.h` — 座標型
- `reference/pdfium/core/fxcrt/bytestring.h` — バイト文字列
- `reference/pdfium/core/fxcrt/xml/` — XMLパーサ
- `reference/pdfium/core/fpdfapi/parser/cpdf_syntax_parser.{h,cpp}` — トークナイザ
- `reference/pdfium/core/fpdfapi/parser/cpdf_parser.{h,cpp}` — パーサ本体
- `reference/pdfium/core/fpdfapi/parser/cpdf_object.h` — オブジェクト階層

---

### Phase 2: 暗号化PDF + ストリームデコード

**成果物**: パスワード保護PDFの復号、全主要ストリームフィルタのデコード

**対象モジュール**:

- `fdrm`: RC4, MD5, AES, SHA
- `fpdfapi/parser/security`, `fpdfapi/parser/decode`
- `fxcodec`: flate, ascii, lzw

**デモ**:

```rust
let doc = pdfium_rs::Document::open_with_password("encrypted.pdf", "secret")?;
let stream = doc.object(42)?.as_stream()?.decode()?;
```

**外部依存**: `flate2`（Flate/zlib用）、RustCrypto (`aes`, `sha2`, `md-5`, `cbc`)（暗号プリミティブ）。RC4のみ自前移植（~70行）

**参照すべきC++ファイル**:

- `reference/pdfium/core/fpdfapi/parser/fpdf_parser_decode.{h,cpp}` — ストリームフィルタ
- `reference/pdfium/core/fpdfapi/parser/cpdf_security_handler.{h,cpp}` — 暗号化
- `reference/pdfium/core/fdrm/` — 暗号プリミティブ

---

### Phase 3: テキスト抽出

**成果物**: PDFページからテキストを位置情報付きで抽出。テキスト検索機能

**対象モジュール**:

- `fpdfapi/page`: content_parser, color_space（基本のみ）, page_object
- `fpdfapi/font`: cmap, encoding, to_unicode
- `fpdftext`: text_page, text_find

**CJK対応**: CMapデータ（数MBのテーブル）はこのフェーズで必要になった時点で追加。初期はASCII/Latin系PDFを対象とする

**デモ**:

```rust
let page = doc.page(0)?;
let text = page.extract_text()?;
let hits = page.find_text("quantum")?;
```

**外部依存**: 追加なし（フォントラスタライズ不要）

**参照すべきC++ファイル**:

- `reference/pdfium/core/fpdfapi/page/cpdf_streamcontentparser.{h,cpp}` — コンテンツ解析
- `reference/pdfium/core/fpdfapi/font/cpdf_tounicodemap.{h,cpp}` — Unicode変換
- `reference/pdfium/core/fpdftext/cpdf_textpage.{h,cpp}` — テキスト抽出

---

### Phase 4: ページレンダリング

**成果物**: PDFページをRGBAビットマップにレンダリング（テキスト描画なし初期版）

**対象モジュール**:

- `fxge`: dib, color, path（全機能）
- `fpdfapi/render`: context, status, image_renderer, path_renderer
- `fxcodec`: jpeg追加

**デモ**:

```rust
let bitmap = page.render(300.0 /* DPI */)?;
bitmap.save_png("output.png")?;
```

**外部依存**: `tiny-skia`（ラスタライザ）, `jpeg-decoder`

---

### Phase 5: テキスト描画 + ドキュメント機能

**成果物**: 完全なテキストレンダリング、ブックマーク・注釈・フォーム（読み取り専用）

**対象モジュール**:

- `fxge/font`: フォントラスタライズ
- `fpdfdoc`: bookmark, annotation, form, metadata
- `fpdfapi/page`: シェーディング、タイリングパターン

**外部依存**: `ab_glyph` or `freetype-rs`（フォントラスタライズ。判断はこの時点で行う）

---

### Phase 6: PDF書き出し・編集

**成果物**: 新規PDF作成、ページ追加/削除、コンテンツ変更

**対象モジュール**:

- `fpdfapi/edit`: creator, content_generator

## 外部依存方針

最小限の依存。フェーズが進むごとに必要なものだけ追加。

| 用途         | crate                       | フェーズ |
| ------------ | --------------------------- | -------- |
| Flate圧縮    | `flate2` (miniz_oxide)      | 2        |
| AES/SHA      | `aes`, `sha2`, `cbc`        | 2        |
| MD5          | `md-5`                      | 2        |
| JPEG         | `jpeg-decoder`              | 4        |
| ラスタライザ | `tiny-skia`                 | 4        |
| PNG出力      | `png`                       | 4        |
| フォント     | `ab_glyph` or `freetype-rs` | 5        |
| Unicode      | `unicode-normalization`     | 3        |
| JBIG2        | 自前移植（延期可）          | 4+       |
| JPEG2000     | `openjpeg-sys`（延期可）    | 4+       |

**Phase 1は外部依存ゼロ**。stdlibのみで完結させる。

## テスト戦略

TDDサイクル（RED → GREEN → REFACTOR を別コミット）に従う。

- `tests/fixtures/`: テスト用PDFファイル群
  - `minimal.pdf`, `hello.pdf`, `multipage.pdf`, `encrypted_*.pdf`, `cjk.pdf` 等
- **ユニットテスト**: 各モジュール内。トークナイザの字句解析、オブジェクトの構築・比較等
- **統合テスト**: `tests/`配下。実PDFを開いてメタデータ・テキスト・レンダリング結果を検証
- **参照比較**: C++の`*_unittest.cpp`のテストロジックをRustに移植し、正確性の仕様として利用
- **Property-based testing**: トークナイザ/パーサに`proptest`を検討（パニックしないことの検証）

## 検証方法

各フェーズ完了時の検証:

1. **Phase 1**: `cargo test` で各種PDFのメタデータ取得テストが通る。CLIで`pdfinfo`相当の出力を確認
2. **Phase 2**: 暗号化PDFテストケース群が通る。ストリームデコード結果をC++版の出力と比較
3. **Phase 3**: テキスト抽出結果を`pdftotext`コマンドの出力と比較。差分が許容範囲内
4. **Phase 4**: レンダリング結果のビットマップを`pdftoppm`出力とピクセル比較。SSIM等で類似度検証
5. **Phase 5-6**: 同上の拡張
