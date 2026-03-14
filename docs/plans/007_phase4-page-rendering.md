# Phase 4: ページレンダリング — 実装計画

## Context

Phase 1-3（基盤型・PDF解析・暗号化・テキスト抽出）が完了し、383テストがパスしている。
次のマイルストーンはPDFページをRGBAビットマップにレンダリングすること。
テキスト描画はPhase 5に回し、Phase 4ではパスオブジェクト（線・曲線・矩形の塗り/線描）と画像オブジェクトを対象とする。

### 前提作業

現在のブランチ `docs/integration-test-policy` をPR→マージしてからPhase 4に着手する。

## ステップ分割

Phase 4を7つのPR単位に分割する。各ステップはTDDサイクル（RED→GREEN→REFACTOR）に従う。

---

### Step 1: fxge基盤 — Color, Path, Bitmap

**新規ファイル**: `src/fxge/mod.rs`, `color.rs`, `path.rs`, `dib.rs`

**Color** (`color.rs`):

- `Color { r, g, b, a: u8 }` — C++の `FX_ARGB`(packed u32)を構造体化
- コンストラクタ: `gray(v)`, `rgb(r,g,b)`, `rgba(r,g,b,a)`
- CMYK→RGB近似変換: `from_cmyk(c,m,y,k)`

**Path** (`path.rs`):

- `PathPoint { point: Point, kind: PathPointKind, close: bool }`
- `PathPointKind::Move | Line | BezierControl`
- `Path { points: Vec<PathPoint> }` + `move_to`, `line_to`, `cubic_to`, `close`, `append_rect`, `transform`, `bounding_box`

**Bitmap** (`dib.rs`):

- RGBA-8888固定（4 bytes/pixel）。C++のマルチフォーマットは不要
- `Bitmap { width: u32, height: u32, data: Vec<u8> }`
- `new`, `clear`, `pixel_at`, `set_pixel`, `save_png`
- PNG書き出しに `png` crateを追加

**依存追加**: `png`

**テスト**:

- Color変換ラウンドトリップ、CMYK→RGB
- Path: rect追加→bounding_box検証、transform検証
- Bitmap: 2×2作成→set_pixel→pixel_at、save_pngでPNGマジック検証

---

### Step 2: ColorSpace — PDF色空間からColorへの変換

**新規ファイル**: `src/fpdfapi/page/color_space.rs`

```rust
pub enum ColorSpace {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
}
```

- `ColorSpace::to_color(components: &[f32]) -> Color`
- ICCBased/CalGray/CalRGBは遭遇時にDevice等価として近似（ログ警告）
- `ColorState` 構造体: fill/strokeそれぞれの色空間+成分値を保持

**テスト**:

- DeviceGray(0.0)→黒、(1.0)→白
- DeviceRGB(1,0,0)→赤
- DeviceCMYK(1,0,0,0)→シアン

---

### Step 3: GraphicsState拡張 + PageObjectデータ化

**変更ファイル**:

- `src/fpdfapi/page/graphics_state.rs` — 色状態・線スタイルを追加
- `src/fpdfapi/page/page_object.rs` — スタブ→データ構造体

**GraphicsState追加フィールド**:

- `color_state: ColorState`
- `line_width: f32` (default 1.0)
- `line_cap: LineCap` (Butt/Round/Square)
- `line_join: LineJoin` (Miter/Round/Bevel)
- `miter_limit: f32` (default 10.0)
- `dash_array: Vec<f32>`, `dash_phase: f32`

**PageObject変更**:

- `Path` → `Path(Box<PathObject>)` with path, fill_rule, stroke, colors, line style, ctm
- `Image` → `Image(Box<ImageObject>)` with decoded pixel data, dimensions, ctm, color_space

```rust
pub enum FillRule { NonZero, EvenOdd, None }

pub struct PathObject {
    pub path: crate::fxge::path::Path,
    pub fill_rule: FillRule,
    pub stroke: bool,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub line_width: f32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub miter_limit: f32,
    pub ctm: Matrix,
}

pub struct ImageObject {
    pub data: Vec<u8>,       // decoded RGBA pixels
    pub width: u32,
    pub height: u32,
    pub ctm: Matrix,
}
```

**テスト**: 構造体のフィールドアクセス、デフォルト値の検証

---

### Step 4: コンテントパーサ拡張 — パス・色・グラフィックス状態オペレータ

**変更ファイル**: `src/fpdfapi/page/content_parser.rs`

Phase 3では `_ => {}` で無視していた30+オペレータを実装する。

**パス構築** (Parser内部に `current_path: Path` を追加):

- `m` moveto, `l` lineto, `c` curveto, `v`/`y` curveto variants, `h` closepath, `re` rectangle

**パス描画** (PathObject生成):

- `S` stroke, `s` close+stroke
- `f`/`F` fill(NonZero), `f*` fill(EvenOdd)
- `B` fill+stroke(NonZero), `B*` fill+stroke(EvenOdd)
- `b` close+fill+stroke(NonZero), `b*` close+fill+stroke(EvenOdd)
- `n` discard path
- `W`/`W*` clipping（基本実装）

**色オペレータ**:

- `g`/`G` DeviceGray, `rg`/`RG` DeviceRGB, `k`/`K` DeviceCMYK
- `cs`/`CS` 色空間設定, `sc`/`SC`/`scn`/`SCN` 色値設定

**グラフィックス状態**:

- `w` line_width, `J` line_cap, `j` line_join, `M` miter_limit, `d` dash

**テスト**:

- `100 200 m 300 400 l S` → PathObject(2点, stroke=true)
- `0 0 100 50 re f` → PathObject(rect, fill_rule=NonZero)
- `1 0 0 RG 0 0 100 100 re S` → stroke_colorが赤
- `0.5 g 0 0 50 50 re f` → fill_colorが50%グレー
- テキスト+パス混在ストリーム → TextObjectとPathObject両方生成

