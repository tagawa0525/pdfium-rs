# Phase 5: テキスト描画 — 実装計画

## Context

Phase 4（パスレンダリング + 画像XObject）が完了し、446テストがパスしている。
`Page::render()` は PathObject と ImageObject を描画するが、TextObject は `status.rs:56` でスキップされている。
hello_world.pdf をレンダリングすると白画像が出力される状態。

Phase 5 では PDF ページ内のテキストをビットマップに描画する。
フォントファイル埋め込み → グリフアウトライン抽出 → tiny-skia パス描画 のパイプラインを構築する。

### スコープ外

- CID フォント（CJK）: CMap テーブルと異なるグリフマッピングが必要。Phase 6+ に先送り
- Type3 フォント: グリフがコンテンツストリームとして定義される。Phase 6+ に先送り
- fpdfdoc（ブックマーク・注釈・フォーム）: Phase 6 として別計画
- グリフキャッシュ / パフォーマンス最適化: プロファイリング後に検討

---

## 設計判断

### 1. グリフアウトライン → tiny-skia パス（Approach A）

移行計画では `ab_glyph` or `freetype-rs` を候補としていたが、**`ttf-parser` + tiny-skia パス変換** を採用する。

| 比較項目             | ttf-parser + tiny-skia   | ab_glyph     | freetype-rs  |
| -------------------- | ------------------------ | ------------ | ------------ |
| 純粋 Rust            | yes                      | yes          | no (C FFI)   |
| フォーマット         | TTF/OTF/CFF              | TTF のみ     | 全形式       |
| 描画方式             | アウトライン→パス        | ビットマップ | ビットマップ |
| Tr モード対応        | 自然（fill/stroke 分離） | 追加実装必要 | 追加実装必要 |
| tiny-skia との整合性 | 同じ AA 品質             | 別系統       | 別系統       |

`ttf-parser` の `OutlineBuilder` → `tiny_skia::PathBuilder` へ直接マッピングする。
TrueType の quadratic Bezier は `tiny_skia::PathBuilder::quad_to` でそのまま処理可能。

### 2. 非埋め込みフォントのフォールバック

hello_world.pdf は Times-Roman / Helvetica を使うが FontDescriptor がない（非埋め込み）。
Liberation Sans/Serif/Mono（SIL OFL ライセンス）を `include_bytes!` でバンドルし、
PDF 標準 14 フォントにマッピングする。合計 ~400KB。

### 3. 変換チェーン

```text
CharEntry.origin = ctm × text_matrix × (text_pos.x, text_pos.y + rise)  [user-space]
                                        ↑ content_parser が計算済み

グリフ描画変換:
  page_to_device × translate(origin) × shape_matrix × scale(fontSize/upm)

shape_matrix = (ctm × text_matrix) の回転・スケール成分（平行移動を除去）
```

TextObject は `ctm` と `text_matrix` を保持しているので shape_matrix は再計算可能。

---

## ステップ分割

### Step 1: TextObject に描画属性を追加

**変更ファイル**:

- `src/fpdfapi/page/page_object.rs` — `TextObject` に 3 フィールド追加
- `src/fpdfapi/page/content_parser.rs` — `flush_text_object()` で GraphicsState から伝播

**追加フィールド**:

```rust
pub struct TextObject {
    // ... 既存フィールド ...
    pub fill_color: Color,
    pub stroke_color: Color,
    pub text_rendering_mode: u8,  // 0=fill, 1=stroke, 2=fill+stroke, 3=invisible
}
```

`flush_text_object()` を修正:

```rust
fill_color: self.gs.color_state.fill_color(),
stroke_color: self.gs.color_state.stroke_color(),
text_rendering_mode: self.gs.text_state.text_rendering_mode,
```

**テスト**:

- `1 0 0 rg 3 Tr BT /F1 12 Tf (A) Tj ET` → fill_color=赤, text_rendering_mode=3
- 既存テスト修正（TextObject 構築箇所にフィールド追加）

---

### Step 2: FontDescriptor からフォントファイル抽出

**新規ファイル**: `src/fpdfapi/font/font_file.rs`

**変更ファイル**:

- `src/fpdfapi/font/mod.rs` — モジュール追加
- `src/fpdfapi/font/pdf_font.rs` — `PdfFont::Simple` に `font_data` フィールド追加

**設計**:

```rust
pub enum FontData {
    TrueType(Vec<u8>),   // /FontFile2
    Type1(Vec<u8>),      // /FontFile (PFB)
    OpenType(Vec<u8>),   // /FontFile3 (CFF/OpenType)
}
```

`PdfFont::load()` 拡張:

1. `/FontDescriptor` 参照を解決
2. `/FontFile2` (TrueType), `/FontFile` (Type1), `/FontFile3` (OpenType) をデコード
3. `font_data: Option<FontData>` に格納（非埋め込みは `None`）

**テスト**:

- FontDescriptor → FontFile2 ストリームを含むテスト PDF → `FontData::TrueType` が取得される
- FontDescriptor なし → `font_data: None`

---

### Step 3: グリフアウトライン抽出（ttf-parser 統合）

**依存追加**: `ttf-parser`

**新規ファイル**: `src/fpdfapi/font/glyph.rs`

**設計**:

```rust
/// フォントファイルからグリフアウトラインを Path として抽出。
pub fn glyph_outline(font_data: &[u8], glyph_id: u16) -> Option<Path>

/// 文字コード → Unicode → glyph ID マッピング。
pub fn char_to_glyph_id(font_data: &[u8], unicode: char) -> Option<u16>

/// フォントの units-per-em を取得。
pub fn units_per_em(font_data: &[u8]) -> Option<u16>
```

