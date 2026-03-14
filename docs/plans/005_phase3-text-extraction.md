# Phase 3: テキスト抽出

Status: **NOT STARTED**

## Context

Phase 1-2でPDF解析基盤（オブジェクトモデル、構文解析、暗号化、ストリームデコード）が完成した。
Phase 3では、この基盤の上にフォント・コンテンツストリーム解析・テキスト抽出を積み、PDFページからテキストを位置情報付きで取得可能にする。

初期スコープはASCII/Latin系PDF。CJK（CIDFont, CMap, 縦書き）は後続で対応。

## デモ

```rust
let mut doc = pdfium_rs::Document::open("test.pdf")?;
let page = doc.page(0)?;
let text = page.extract_text();
let hits = page.find_text("quantum");
```

## モジュール構成

```text
src/
  fpdfapi/
    font/                      # NEW: フォントサブシステム
      mod.rs
      encoding.rs              # 定義済みエンコーディングテーブル (WinAnsi, MacRoman等)
      to_unicode.rs            # ToUnicode CMAPパーサ
      font.rs                  # PdfFont (フォント読み込み・文字コード→Unicode変換)
    page/                      # NEW: ページコンテンツ
      mod.rs
      graphics_state.rs        # テキスト状態・グラフィックス状態
      page_object.rs           # PageObject enum (TextObject等)
      content_parser.rs        # コンテンツストリームパーサ (オペレータディスパッチ)
      page.rs                  # Page構造体 (Document.page(n) の戻り値)
    parser/
      object.rs                # MODIFIED: get_f64, get_reference 追加
      document.rs              # MODIFIED: resolve, page(n) 追加
  fpdftext/                    # NEW: テキスト抽出・検索
    mod.rs
    text_page.rs               # TextPage (文字列抽出・位置情報)
    text_find.rs               # TextFind (テキスト検索)
  lib.rs                       # MODIFIED: 再エクスポート追加
```

## 設計判断

### D1: PageObject — enum（traitではない）

C++は`CPDF_PageObject`基底クラスの継承階層。Rustでは既存の`PdfObject`と同様にenumで表現。Phase 3テキスト専用スコープでは`Text`のみデータを持ち、`Path`/`Image`/`Form`はスタブ。

### D2: GraphicsState — 最小限のフラット構造体

C++のCOW (copy-on-write) 共有ポインタは不要。テキスト抽出に必要な状態のみ保持:
フォント, フォントサイズ, 文字間隔, 単語間隔, テキスト行送り, テキストライズ, 水平スケール, テキスト行列, CTM, テキスト位置。`Clone`で`q`/`Q`スタックに対応。

### D3: PdfFont — 軽量enum（継承階層ではない）

C++の深い継承ツリーをフラットに。Phase 3では`Simple`（Type1/TrueType）と`Unsupported`（CIDFont/Type3等）の2バリアント。
`Simple`はエンコーディングテーブル + ToUnicodeMap + 文字幅配列を保持。

### D4: コンテンツストリームパーサ — インメモリ`&[u8]`入力

既に`Document::decode_stream`でデコード済みのバイト列を受け取る。既存の`SyntaxParser<R: Read + Seek>`とは異なり、シーク不要のバッファパーサ。
オペランドをスタックに積み、オペレータ出現時にディスパッチ。

### D5: Page — Documentから独立した所有権

`Document::page(n)`は`&mut self`（遅延パース用）でページ辞書・リソース・コンテンツストリームを全て解決し、
所有データとして`Page`に渡す。`Page`は`Document`へのライフタイム結合を持たない。
フォントはページ単位でパース（ドキュメントレベルキャッシュは将来の最適化）。

### D6: 外部依存 — 追加なし

エンコーディングテーブルは静的データ。ToUnicode CMAPパーサは自前実装。Phase 3初期スコープでは外部crateは不要。

## 実装ステップ

### Step 1: パーサ拡張

既存型に Phase 3 で必要なヘルパーメソッドを追加。

- `PdfDictionary::get_f64(key) -> Option<f64>` — Integer/Realをf64として取得
- `PdfDictionary::get_reference(key) -> Option<ObjectId>` — Reference取得
- `Document::resolve(&mut self, obj: &PdfObject) -> Result<&PdfObject>` — Referenceを辿って実体を返す