---

### Step 5: JPEGデコーダ + 画像XObjectデコード

**新規ファイル**: `src/fxcodec/jpeg.rs`, `src/fpdfapi/page/image.rs`

**fxcodec/jpeg.rs**: `jpeg-decoder` crateのラッパー

**parser/decode.rs変更**: `DCTDecode` フィルタを `Filter` enumと `apply_filter` に追加

**fpdfapi/page/image.rs**: `decode_image_xobject(stream, doc) -> Result<ImageObject>`

- `/Width`, `/Height`, `/ColorSpace`, `/BitsPerComponent` 読み取り
- フィルタパイプラインでデコード（DCTDecode対応済み）
- DeviceGray/RGB/CMYK → RGBA変換
- 1bpc（モノクロ）ビット展開
- `/SMask` (ソフトマスク) → アルファチャンネル

**content_parser.rs変更**: `Do` オペレータで Image XObject検出時に `decode_image_xobject` 呼び出し

**依存追加**: `jpeg-decoder`

**テスト**:

- 最小JPEGバッファのデコード
- DCTDecodeフィルタ経由のストリームデコード
- 合成PDF内の2×2 RGB画像XObjectのパース

---

### Step 6: レンダリングパイプライン (tiny-skia)

**新規ファイル**: `src/fpdfapi/render/mod.rs`, `context.rs`, `status.rs`, `path_renderer.rs`, `image_renderer.rs`

**設計**: C++の深いコールスタック (RenderContext → RenderStatus → RenderDevice → AGG) を平坦化。tiny-skiaを直接呼び出す。

**RenderContext**:

- ページ→デバイス座標変換マトリクス: DPIスケーリング + Y軸反転

**render (status.rs)**:

```rust
pub fn render(page: &Page, dpi: f32) -> Result<Bitmap> {
    // 1. Pixmap作成 + 白で初期化
    // 2. PageObject順にディスパッチ
    //    - Path → path_renderer (tiny-skia fill/stroke)
    //    - Image → image_renderer (tiny-skia drawPixmap)
    //    - Text → skip (Phase 5)
    // 3. Pixmap → Bitmap変換
}
```

**path_renderer**: PathObject → tiny_skia::Path変換、fill/stroke実行
**image_renderer**: ImageObject → tiny_skia::Pixmap変換、アフィン変換で配置

**Page.render()メソッド追加**:

```rust
impl Page { pub fn render(&self, dpi: f32) -> Result<Bitmap> { ... } }
```

**依存追加**: `tiny-skia`

**テスト**:

- 空ページ→全白Bitmap
- 赤矩形1つ→中心ピクセルが赤、外側が白
- 線描→線上ピクセルがstroke色
- DPIスケーリング: 72→144でBitmapサイズ2倍

---

### Step 7: 統合テスト + CLIデモ

**変更ファイル**: `src/main.rs`, `src/lib.rs`, `tests/integration_render.rs`

**main.rs**: `pdfium-rs render <input.pdf> <output.png> [--dpi N]` コマンド追加

**lib.rs**: `Bitmap` を再エクスポート

**統合テスト**:

- 実PDFフィクスチャ（`reference/pdfium/testing/resources/` から `tests/fixtures/` にコピー）
- PDF open → render → Bitmap寸法検証 + 特定領域の色検証
- C++ `fpdf_render*_embeddertest.cpp` から基本パスレンダリングテストを移植

---

## 依存順序

```text
Step 1 (fxge)
  ↓
Step 2 (ColorSpace) ← Step 1の Color に依存
  ↓
Step 3 (GraphicsState + PageObject拡張) ← Steps 1, 2 に依存
  ↓
Step 4 (content_parser拡張) ← Steps 1-3 に依存
Step 5 (JPEG + image decode) ← Steps 1-3 に依存（Step 4と並行可能）
  ↓
Step 6 (render pipeline) ← Steps 1-5 に依存
  ↓
Step 7 (統合テスト) ← Step 6 に依存
```

## 外部依存の追加

| crate          | 用途         | Step |
| -------------- | ------------ | ---- |
| `png`          | PNG書き出し  | 1    |
| `jpeg-decoder` | JPEG伸張     | 5    |
| `tiny-skia`    | ラスタライズ | 6    |

## 設計判断

1. **Bitmap = RGBA-8888固定**: C++のマルチフォーマットDIBitmapは不要。単純化優先
2. **tiny-skia一択**: 自前ラスタライザ不要。pure Rustで外部C依存なし
3. **レンダリングパイプライン平坦化**: C++の5層抽象を1層に。バックエンドは1つしかない
4. **テキスト描画Skip**: Phase 4ではTextObjectを無視。Phase 5でフォントラスタライズと共に追加
5. **色空間はDevice系のみ**: ICCBased等は遭遇時にDevice近似。Phase 5+で精度向上

## 検証方法

- `cargo test --all-features` 全テストパス
- `cargo clippy --all-features --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- CLI: `cargo run -- render tests/fixtures/hello_world.pdf output.png` でPNG出力確認
- レンダリング結果を `pdftoppm` 出力と目視比較

## 進捗

| Step | 内容                           | ステータス |
| ---- | ------------------------------ | ---------- |
| 1    | fxge基盤 (Color, Path, Bitmap) | 完了       |
| 2    | ColorSpace                     | 完了       |
| 3    | GraphicsState + PageObject     | 完了       |
| 4    | コンテントパーサ拡張           | 完了       |
| 5    | JPEG + 画像XObject             | PR #37     |
| 6    | レンダリングパイプライン       | 未着手     |
| 7    | 統合テスト + CLI               | 未着手     |