`OutlineBuilder` 実装で `ttf_parser` の `move_to/line_to/quad_to/curve_to/close` を
`Path` の `move_to/line_to/cubic_to/close` にマッピング。
`quad_to` は `tiny_skia::PathBuilder::quad_to` でネイティブ対応。

**テスト**:

- 既知 TrueType フォントで 'A' のアウトライン取得 → パスポイント数 > 0
- units_per_em → 期待値（例: 2048）
- スペース文字 → `None`（アウトラインなし）

---

### Step 4: テキストレンダラ — 埋め込みフォント描画

**新規ファイル**: `src/fpdfapi/render/text_renderer.rs`

**変更ファイル**:

- `src/fpdfapi/render/mod.rs` — モジュール追加
- `src/fpdfapi/render/status.rs` — TextObject → `render_text()` にディスパッチ

**設計**:

```rust
pub fn render_text(
    pixmap: &mut tiny_skia::Pixmap,
    text_obj: &TextObject,
    page_to_device: tiny_skia::Transform,
) {
    // mode 3 (invisible) → スキップ
    // 各 CharEntry について:
    //   1. char_code → unicode → glyph_id
    //   2. glyph_outline() → tiny_skia::Path
    //   3. 変換行列を構築: page_to_device × translate(origin) × shape × scale(fs/upm)
    //   4. mode に応じて fill / stroke / fill+stroke
}
```

`status.rs` 変更:

```rust
PageObject::Text(text_obj) => {
    render_text(&mut pixmap, text_obj, page_to_device);
}
```

**テスト**:

- 埋め込み TrueType フォントの合成 TextObject → 描画位置に非白ピクセル
- text_rendering_mode=3 (invisible) → 全白
- text_rendering_mode=1 (stroke) → ストローク描画

---

### Step 5: 標準 14 フォントのフォールバック

**新規ファイル**: `src/fpdfapi/font/standard_fonts.rs`

**新規ディレクトリ**: `assets/fonts/` — Liberation Sans/Serif/Mono TTF

**設計**:

```rust
/// PDF 標準フォント名 → バンドルフォントデータ
pub fn standard_font_data(base_font: &str) -> Option<&'static [u8]> {
    match base_font {
        "Helvetica" | ... => Some(include_bytes!("../../../assets/fonts/LiberationSans-Regular.ttf")),
        "Times-Roman" | ... => Some(include_bytes!("../../../assets/fonts/LiberationSerif-Regular.ttf")),
        "Courier" | ... => Some(include_bytes!("../../../assets/fonts/LiberationMono-Regular.ttf")),
        _ => None,
    }
}
```

`text_renderer.rs` を修正: `font_data` が `None` かつ標準フォント名の場合、
`standard_font_data()` からフォールバックフォントを取得してグリフ描画。

**テスト**:

- 標準フォント名マッピング（"Helvetica" → Some, "Unknown" → None）
- hello_world.pdf レンダリング → テキスト領域に非白ピクセル出現

---

### Step 6: 統合テスト

**変更ファイル**:

- `tests/integration_render.rs` — `hello_world_renders_without_panic` 更新（白→テキスト描画）
- テスト PDF フィクスチャ追加（埋め込みフォント付き）

**テストケース**:

- hello_world.pdf: テキスト領域に非白ピクセル + 背景は白
- 埋め込み TrueType フォント PDF: グリフ描画検証
- 色付きテキスト: fill_color が適用される
- テキスト + パス混在: 両方が正しく描画される
- DPI スケーリング: テキスト描画もスケール

---

## 進捗

| Step | 内容                            | 状態 |
| ---- | ------------------------------- | ---- |
| 1    | TextObject に描画属性を追加     | done |
| 2    | FontDescriptor からフォント抽出 | done |
| 3    | グリフアウトライン抽出          | done |
| 4    | テキストレンダラ                | done |
| 5    | 標準 14 フォントフォールバック  | done |
| 6    | 統合テスト                      | done |

## 依存順序

```text
Step 1 (TextObject 属性)
  ↓
Step 2 (フォントファイル抽出)
  ↓
Step 3 (グリフアウトライン + ttf-parser) ← 依存追加
  ↓
Step 4 (テキストレンダラ)
  ↓
Step 5 (標準フォントフォールバック)
  ↓
Step 6 (統合テスト)
```

## 外部依存の追加

| crate        | 用途                                             | Step |
| ------------ | ------------------------------------------------ | ---- |
| `ttf-parser` | TrueType/OpenType パース・グリフアウトライン抽出 | 3    |

バンドルフォント（Liberation Sans/Serif/Mono, SIL OFL）は `assets/fonts/` に配置。

## リスク

1. **変換チェーンの Y 方向**: TrueType は Y-up、page_to_device は Y-flip。初回テストで方向を検証し必要に応じて `-scale` 補正
2. **Type1 PFB フォント**: `ttf-parser` は PFB 非対応。CFF 形式の `/FontFile3` は対応。生の PFB は稀なためフォールバックで対応
3. **バイナリサイズ**: Liberation フォント ~400KB。必要なら cargo feature で opt-in に
4. **グリフ ID 解決**: char_code → unicode → glyph_id の変換チェーンが壊れるカスタムエンコーディングあり。unicode → glyph_id 失敗時は .notdef にフォールバック

## 検証方法

```bash
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -- render tests/fixtures/hello_world.pdf /tmp/hello.png
# /tmp/hello.png にテキストが描画されていること
```