**ファイル**: `src/fpdfapi/parser/object.rs`, `src/fpdfapi/parser/document.rs`

### Step 2: フォントエンコーディング + ToUnicode

文字コード→Unicode変換の2つの仕組みを実装。

**エンコーディングテーブル** (`encoding.rs`):

- `PredefinedEncoding` enum: `WinAnsi`, `MacRoman`, `Standard`, `PDFDoc`, `MacExpert`, `Symbol`, `ZapfDingbats`
- 各エンコーディングの `[u16; 256]` 静的テーブル
- `unicode_from_char_code(encoding, code: u8) -> Option<char>`
- `/Differences`配列によるカスタムエンコーディング対応

**ToUnicode CMapパーサ** (`to_unicode.rs`):

- `ToUnicodeMap` struct: `HashMap<u32, String>`
- `beginbfchar` / `beginbfrange` パース
- 配列形式の範囲マッピング対応
- `lookup(char_code: u32) -> Option<&str>`

**ファイル**: `src/fpdfapi/font/mod.rs`, `src/fpdfapi/font/encoding.rs`, `src/fpdfapi/font/to_unicode.rs`

### Step 3: フォント読み込み (PdfFont)

フォント辞書からフォントを構築し、文字コード→Unicode変換と文字幅取得を提供。

- `PdfFont` enum:
  - `Simple { encoding, widths: Vec<u16>, to_unicode: Option<ToUnicodeMap>, base_font: String }`
  - `Unsupported { base_font: String }`
- `PdfFont::load(font_dict, doc) -> Result<PdfFont>`
  - `/Subtype` で Type1/TrueType → `Simple`, Type0/Type3 → `Unsupported`
  - `/Encoding`: 名前 or 辞書（`/BaseEncoding` + `/Differences`）
  - `/Widths` + `/FirstChar` + `/LastChar` から幅配列構築
  - `/ToUnicode` ストリームのデコード・パース
- `unicode_from_char_code(code: u32) -> Option<String>` — ToUnicode優先、エンコーディングにフォールバック
- `char_width(code: u32) -> f64` — 1000分の1単位

**ファイル**: `src/fpdfapi/font/font.rs`

**借用の問題**: `PdfFont::load`は`&mut Document`でToUnicodeストリームを解決。ストリームデータを前もってデコードし、所有データとして`PdfFont`内に保持。

### Step 4: グラフィックス状態 + ページオブジェクト型

コンテンツストリーム解析の入出力型を定義。

**GraphicsState** (`graphics_state.rs`):

- `TextState`: font_size, char_space, word_space, text_rendering_mode
- `GraphicsState`: ctm (Matrix), text_matrix, text_pos (Point), text_line_pos, text_leading, text_rise, text_horz_scale, text_state, font (Option<PdfFont>)
- メソッド: `move_text_point(dx, dy)`, `move_to_next_line()`, `set_text_matrix(...)`, `advance_text_position(dx)`

**PageObject** (`page_object.rs`):

```rust
pub enum PageObject {
    Text(TextObject),
    Path,   // Phase 3ではスタブ
    Image,  // Phase 3ではスタブ
    Form,   // Phase 3ではスタブ
}

pub struct TextObject {
    pub char_codes: Vec<CharEntry>,
    pub font: PdfFont,
    pub font_size: f64,
    pub text_matrix: Matrix,
    pub ctm: Matrix,
}

pub struct CharEntry {
    pub code: u32,
    pub origin: Point,       // ユーザー空間での位置
    pub width: f64,          // フォント単位での幅
}
```

**ファイル**: `src/fpdfapi/page/mod.rs`, `src/fpdfapi/page/graphics_state.rs`, `src/fpdfapi/page/page_object.rs`

### Step 5: コンテンツストリームパーサ

PDFコンテンツストリームのオペランド/オペレータを解析し、`Vec<PageObject>`を生成。

**トークナイザ**:

- `&[u8]`バッファからPDFトークン（数値、名前、文字列、配列、辞書、オペレータ）を読み取り
- インライン画像 (`BI`/`ID`/`EI`) はデータをスキップ

**オペレータディスパッチ** — テキスト関連を完全実装、他はスタブ/スキップ:

| オペレータ                         | 動作                                | 実装レベル |
| ---------------------------------- | ----------------------------------- | ---------- |
| `BT`/`ET`                          | テキストオブジェクト開始/終了       | 完全       |
| `Tf`                               | フォント設定                        | 完全       |
| `Tm`                               | テキスト行列設定                    | 完全       |
| `Td`, `TD`, `T*`                   | テキスト位置移動                    | 完全       |
| `Tc`, `Tw`, `Tz`, `TL`, `Tr`, `Ts` | テキスト状態設定                    | 完全       |
| `Tj`                               | テキスト表示                        | 完全       |
| `TJ`                               | カーニング付きテキスト表示          | 完全       |
| `'`, `"`                           | 改行+テキスト表示                   | 完全       |
| `q`/`Q`                            | 状態スタック保存/復元               | 完全       |
| `cm`                               | CTM連結                             | 完全       |
| `Do`                               | XObject実行（Form XObjectのみ再帰） | 最小限     |
| パス系 (`m`,`l`,`c`,`re`等)        | スキップ                            | スタブ     |
| 色系 (`CS`,`sc`,`rg`等)            | スキップ                            | スタブ     |
| 画像系 (`BI`/`ID`/`EI`)            | データスキップ                      | スタブ     |

**フォント解決**: `/Resources/Font/<name>` からフォント辞書を取得、`PdfFont::load`で構築。パーサ内でHashMapキャッシュ。

**テキスト位置計算** (`Tj`/`TJ`処理):

1. 各文字コードについて: `width = font.char_width(code) / 1000.0 * font_size`
2. 変位 = `(width + char_space) * horz_scale` （スペース文字は `+ word_space`）
3. ユーザー空間位置 = `CTM * text_matrix * (text_pos.x, text_pos.y + text_rise)`
4. `text_pos.x += 変位` で次の文字位置に進む
5. `TJ`の数値要素: `text_pos.x -= num / 1000.0 * font_size * horz_scale`

**ファイル**: `src/fpdfapi/page/content_parser.rs`

### Step 6: Page構造体 + Document.page()

ページツリー走査とコンテンツストリーム解析のオーケストレーション。

**Page構造体** (`page.rs`):

```rust
pub struct Page {
    pub media_box: Rect,
    pub crop_box: Option<Rect>,
    pub rotation: u16,
    pub objects: Vec<PageObject>,
}
```

**Document::page(n)** の処理:

1. `/Root` → `/Pages` → ページツリーを走査してn番目のページ辞書を取得
2. `/MediaBox`, `/CropBox` を読み取り（親Pagesノードから継承可能）
3. `/Contents` を取得（ストリーム単体 or ストリーム配列）、各ストリームをデコード
4. `/Resources` 辞書を取得（親から継承可能）
5. コンテンツストリームを `ContentStreamParser` に渡して `Vec<PageObject>` を取得
6. `Page` を構築して返す

**ページツリー走査**:

- `/Pages` ノードの `/Kids` 配列を再帰的に辿る
- 各 `/Pages` の `/Count` でページ数を把握し、目的のページを効率的に見つける
- `/MediaBox` 等は子に明示されていなければ親から継承

**ファイル**: `src/fpdfapi/page/page.rs`, `src/fpdfapi/parser/document.rs`

### Step 7: テキスト抽出 (TextPage)

ページオブジェクトからテキストを読み順に抽出。

**TextPage** (`text_page.rs`):

```rust
pub struct TextPage {
    pub chars: Vec<CharInfo>,
    pub text: String,
}

pub struct CharInfo {
    pub unicode: char,
    pub origin: Point,
    pub char_box: Rect,
    pub font_size: f64,
}
```

**抽出アルゴリズム**:

1. `TextObject`を走査順に処理
2. 各文字コード → `font.unicode_from_char_code()` でUnicode変換
3. 文字位置は `CharEntry.origin`（Step 5で計算済み）を使用
4. **スペース検出**: 前の文字の右端と現在の文字の左端の間隔が `font_size * 0.25` を超えたらスペース挿入
5. **改行検出**: Y座標が `font_size * 0.5` 以上変化したら改行挿入
6. 結果を `chars` と `text` に蓄積

**API**:

- `TextPage::build(page: &Page) -> TextPage`
- `text_page.text()` → 全テキスト
- `text_page.char_count()` → 文字数
- `text_page.char_info(index)` → 位置情報

**Page統合**:

- `Page::extract_text() -> String` — TextPageを構築してテキストを返す

**ファイル**: `src/fpdftext/mod.rs`, `src/fpdftext/text_page.rs`

### Step 8: テキスト検索 + 公開API

**TextFind** (`text_find.rs`):

- `TextMatch { start: usize, end: usize }` — TextPage.textのインデックス
- `FindOptions { case_sensitive: bool, whole_word: bool }`
- `TextFind::find_all(text_page, query, options) -> Vec<TextMatch>`
- 大文字小文字無視: ASCII toLower 比較（Phase 3スコープ）
- 全語一致: 前後が非英数字であることを確認

**公開API更新** (`lib.rs`):

- `Page`, `TextMatch`, `CharInfo` の再エクスポート
- `Page::find_text(query) -> Vec<TextMatch>` ヘルパー

**ファイル**: `src/fpdftext/text_find.rs`, `src/lib.rs`, `src/fpdfapi/page/page.rs`

## テスト戦略

Phase 1-2と同様、全テストはプログラムで構築したインメモリPDFを使用。

**テストPDFの構築パターン**:

- ヘッダ + カタログ + ページツリー + ページ + フォント辞書 + コンテンツストリーム
- コンテンツストリーム例: `BT /F1 12 Tf 100 700 Td (Hello World) Tj ET`
- フォント辞書にWinAnsiエンコーディングを指定

**ステップ別テスト概要**:

| Step     | テスト数 | 主なテスト内容                                            |
| -------- | -------- | --------------------------------------------------------- |
| 1        | ~8       | get_f64, get_reference, resolve                           |
| 2        | ~25      | WinAnsiマッピング、Standard差分、ToUnicode bfchar/bfrange |
| 3        | ~15      | フォント読み込み、unicode_from_char_code、char_width      |
| 4        | ~20      | テキスト位置計算、Td/T*/Tm動作、TextObject構築・変換      |
| 5        | ~20      | トークン解析、Tj/TJ/Td/Tf等オペレータディスパッチ         |
| 6        | ~10      | ページツリー走査、Document.page()統合                     |
| 7        | ~15      | テキスト抽出、スペース/改行検出                           |
| 8        | ~10      | テキスト検索、大文字小文字無視、全語一致                  |
| **合計** | **~123** |                                                           |

## 外部依存

Phase 3初期スコープでは追加なし。

## 参照すべきC++ファイル

- `reference/pdfium/core/fpdfapi/page/cpdf_streamcontentparser.{h,cpp}` — コンテンツストリーム解析
- `reference/pdfium/core/fpdfapi/page/cpdf_page.{h,cpp}` — ページ
- `reference/pdfium/core/fpdfapi/font/cpdf_font.{h,cpp}` — フォント基底
- `reference/pdfium/core/fpdfapi/font/cpdf_simplefont.{h,cpp}` — 単純フォント
- `reference/pdfium/core/fpdfapi/font/cpdf_tounicodemap.{h,cpp}` — ToUnicode
- `reference/pdfium/core/fpdfapi/font/cpdf_fontencoding.{h,cpp}` — エンコーディング
- `reference/pdfium/core/fpdftext/cpdf_textpage.{h,cpp}` — テキスト抽出
- `reference/pdfium/core/fpdftext/cpdf_textpagefind.{h,cpp}` — テキスト検索

## 検証方法

1. `cargo test --all-features` — 全テスト通過
2. `cargo clippy --all-features --all-targets -- -D warnings` — 警告ゼロ
3. `cargo fmt --all -- --check` — フォーマット準拠
4. インメモリPDFでEnd-to-End: ページ開く → テキスト抽出 → 検索 の全パイプライン動作確認
